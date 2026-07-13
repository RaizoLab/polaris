"""Aggregate real-time APY data across Blend, Phoenix, and Soroswap."""

from __future__ import annotations

import argparse
import json
import os
import sys
from typing import Any

from yields.providers.blend import fetch_blend_yields
from yields.providers.phoenix import fetch_phoenix_yields
from yields.providers.soroswap import fetch_soroswap_yields
from yields.schema import YieldSnapshot, utc_now_iso


def fetch_all_yields(
    *,
    network: str | None = None,
    timeout: float | None = None,
) -> dict[str, Any]:
    net = (network or os.getenv("STELLAR_NETWORK") or "mainnet").lower()
    api_network = "MAINNET" if net == "mainnet" else "TESTNET"
    http_timeout = timeout if timeout is not None else float(os.getenv("YIELD_HTTP_TIMEOUT", "10"))

    providers = (
        ("blend", lambda: fetch_blend_yields(timeout=http_timeout)),
        ("phoenix", lambda: fetch_phoenix_yields(timeout=http_timeout, network=api_network)),
        ("soroswap", lambda: fetch_soroswap_yields(timeout=http_timeout, network=api_network)),
    )

    yields = []
    errors: list[dict[str, str]] = []

    for name, loader in providers:
        try:
            entries = loader()
            yields.extend(entries)
            for entry in entries:
                if entry.status == "error":
                    errors.append({"protocol": name, "error": entry.error or "unknown error"})
        except Exception as exc:  # noqa: BLE001 — isolate provider failures
            errors.append({"protocol": name, "error": str(exc)})

    snapshot = YieldSnapshot(
        network=net,
        fetched_at=utc_now_iso(),
        yields=yields,
        errors=errors,
    )
    return snapshot.to_dict()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Fetch Stellar DeFi yields as JSON")
    parser.add_argument("--network", default=None, help="testnet|mainnet")
    parser.add_argument("--timeout", type=float, default=None, help="HTTP timeout seconds")
    args = parser.parse_args(argv)

    payload = fetch_all_yields(network=args.network, timeout=args.timeout)
    json.dump(payload, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0 if not payload.get("errors") else 0  # always emit JSON; errors are in payload


if __name__ == "__main__":
    raise SystemExit(main())
