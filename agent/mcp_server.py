"""Polaris MCP server — lets LLMs query vault state over the Model Context Protocol.

Runs over stdio, the standard transport for local MCP clients (Claude Desktop,
Cursor, etc.). Register it with an MCP client like:

    {
      "mcpServers": {
        "polaris": {
          "command": "python",
          "args": ["/path/to/polaris/agent/mcp_server.py"]
        }
      }
    }

Run directly:  python mcp_server.py  (from agent/, with requirements installed)
"""

from __future__ import annotations

import sys
from pathlib import Path

# Ensure sibling modules resolve when launched from any cwd by an MCP client.
sys.path.insert(0, str(Path(__file__).resolve().parent))

from mcp.server.fastmcp import FastMCP

import mcp_tools

mcp = FastMCP(
    "polaris",
    instructions=(
        "Polaris is a Stellar DeFi yield agent. Use vault_yield to answer questions about "
        "the user's current vault yield, market_scan to survey yields across Stellar "
        "protocols (Blend, Phoenix, Soroswap), rebalance_check to decide whether the vault "
        "should be rebalanced, and forecast_yields for the 24h ML yield forecast."
    ),
)


@mcp.tool()
def vault_yield(network: str = "mainnet") -> dict:
    """Current yield of the Polaris vault: blended APY plus per-position breakdown.

    Use this to answer questions like "What is the current yield in my vault?".
    """
    return mcp_tools.vault_yield(network=network)


@mcp.tool()
def market_scan(network: str = "mainnet") -> dict:
    """Scan Stellar DeFi markets (Blend, Phoenix, Soroswap) and rank yield opportunities."""
    return mcp_tools.market_scan(network=network)


@mcp.tool()
def rebalance_check(network: str = "mainnet") -> dict:
    """Check whether the vault should rebalance: current vs model-optimal allocation."""
    return mcp_tools.rebalance_check(network=network)


@mcp.tool()
def forecast_yields() -> dict:
    """24h yield forecast per pool and the model's Optimal Allocation array."""
    return mcp_tools.forecast_yields()


if __name__ == "__main__":
    mcp.run()
