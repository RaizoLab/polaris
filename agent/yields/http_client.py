"""HTTP helpers with timeout and structured error handling for protocol APIs."""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Any


class ProtocolHttpError(Exception):
    def __init__(self, protocol: str, message: str, status_code: int | None = None) -> None:
        super().__init__(message)
        self.protocol = protocol
        self.status_code = status_code
        self.message = message


def http_get_json(
    url: str,
    *,
    protocol: str,
    timeout: float = 10.0,
    headers: dict[str, str] | None = None,
) -> Any:
    req_headers = {"Accept": "application/json", "User-Agent": "polaris-agent/0.1"}
    if headers:
        req_headers.update(headers)

    request = urllib.request.Request(url, headers=req_headers, method="GET")
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read().decode("utf-8")
            if not body:
                return None
            return json.loads(body)
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")[:300]
        raise ProtocolHttpError(
            protocol,
            f"HTTP {exc.code} from {url}: {detail or exc.reason}",
            status_code=exc.code,
        ) from exc
    except urllib.error.URLError as exc:
        raise ProtocolHttpError(protocol, f"Network error contacting {url}: {exc.reason}") from exc
    except TimeoutError as exc:
        raise ProtocolHttpError(protocol, f"Timeout contacting {url}") from exc
    except json.JSONDecodeError as exc:
        raise ProtocolHttpError(protocol, f"Invalid JSON from {url}: {exc}") from exc
