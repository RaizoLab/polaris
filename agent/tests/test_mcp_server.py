import asyncio
import math

import pytest

import mcp_tools


def test_vault_yield_answers_current_yield_question(monkeypatch) -> None:
    """LLM asks "What is the current yield in my vault?" -> blended APY answer."""
    monkeypatch.delenv("VAULT_ALLOCATIONS", raising=False)
    result = mcp_tools.vault_yield(use_live=False)
    assert isinstance(result["vault_apy"], float)
    assert result["vault_apy"] > 0
    assert "APY" in result["summary"]
    weights = [p["weight"] for p in result["positions"]]
    assert math.isclose(sum(weights), 1.0, abs_tol=1e-6)


def test_vault_yield_honors_custom_allocations(monkeypatch) -> None:
    monkeypatch.setenv("VAULT_ALLOCATIONS", "blend-fixed-xlm:1.0")
    result = mcp_tools.vault_yield(use_live=False)
    assert len(result["positions"]) == 1
    assert result["positions"][0]["pool_id"] == "blend-fixed-xlm"
    assert result["positions"][0]["weight"] == 1.0


def test_market_scan_ranks_opportunities(monkeypatch) -> None:
    monkeypatch.delenv("VAULT_ALLOCATIONS", raising=False)
    result = mcp_tools.market_scan(use_live=False)
    opportunities = result["opportunities"]
    assert len(opportunities) >= 3
    expected = [o["expected_apy_24h"] for o in opportunities]
    assert expected == sorted(expected, reverse=True)
    assert result["best"] == opportunities[0]
    for opportunity in opportunities:
        assert {"protocol", "pool_id", "current_apy", "expected_apy_24h", "confidence_90"} <= set(opportunity)


def test_rebalance_check_compares_current_vs_optimal(monkeypatch) -> None:
    monkeypatch.delenv("VAULT_ALLOCATIONS", raising=False)
    result = mcp_tools.rebalance_check(use_live=False)
    assert isinstance(result["should_rebalance"], bool)
    assert result["expected_gain_bps"] == pytest.approx(
        (result["optimal_expected_apy_24h"] - result["current_expected_apy_24h"]) * 100.0,
        abs=0.5,
    )
    optimal_weights = [entry["weight"] for entry in result["optimal_allocation"]]
    assert math.isclose(sum(optimal_weights), 1.0, abs_tol=1e-4)


def test_forecast_yields_outputs_optimal_allocation_array(monkeypatch) -> None:
    result = mcp_tools.forecast_yields()
    assert result["horizon_hours"] == 24
    assert len(result["forecasts"]) >= 3
    allocation = result["optimal_allocation"]
    assert isinstance(allocation, list) and allocation
    assert math.isclose(sum(a["weight"] for a in allocation), 1.0, abs_tol=1e-4)


def test_mcp_server_exposes_required_tools() -> None:
    """Acceptance: server exposes rebalance_check and market_scan tools."""
    pytest.importorskip("mcp")
    import mcp_server

    tools = asyncio.run(mcp_server.mcp.list_tools())
    names = {tool.name for tool in tools}
    assert {"vault_yield", "market_scan", "rebalance_check", "forecast_yields"} <= names
