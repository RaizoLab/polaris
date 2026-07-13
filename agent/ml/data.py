"""Load historical Stellar pool-rate datasets."""

from __future__ import annotations

import csv
from dataclasses import dataclass
from pathlib import Path

DEFAULT_DATASET = Path(__file__).resolve().parent.parent / "data" / "historical_pool_rates.csv"


@dataclass(frozen=True)
class RatePoint:
    date: str
    protocol: str
    pool_id: str
    asset: str
    apy: float


@dataclass(frozen=True)
class PoolSeries:
    protocol: str
    pool_id: str
    asset: str
    points: tuple[RatePoint, ...]

    @property
    def apys(self) -> list[float]:
        return [p.apy for p in self.points]

    @property
    def latest(self) -> RatePoint:
        return self.points[-1]


def load_rate_history(path: Path | str = DEFAULT_DATASET) -> dict[str, PoolSeries]:
    """Return per-pool APY series keyed by pool_id, ordered by date."""
    rows: dict[str, list[RatePoint]] = {}
    meta: dict[str, tuple[str, str]] = {}
    with Path(path).open(newline="") as fh:
        for row in csv.DictReader(fh):
            point = RatePoint(
                date=row["date"],
                protocol=row["protocol"],
                pool_id=row["pool_id"],
                asset=row["asset"],
                apy=float(row["apy"]),
            )
            rows.setdefault(point.pool_id, []).append(point)
            meta[point.pool_id] = (point.protocol, point.asset)

    series: dict[str, PoolSeries] = {}
    for pool_id, points in rows.items():
        points.sort(key=lambda p: p.date)
        protocol, asset = meta[pool_id]
        series[pool_id] = PoolSeries(
            protocol=protocol,
            pool_id=pool_id,
            asset=asset,
            points=tuple(points),
        )
    return series
