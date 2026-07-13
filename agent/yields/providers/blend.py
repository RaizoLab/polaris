"""Blend lending yield provider."""

from __future__ import annotations

import os
from typing import Any, Callable

from yields.http_client import ProtocolHttpError, http_get_json
from yields.schema import YieldEntry


Fetcher = Callable[..., Any]


def fetch_blend_yields(
    *,
    base_url: str | None = None,
    api_key: str | None = None,
    timeout: float = 10.0,
    fetch: Fetcher = http_get_json,
) -> list[YieldEntry]:
    """Query Blend (or compatible) rates API and normalize to YieldEntry list.

    Expected successful payload shapes (first match wins):
      {"pools": [{"id", "asset"|"symbol", "supplyApy"|"apy", "tvlUsd"|"tvl", ...}, ...]}
      [{"id", "asset", "apy", ...}, ...]
    """
    url = (base_url or os.getenv("BLEND_API_URL", "https://api.blend.capital")).rstrip("/")
    key = api_key if api_key is not None else os.getenv("BLEND_API_KEY", "")
    headers: dict[str, str] = {}
    if key:
        headers["Authorization"] = f"Bearer {key}"
        headers["x-api-key"] = key

    try:
        payload = fetch(
            f"{url}/v1/pools",
            protocol="blend",
            timeout=timeout,
            headers=headers or None,
        )
    except ProtocolHttpError:
        # Some deployments expose /pools without version prefix.
        try:
            payload = fetch(
                f"{url}/pools",
                protocol="blend",
                timeout=timeout,
                headers=headers or None,
            )
        except ProtocolHttpError as exc:
            return [
                YieldEntry(
                    protocol="blend",
                    pool_id="*",
                    asset="*",
                    apy=None,
                    apr=None,
                    tvl_usd=None,
                    source=url,
                    status="error",
                    error=exc.message,
                )
            ]

    pools = _extract_pools(payload)
    if not pools:
        return [
            YieldEntry(
                protocol="blend",
                pool_id="*",
                asset="*",
                apy=None,
                apr=None,
                tvl_usd=None,
                source=url,
                status="unavailable",
                error="No pool records returned by Blend API",
            )
        ]

    entries: list[YieldEntry] = []
    for pool in pools:
        asset = str(pool.get("asset") or pool.get("symbol") or pool.get("reserve") or "UNKNOWN")
        apy = _as_float(pool.get("supplyApy") or pool.get("apy") or pool.get("supply_apy"))
        apr = _as_float(pool.get("supplyApr") or pool.get("apr") or pool.get("supply_apr"))
        tvl = _as_float(pool.get("tvlUsd") or pool.get("tvl") or pool.get("tvl_usd"))
        pool_id = str(pool.get("id") or pool.get("poolId") or pool.get("address") or asset)
        entries.append(
            YieldEntry(
                protocol="blend",
                pool_id=pool_id,
                asset=asset,
                apy=apy,
                apr=apr,
                tvl_usd=tvl,
                source=f"{url}/pools",
                status="ok" if apy is not None or apr is not None else "unavailable",
                error=None if apy is not None or apr is not None else "Missing APY/APR fields",
                metadata={"raw_keys": sorted(pool.keys())},
            )
        )
    return entries


def _extract_pools(payload: Any) -> list[dict[str, Any]]:
    if payload is None:
        return []
    if isinstance(payload, list):
        return [p for p in payload if isinstance(p, dict)]
    if isinstance(payload, dict):
        for key in ("pools", "data", "reserves", "results"):
            value = payload.get(key)
            if isinstance(value, list):
                return [p for p in value if isinstance(p, dict)]
    return []


def _as_float(value: Any) -> float | None:
    if value is None or value == "":
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None
