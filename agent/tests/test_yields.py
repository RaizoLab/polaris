from yields.fetch_yields import fetch_all_yields
from yields.http_client import ProtocolHttpError
from yields.providers import blend, phoenix, soroswap
from yields.schema import YieldEntry


def test_blend_provider_normalizes_pools() -> None:
    def fake_fetch(url, protocol, timeout, headers=None):
        assert protocol == "blend"
        return {
            "pools": [
                {
                    "id": "pool-1",
                    "asset": "USDC",
                    "supplyApy": 4.2,
                    "tvlUsd": 1_000_000,
                }
            ]
        }

    entries = blend.fetch_blend_yields(fetch=fake_fetch, timeout=1)
    assert len(entries) == 1
    assert entries[0].status == "ok"
    assert entries[0].asset == "USDC"
    assert entries[0].apy == 4.2


def test_blend_provider_handles_downtime() -> None:
    def boom(url, protocol, timeout, headers=None):
        raise ProtocolHttpError(protocol, "down", status_code=503)

    entries = blend.fetch_blend_yields(fetch=boom, timeout=1)
    assert len(entries) == 1
    assert entries[0].status == "error"
    assert "down" in (entries[0].error or "")


def test_phoenix_estimates_fee_apy() -> None:
    def fake_fetch(url, protocol, timeout, headers=None):
        assert "PHOENIX" in url
        return [
            {
                "address": "CPOOL",
                "tokenA": {"code": "USDC"},
                "tokenB": {"code": "USDT"},
                "tvlUsd": 100_000,
                "volume24h": 10_000,
                "feeBps": 30,
            }
        ]

    entries = phoenix.fetch_phoenix_yields(fetch=fake_fetch, timeout=1)
    assert entries[0].status == "ok"
    assert entries[0].asset == "USDC/USDT"
    assert entries[0].apy is not None
    assert entries[0].apy > 0


def test_soroswap_provider_handles_forbidden() -> None:
    def forbidden(url, protocol, timeout, headers=None):
        raise ProtocolHttpError(protocol, "HTTP 403 Forbidden", status_code=403)

    entries = soroswap.fetch_soroswap_yields(fetch=forbidden, timeout=1)
    assert entries[0].status == "error"
    assert entries[0].error is not None


def test_fetch_all_yields_aggregates_with_errors(monkeypatch) -> None:
    import yields.fetch_yields as fy

    monkeypatch.setattr(
        fy,
        "fetch_blend_yields",
        lambda timeout=10: [
            YieldEntry(
                protocol="blend",
                pool_id="p1",
                asset="USDC",
                apy=3.1,
                apr=3.0,
                tvl_usd=5000,
                source="mock",
                status="ok",
            )
        ],
    )
    monkeypatch.setattr(
        fy,
        "fetch_phoenix_yields",
        lambda timeout=10, network="MAINNET": [
            YieldEntry(
                protocol="phoenix",
                pool_id="*",
                asset="*",
                apy=None,
                apr=None,
                tvl_usd=None,
                source="mock",
                status="error",
                error="timeout",
            )
        ],
    )
    monkeypatch.setattr(
        fy,
        "fetch_soroswap_yields",
        lambda timeout=10, network="MAINNET": [
            YieldEntry(
                protocol="soroswap",
                pool_id="s1",
                asset="XLM/USDC",
                apy=1.5,
                apr=1.5,
                tvl_usd=20000,
                source="mock",
                status="ok",
            )
        ],
    )

    payload = fetch_all_yields(network="mainnet", timeout=1)
    assert payload["network"] == "mainnet"
    assert "fetched_at" in payload
    assert payload["summary"]["ok"] == 2
    assert "phoenix" in payload["summary"]["failed_protocols"]
    assert isinstance(payload["yields"], list)
