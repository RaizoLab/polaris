/**
 * Dynamic gas & fee manager for Soroban transactions.
 *
 * Inclusion fee: read live fee stats from Soroban RPC (`getFeeStats`) and bid
 * at a congestion percentile that escalates on each retry, so transactions
 * land in the next ledger without a static overpaying bid.
 *
 * Resource fee ("gas"): pad the simulation's minResourceFee by a safety
 * margin so complex rebalances don't fail with out-of-gas style errors when
 * ledger state shifts between simulation and execution.
 */

import type { FeePercentile, FeeStatsLike } from "./types.js";

/** Stellar network minimum inclusion fee, in stroops. */
export const BASE_FEE_STROOPS = 100;

/** Percentile bid ladder: first attempt p70, then p90, then p99. */
const PERCENTILE_LADDER: FeePercentile[] = ["p70", "p90", "p99"];

export type CongestionLevel = "low" | "medium" | "high" | "unknown";

export interface FeeRecommendation {
  /** Inclusion fee bid in stroops (as required by TransactionBuilder). */
  inclusionFee: string;
  percentileUsed: FeePercentile | "fallback";
  congestion: CongestionLevel;
}

export interface FeeManagerOptions {
  /** Never bid below this (network base fee). */
  floorStroops?: number;
  /** Hard cap so congestion spikes cannot drain the account. */
  capStroops?: number;
  /** Headroom multiplier applied over the observed percentile. */
  inclusionBuffer?: number;
  /** Multiplier applied over simulated minResourceFee (out-of-gas guard). */
  resourceFeeSafetyMultiplier?: number;
}

const DEFAULTS: Required<FeeManagerOptions> = {
  floorStroops: BASE_FEE_STROOPS,
  capStroops: 2_000_000, // 0.2 XLM
  inclusionBuffer: 1.1,
  resourceFeeSafetyMultiplier: 1.15,
};

export class FeeManager {
  private readonly opts: Required<FeeManagerOptions>;

  constructor(
    private readonly rpc: { getFeeStats(): Promise<FeeStatsLike> },
    options: FeeManagerOptions = {}
  ) {
    this.opts = { ...DEFAULTS, ...options };
  }

  /**
   * Recommend an inclusion fee for the given retry attempt (0-based).
   * Falls back to an escalating static bid if fee stats are unavailable.
   */
  async recommendInclusionFee(attempt = 0): Promise<FeeRecommendation> {
    let stats: FeeStatsLike;
    try {
      stats = await this.rpc.getFeeStats();
    } catch {
      return {
        inclusionFee: String(
          this.clamp(this.opts.floorStroops * 2 * (attempt + 1))
        ),
        percentileUsed: "fallback",
        congestion: "unknown",
      };
    }

    const dist = stats.sorobanInclusionFee ?? {};
    const percentile = PERCENTILE_LADDER[Math.min(attempt, PERCENTILE_LADDER.length - 1)];
    const observed =
      toNumber(dist[percentile]) ?? toNumber(dist.max) ?? this.opts.floorStroops;

    // Integer math avoids float drift (e.g. 200 * 1.1 === 220.00000000000003).
    const bufferPct = Math.round(this.opts.inclusionBuffer * 100);
    const bid = this.clamp(Math.ceil((observed * bufferPct) / 100));
    return {
      inclusionFee: String(bid),
      percentileUsed: percentile,
      congestion: classifyCongestion(dist),
    };
  }

  /** Pad the simulated minimum resource fee to prevent out-of-gas failures. */
  padResourceFee(minResourceFee: string | number | bigint): bigint {
    const min = BigInt(minResourceFee);
    const pct = BigInt(Math.round(this.opts.resourceFeeSafetyMultiplier * 100));
    return (min * pct + 99n) / 100n; // ceiling division
  }

  private clamp(fee: number): number {
    return Math.max(this.opts.floorStroops, Math.min(this.opts.capStroops, fee));
  }
}

export function classifyCongestion(dist: FeeStatsLike["sorobanInclusionFee"]): CongestionLevel {
  const p90 = toNumber(dist?.p90);
  if (p90 == null) return "unknown";
  if (p90 <= BASE_FEE_STROOPS * 1.5) return "low";
  if (p90 <= BASE_FEE_STROOPS * 10) return "medium";
  return "high";
}

function toNumber(value: string | undefined): number | null {
  if (value == null || value === "") return null;
  const n = Number(value);
  return Number.isFinite(n) && n > 0 ? n : null;
}
