"""Generate the bundled historical Stellar pool-rates dataset.

Produces a deterministic (seeded) daily APY series for representative Stellar
DeFi pools, calibrated to observed rate ranges on Blend / Soroswap / Phoenix.
Each pool follows a mean-reverting random walk with occasional yield spikes,
which mirrors how lending rates and LP fee APYs behave on-chain.

Run from agent/:  python -m ml.generate_dataset
"""

from __future__ import annotations

import csv
import random
from dataclasses import dataclass
from datetime import date, timedelta
from pathlib import Path

DATASET_PATH = Path(__file__).resolve().parent.parent / "data" / "historical_pool_rates.csv"

DAYS = 120
SEED = 20260713


@dataclass(frozen=True)
class PoolSpec:
    protocol: str
    pool_id: str
    asset: str
    mean_apy: float
    volatility: float  # daily std-dev of APY changes, in percentage points
    spike_prob: float  # chance of a temporary yield spike on a given day
    spike_size: float


POOLS = [
    PoolSpec("blend", "blend-fixed-usdc", "USDC", 4.8, 0.15, 0.03, 2.0),
    PoolSpec("blend", "blend-fixed-xlm", "XLM", 2.4, 0.10, 0.02, 1.0),
    PoolSpec("soroswap", "soroswap-xlm-usdc", "XLM/USDC", 11.0, 0.90, 0.06, 6.0),
    PoolSpec("phoenix", "phoenix-usdc-usdt", "USDC/USDT", 3.6, 0.25, 0.03, 1.5),
]


def generate_series(spec: PoolSpec, days: int, rng: random.Random) -> list[float]:
    apy = spec.mean_apy
    series: list[float] = []
    spike_left = 0
    for _ in range(days):
        # Mean reversion pulls the rate back toward the pool's long-run level.
        drift = 0.08 * (spec.mean_apy - apy)
        shock = rng.gauss(0.0, spec.volatility)
        if spike_left > 0:
            spike_left -= 1
        elif rng.random() < spec.spike_prob:
            apy += spec.spike_size * rng.uniform(0.5, 1.0)
            spike_left = rng.randint(1, 3)
        apy = max(0.05, apy + drift + shock)
        series.append(round(apy, 4))
    return series


def main() -> None:
    rng = random.Random(SEED)
    start = date(2026, 3, 1)
    DATASET_PATH.parent.mkdir(parents=True, exist_ok=True)
    with DATASET_PATH.open("w", newline="") as fh:
        writer = csv.writer(fh)
        writer.writerow(["date", "protocol", "pool_id", "asset", "apy"])
        for spec in POOLS:
            series = generate_series(spec, DAYS, rng)
            for i, apy in enumerate(series):
                writer.writerow(
                    [(start + timedelta(days=i)).isoformat(), spec.protocol, spec.pool_id, spec.asset, apy]
                )
    print(f"Wrote {DAYS * len(POOLS)} rows to {DATASET_PATH}")


if __name__ == "__main__":
    main()
