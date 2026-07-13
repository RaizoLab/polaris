"""Soroswap AMM / aggregator yield provider."""

from __future__ import annotations

import os
from typing import Any, Callable

from yields.http_client import ProtocolHttpError, http_get_json
from yields.schema import YieldEntry


Fetcher = Callable[..., Any]


def fetch_soroswap_yields(
    *,
    base_url: str | None = None,
    api_key: str | None = None,
    timeout: float = 10.0,
    network: str = "MAINNET",
    fetch: Fetcher = http_get_json,
) -> list[YieldEntry]:
    url = (base_url or os.getenv("SOROSWAP_API_URL", "https://api.soroswap.finance")).rstrip("/")
    key = api_key if api_key is not None else os.getenv("SOROSWAP_API_KEY", "")
    headers: dict[str, str] = {}
    if key:
        headers["Authorization"] = f"Bearer {key}"
        headers["x-api-key"] = key

    endpoint = f"{url}/pools?network={network.upper()}&protocol=SOROSWAP"
    try:
        payload = fetch(endpoint, protocol="soroswap", timeout=timeout, headers=headers or None)
    except ProtocolHttpError as exc:
        return [
            YieldEntry(
                protocol="soroswap",
                pool_id="*",
                asset="*",
                apy=None,
                apr=None,
                tvl_usd=None,
                source=endpoint,
                status="error",
                error=exc.message,
            )
        ]

    pools = payload if isinstance(payload, list) else payload.get("pools") if isinstance(payload, dict) else []
    if not isinstance(pools, list) or not pools:
        return [
            YieldEntry(
                protocol="soroswap",
                pool_id="*",
                asset="*",
                apy=None,
                apr=None,
                tvl_usd=None,
                source=endpoint,
                status="unavailable",
                error="No Soroswap pools returned (API key may be required)",
            )
        ]

    entries: list[YieldEntry] = []
    for pool in pools:
        if not isinstance(pool, dict):
            continue
        entries.append(_pool_to_entry(pool, source=endpoint))
    return entries or [
        YieldEntry(
            protocol="soroswap",
            pool_id="*",
            asset="*",
            apy=None,
            apr=None,
            tvl_usd=None,
            source=endpoint,
            status="unavailable",
            error="Soroswap pool payload could not be normalized",
        )
    ]


def _pool_to_entry(pool: dict[str, Any], *, source: str) -> YieldEntry:
    token_a = pool.get("tokenA") or pool.get("token_a") or {}
    token_b = pool.get("tokenB") or pool.get("token_b") or {}
    a_sym = _token_symbol(token_a, fallback="A")
    b_sym = _token_symbol(token_b, fallback="B")
    asset = f"{a_sym}/{b_sym}"
    pool_id = str(pool.get("address") or pool.get("id") or asset)
    tvl = _as_float(pool.get("tvlUsd") or pool.get("tvl") or pool.get("liquidityUsd"))
    volume = _as_float(pool.get("volume24h") or pool.get("volumeUsd24h") or pool.get("volume_24h"))
    fee_bps = _as_float(pool.get("feeBps") or pool.get("fee_bps") or 30) or 30.0
    apy = _as_float(pool.get("apy") or pool.get("feeApy"))
    if apy is None and tvl and tvl > 0 and volume is not None:
        daily_fees = volume * (fee_bps / 10_000.0)
        apy = (daily_fees / tvl) * 365.0 * 100.0

    return YieldEntry(
        protocol="soroswap",
        pool_id=pool_id,
        asset=asset,
        apy=apy,
        apr=apy,
        tvl_usd=tvl,
        source=source,
        status="ok" if apy is not None else "unavailable",
        error=None if apy is not None else "Insufficient fields to estimate APY",
        metadata={"fee_bps": fee_bps, "volume_24h": volume},
    )


def _token_symbol(token: Any, *, fallback: str) -> str:
    if isinstance(token, dict):
        return str(token.get("code") or token.get("symbol") or fallback)
    return str(token) if token else fallback


def _as_float(value: Any) -> float | None:
    if value is None or value == "":
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None
