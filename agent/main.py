"""Polaris off-chain agent entry point."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def run_hello_world() -> int:
    """Delegate Hello World tx to the TypeScript Stellar Agent Kit flow."""
    agent_dir = Path(__file__).resolve().parent
    cmd = ["npm", "run", "hello", "--silent"]
    try:
        completed = subprocess.run(cmd, cwd=agent_dir, check=False)
        return completed.returncode
    except FileNotFoundError:
        print(
            json.dumps(
                {
                    "ok": False,
                    "error": "npm not found. Install Node.js 18+ and run `npm install` in agent/.",
                }
            ),
            file=sys.stderr,
        )
        return 1


def run_yields(network: str | None) -> int:
    from yields.fetch_yields import fetch_all_yields

    payload = fetch_all_yields(network=network)
    print(json.dumps(payload, indent=2))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Polaris agent engine")
    parser.add_argument(
        "command",
        nargs="?",
        default="status",
        choices=["status", "hello", "yields"],
        help="status | hello | yields",
    )
    parser.add_argument("--network", default=None, help="testnet|mainnet (yields)")
    args = parser.parse_args(argv)

    if args.command == "status":
        print(
            json.dumps(
                {
                    "ok": True,
                    "agent": "polaris",
                    "commands": ["status", "hello", "yields"],
                    "hint": "Copy agent/.env.example to agent/.env, fund a testnet key, then run: python main.py hello",
                },
                indent=2,
            )
        )
        return 0
    if args.command == "hello":
        return run_hello_world()
    if args.command == "yields":
        return run_yields(args.network)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
