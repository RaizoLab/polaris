"""Volatility-aware 24h yield forecasting and allocation optimization.

The model forecasts the next-24h APY for each pool as a blend of the latest
observation and the recent trailing mean. The blend weight adapts to realized
volatility: stable series trust the latest print, noisy series lean on the
mean (mean reversion dominates when daily moves are mostly noise).

``optimal_allocation`` converts per-pool forecasts into portfolio weights by
scoring each pool on volatility-penalized expected yield, then normalizing
with a max-weight cap for diversification.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Any, Sequence

from ml.data import PoolSeries

TRAILING_WINDOW = 14
Z_90 = 1.645  # 90% confidence band


@dataclass(frozen=True)
class Forecast:
    protocol: str
    pool_id: str
    asset: str
    current_apy: float
    expected_apy_24h: float
    volatility: float  # daily std-dev of APY changes (percentage points)
    low_90: float
    high_90: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "protocol": self.protocol,
            "pool_id": self.pool_id,
            "asset": self.asset,
            "current_apy": round(self.current_apy, 4),
            "expected_apy_24h": round(self.expected_apy_24h, 4),
            "volatility_daily": round(self.volatility, 4),
            "confidence_90": [round(self.low_90, 4), round(self.high_90, 4)],
        }


@dataclass(frozen=True)
class AllocationEntry:
    protocol: str
    pool_id: str
    asset: str
    weight: float
    expected_apy_24h: float
    volatility: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "protocol": self.protocol,
            "pool_id": self.pool_id,
            "asset": self.asset,
            "weight": round(self.weight, 4),
            "expected_apy_24h": round(self.expected_apy_24h, 4),
            "volatility_daily": round(self.volatility, 4),
        }


def realized_volatility(apys: Sequence[float], window: int = TRAILING_WINDOW) -> float:
    """Std-dev of day-over-day APY changes over the trailing window."""
    recent = list(apys)[-(window + 1):]
    diffs = [b - a for a, b in zip(recent, recent[1:])]
    if len(diffs) < 2:
        return 0.0
    mean = sum(diffs) / len(diffs)
    var = sum((d - mean) ** 2 for d in diffs) / (len(diffs) - 1)
    return math.sqrt(var)


def forecast_next_24h(apys: Sequence[float], window: int = TRAILING_WINDOW) -> tuple[float, float]:
    """Return (expected_apy, volatility) for the next 24 hours.

    expected = alpha * latest + (1 - alpha) * trailing_mean, where alpha
    shrinks as volatility grows relative to the trailing mean level.
    """
    if not apys:
        raise ValueError("Cannot forecast from an empty series")
    latest = apys[-1]
    recent = list(apys)[-window:]
    trailing_mean = sum(recent) / len(recent)
    vol = realized_volatility(apys, window)

    scale = max(trailing_mean, 0.1)
    noise_ratio = min(vol / scale, 1.0)
    alpha = 1.0 - 0.7 * noise_ratio  # stable pool: ~1.0, noisy pool: ~0.3

    expected = alpha * latest + (1.0 - alpha) * trailing_mean
    return max(expected, 0.0), vol


def forecast_pool(series: PoolSeries, window: int = TRAILING_WINDOW) -> Forecast:
    expected, vol = forecast_next_24h(series.apys, window)
    return Forecast(
        protocol=series.protocol,
        pool_id=series.pool_id,
        asset=series.asset,
        current_apy=series.latest.apy,
        expected_apy_24h=expected,
        volatility=vol,
        low_90=max(expected - Z_90 * vol, 0.0),
        high_90=expected + Z_90 * vol,
    )


def forecast_all(series_by_pool: dict[str, PoolSeries], window: int = TRAILING_WINDOW) -> list[Forecast]:
    return [forecast_pool(s, window) for s in series_by_pool.values()]


def optimal_allocation(
    forecasts: Sequence[Forecast],
    *,
    risk_aversion: float = 0.5,
    max_weight: float = 0.6,
) -> list[AllocationEntry]:
    """Convert forecasts into portfolio weights (sums to 1.0).

    Score = expected APY penalized by volatility; weights are proportional to
    score, capped at ``max_weight`` per pool with the excess redistributed.
    """
    if not forecasts:
        return []

    scores = {
        f.pool_id: max(f.expected_apy_24h / (1.0 + risk_aversion * f.volatility), 0.0)
        for f in forecasts
    }
    total = sum(scores.values())
    if total <= 0:
        weights = {f.pool_id: 1.0 / len(forecasts) for f in forecasts}
    else:
        weights = {pid: s / total for pid, s in scores.items()}
        weights = _cap_weights(weights, max_weight)

    return [
        AllocationEntry(
            protocol=f.protocol,
            pool_id=f.pool_id,
            asset=f.asset,
            weight=weights[f.pool_id],
            expected_apy_24h=f.expected_apy_24h,
            volatility=f.volatility,
        )
        for f in sorted(forecasts, key=lambda f: weights[f.pool_id], reverse=True)
    ]


def _cap_weights(weights: dict[str, float], max_weight: float) -> dict[str, float]:
    """Cap weights at max_weight, redistributing excess to uncapped pools."""
    if max_weight * len(weights) < 1.0:
        # Cap infeasible for this pool count; fall back to equal weights.
        return {pid: 1.0 / len(weights) for pid in weights}

    capped = dict(weights)
    for _ in range(len(weights)):
        over = {pid: w for pid, w in capped.items() if w > max_weight + 1e-12}
        if not over:
            break
        excess = sum(w - max_weight for w in over.values())
        for pid in over:
            capped[pid] = max_weight
        under = {pid: w for pid, w in capped.items() if w < max_weight - 1e-12}
        under_total = sum(under.values())
        if under_total <= 0:
            break
        for pid, w in under.items():
            capped[pid] = w + excess * (w / under_total)
    return capped
