"""Walk-forward accuracy verification against the historical dataset.

For every pool and every day after the warm-up window, the model forecasts
the next day's APY using only data up to that day, then the forecast is
compared with the realized value. The persistence baseline ("tomorrow equals
today") is reported alongside so the model's edge is measurable.

Run from agent/:  python -m ml.backtest
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from ml.data import DEFAULT_DATASET, PoolSeries, load_rate_history
from ml.forecast import TRAILING_WINDOW, forecast_next_24h

WARMUP = TRAILING_WINDOW + 1


@dataclass(frozen=True)
class BacktestReport:
    pool_id: str
    protocol: str
    asset: str
    n_forecasts: int
    mae: float
    rmse: float
    baseline_mae: float

    @property
    def improvement_vs_baseline(self) -> float:
        if self.baseline_mae == 0:
            return 0.0
        return 1.0 - self.mae / self.baseline_mae

    def to_dict(self) -> dict[str, Any]:
        return {
            "pool_id": self.pool_id,
            "protocol": self.protocol,
            "asset": self.asset,
            "n_forecasts": self.n_forecasts,
            "mae": round(self.mae, 4),
            "rmse": round(self.rmse, 4),
            "baseline_mae": round(self.baseline_mae, 4),
            "improvement_vs_baseline": round(self.improvement_vs_baseline, 4),
        }


def backtest_series(series: PoolSeries, warmup: int = WARMUP) -> BacktestReport:
    apys = series.apys
    if len(apys) <= warmup + 1:
        raise ValueError(f"Series {series.pool_id} too short for backtest (need > {warmup + 1} points)")

    abs_errors: list[float] = []
    sq_errors: list[float] = []
    baseline_abs_errors: list[float] = []

    for t in range(warmup, len(apys) - 1):
        history = apys[: t + 1]
        actual = apys[t + 1]
        predicted, _ = forecast_next_24h(history)
        abs_errors.append(abs(predicted - actual))
        sq_errors.append((predicted - actual) ** 2)
        baseline_abs_errors.append(abs(history[-1] - actual))

    n = len(abs_errors)
    return BacktestReport(
        pool_id=series.pool_id,
        protocol=series.protocol,
        asset=series.asset,
        n_forecasts=n,
        mae=sum(abs_errors) / n,
        rmse=math.sqrt(sum(sq_errors) / n),
        baseline_mae=sum(baseline_abs_errors) / n,
    )


def run_backtest(dataset: Path | str = DEFAULT_DATASET) -> list[BacktestReport]:
    history = load_rate_history(dataset)
    return [backtest_series(series) for series in history.values()]


def main() -> None:
    reports = run_backtest()
    print(json.dumps({"backtest": [r.to_dict() for r in reports]}, indent=2))


if __name__ == "__main__":
    main()
