"""Tool implementations behind the Polaris MCP server.

Kept free of MCP SDK imports so the logic is directly unit-testable; the
server in mcp_server.py registers these as MCP tools.

Rates come from the live yield pipeline (Blend / Phoenix / Soroswap APIs)
when reachable and fall back to the bundled historical dataset otherwise, so
every tool always returns a usable answer.
"""

from __future__ import annotations

import os
from typing import Any

from ml.data import load_rate_history
from ml.forecast import forecast_all, optimal_allocation
from yields.fetch_yields import fetch_all_yields
from yields.schema import utc_now_iso

DEFAULT_ALLOCATIONS = "blend-fixed-usdc:0.6,soroswap-xlm-usdc:0.4"
REBALANCE_THRESHOLD_BPS = 25.0


def _vault_allocations() -> dict[str, float]:
    """Parse VAULT_ALLOCATIONS ("pool_id:weight,...") and normalize to 1.0."""
    raw = os.getenv("VAULT_ALLOCATIONS", "").strip() or DEFAULT_ALLOCATIONS
    allocations: dict[str, float] = {}
    for part in raw.split(","):
        part = part.strip()
        if not part:
            continue
        pool_id, _, weight = part.partition(":")
        try:
            allocations[pool_id.strip()] = float(weight)
        except ValueError:
            continue
    total = sum(allocations.values())
    if total <= 0:
        raise ValueError(f"VAULT_ALLOCATIONS has no valid 'pool_id:weight' entries: {raw!r}")
    return {pid: w / total for pid, w in allocations.items()}


def _live_rate_index(network: str) -> dict[str, float]:
    """Map 'protocol:asset' -> APY from the live pipeline; empty on failure."""
    try:
        snapshot = fetch_all_yields(network=network)
    except Exception:
        return {}
    index: dict[str, float] = {}
    for entry in snapshot.get("yields", []):
        if entry.get("status") == "ok" and entry.get("apy") is not None:
            index[f"{entry['protocol']}:{entry['asset']}"] = float(entry["apy"])
    return index


def _resolve_pool_apy(series: Any, live_index: dict[str, float]) -> tuple[float, str]:
    live = live_index.get(f"{series.protocol}:{series.asset}")
    if live is not None:
        return live, "live"
    return series.latest.apy, "historical-dataset"


def vault_yield(network: str = "mainnet", use_live: bool = True) -> dict[str, Any]:
    """Answer "What is the current yield in my vault?".

    Blends per-pool APYs (live when available) with the vault's allocation
    weights into a single portfolio APY.
    """
    history = load_rate_history()
    allocations = _vault_allocations()
    live_index = _live_rate_index(network) if use_live else {}

    positions: list[dict[str, Any]] = []
    blended_apy = 0.0
    for pool_id, weight in allocations.items():
        series = history.get(pool_id)
        if series is None:
            positions.append(
                {"pool_id": pool_id, "weight": round(weight, 4), "apy": None, "error": "unknown pool"}
            )
            continue
        apy, source = _resolve_pool_apy(series, live_index)
        blended_apy += weight * apy
        positions.append(
            {
                "pool_id": pool_id,
                "protocol": series.protocol,
                "asset": series.asset,
                "weight": round(weight, 4),
                "apy": round(apy, 4),
                "rate_source": source,
            }
        )

    return {
        "as_of": utc_now_iso(),
        "network": network,
        "vault_apy": round(blended_apy, 4),
        "positions": positions,
        "summary": f"Your vault currently earns approximately {blended_apy:.2f}% APY across {len(positions)} positions.",
    }


def market_scan(network: str = "mainnet", use_live: bool = True) -> dict[str, Any]:
    """Scan Stellar DeFi yields and rank opportunities with 24h forecasts."""
    history = load_rate_history()
    live_index = _live_rate_index(network) if use_live else {}
    forecasts = forecast_all(history)

    opportunities: list[dict[str, Any]] = []
    for forecast in forecasts:
        series = history[forecast.pool_id]
        apy, source = _resolve_pool_apy(series, live_index)
        record = forecast.to_dict()
        record["current_apy"] = round(apy, 4)
        record["rate_source"] = source
        opportunities.append(record)
    opportunities.sort(key=lambda o: o["expected_apy_24h"], reverse=True)

    live_extras = [
        {"market": key, "apy": round(apy, 4)}
        for key, apy in sorted(live_index.items())
        if not any(o["protocol"] + ":" + o["asset"] == key for o in opportunities)
    ]

    return {
        "as_of": utc_now_iso(),
        "network": network,
        "live_data_available": bool(live_index),
        "opportunities": opportunities,
        "other_live_markets": live_extras[:20],
        "best": opportunities[0] if opportunities else None,
    }


def rebalance_check(network: str = "mainnet", use_live: bool = True) -> dict[str, Any]:
    """Compare current vault allocation to the model's optimal allocation."""
    history = load_rate_history()
    allocations = _vault_allocations()
    forecasts = forecast_all(history)
    optimal = optimal_allocation(forecasts)
    expected = {f.pool_id: f.expected_apy_24h for f in forecasts}

    current_apy = sum(w * expected.get(pid, 0.0) for pid, w in allocations.items())
    optimal_apy = sum(entry.weight * entry.expected_apy_24h for entry in optimal)
    gain_bps = (optimal_apy - current_apy) * 100.0
    should_rebalance = gain_bps > REBALANCE_THRESHOLD_BPS

    return {
        "as_of": utc_now_iso(),
        "network": network,
        "current_allocation": [
            {"pool_id": pid, "weight": round(w, 4), "expected_apy_24h": round(expected.get(pid, 0.0), 4)}
            for pid, w in allocations.items()
        ],
        "optimal_allocation": [entry.to_dict() for entry in optimal],
        "current_expected_apy_24h": round(current_apy, 4),
        "optimal_expected_apy_24h": round(optimal_apy, 4),
        "expected_gain_bps": round(gain_bps, 2),
        "threshold_bps": REBALANCE_THRESHOLD_BPS,
        "should_rebalance": should_rebalance,
        "summary": (
            f"Rebalancing would lift expected 24h APY by {gain_bps:.1f} bps"
            f" ({current_apy:.2f}% -> {optimal_apy:.2f}%)."
            if should_rebalance
            else f"Hold: expected gain {gain_bps:.1f} bps is under the {REBALANCE_THRESHOLD_BPS:.0f} bps threshold."
        ),
    }


def forecast_yields() -> dict[str, Any]:
    """24h yield forecast and optimal allocation from the ML model."""
    history = load_rate_history()
    forecasts = forecast_all(history)
    optimal = optimal_allocation(forecasts)
    return {
        "as_of": utc_now_iso(),
        "horizon_hours": 24,
        "forecasts": [f.to_dict() for f in forecasts],
        "optimal_allocation": [entry.to_dict() for entry in optimal],
    }
