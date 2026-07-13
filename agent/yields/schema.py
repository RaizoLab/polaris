"""Standardized yield record schema for Stellar DeFi protocols."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from typing import Any, Literal


ProtocolName = Literal["blend", "phoenix", "soroswap"]
YieldStatus = Literal["ok", "error", "unavailable"]


@dataclass
class YieldEntry:
    protocol: ProtocolName
    pool_id: str
    asset: str
    apy: float | None
    apr: float | None
    tvl_usd: float | None
    source: str
    status: YieldStatus
    error: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class YieldSnapshot:
    network: str
    fetched_at: str
    yields: list[YieldEntry]
    errors: list[dict[str, str]] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return {
            "network": self.network,
            "fetched_at": self.fetched_at,
            "yields": [y.to_dict() for y in self.yields],
            "errors": self.errors,
            "summary": {
                "total": len(self.yields),
                "ok": sum(1 for y in self.yields if y.status == "ok"),
                "failed_protocols": sorted({e["protocol"] for e in self.errors}),
            },
        }


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat()
