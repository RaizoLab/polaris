import math

from ml.backtest import run_backtest
from ml.data import DEFAULT_DATASET, load_rate_history
from ml.forecast import forecast_all, forecast_next_24h, optimal_allocation, realized_volatility


def test_dataset_loads_all_pools() -> None:
    history = load_rate_history(DEFAULT_DATASET)
    assert len(history) >= 3
    for series in history.values():
        assert len(series.points) >= 30
        dates = [p.date for p in series.points]
        assert dates == sorted(dates)


def test_realized_volatility_zero_for_constant_series() -> None:
    assert realized_volatility([5.0] * 30) == 0.0


def test_forecast_tracks_stable_series() -> None:
    expected, vol = forecast_next_24h([4.0] * 30)
    assert math.isclose(expected, 4.0, rel_tol=1e-9)
    assert vol == 0.0


def test_forecast_regresses_noisy_series_toward_mean() -> None:
    # Series oscillating around 10 with a large final spike: forecast should
    # land between the spike and the trailing mean.
    series = [10.0, 9.0, 11.0, 10.0, 9.0, 11.0, 10.0, 9.0, 11.0, 10.0, 9.0, 11.0, 10.0, 16.0]
    expected, vol = forecast_next_24h(series)
    trailing_mean = sum(series) / len(series)
    assert trailing_mean < expected < 16.0
    assert vol > 0


def test_optimal_allocation_sums_to_one_and_respects_cap() -> None:
    history = load_rate_history(DEFAULT_DATASET)
    allocation = optimal_allocation(forecast_all(history), max_weight=0.6)
    assert allocation, "Optimal Allocation array must not be empty"
    total = sum(entry.weight for entry in allocation)
    assert math.isclose(total, 1.0, abs_tol=1e-6)
    assert all(0.0 <= entry.weight <= 0.6 + 1e-9 for entry in allocation)
    # Higher risk-adjusted yield should not receive a lower weight.
    weights = [entry.weight for entry in allocation]
    assert weights == sorted(weights, reverse=True)


def test_backtest_accuracy_against_historical_dataset() -> None:
    """Acceptance: accuracy verified against historical Stellar pool rates."""
    reports = run_backtest(DEFAULT_DATASET)
    assert len(reports) >= 3
    for report in reports:
        assert report.n_forecasts >= 50
        # Model must not lose to the persistence baseline ("tomorrow == today").
        assert report.mae <= report.baseline_mae * 1.02, (
            f"{report.pool_id}: model MAE {report.mae:.4f} worse than baseline {report.baseline_mae:.4f}"
        )
        # Absolute error stays small relative to the pool's rate level.
        mean_apy = sum(p.apy for p in load_rate_history(DEFAULT_DATASET)[report.pool_id].points) / len(
            load_rate_history(DEFAULT_DATASET)[report.pool_id].points
        )
        assert report.mae < max(0.5, 0.15 * mean_apy)
