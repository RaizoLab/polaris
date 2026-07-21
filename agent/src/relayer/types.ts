/**
 * Narrow structural interfaces over Soroban RPC so the relayer and fee
 * manager can be unit-tested with fakes. `rpc.Server` from
 * @stellar/stellar-sdk satisfies all of them.
 */
import type { Account, Transaction, xdr } from "@stellar/stellar-sdk";

export type FeePercentile = "p50" | "p70" | "p90" | "p95" | "p99";

export interface FeeDistributionLike {
  max?: string;
  min?: string;
  mode?: string;
  p50?: string;
  p70?: string;
  p90?: string;
  p95?: string;
  p99?: string;
}

export interface FeeStatsLike {
  sorobanInclusionFee: FeeDistributionLike;
}

export interface SorobanDataBuilderLike {
  setResourceFee(fee: bigint): { build(): xdr.SorobanTransactionData };
}

export type SimulationLike =
  | { error: string }
  | { minResourceFee: string; transactionData: SorobanDataBuilderLike };

export interface SendTransactionLike {
  status: "PENDING" | "DUPLICATE" | "TRY_AGAIN_LATER" | "ERROR";
  hash: string;
  errorResult?: unknown;
}

export interface GetTransactionLike {
  status: "NOT_FOUND" | "SUCCESS" | "FAILED";
  ledger?: number;
  resultXdr?: unknown;
}

export interface SorobanRpcLike {
  getAccount(address: string): Promise<Account>;
  getFeeStats(): Promise<FeeStatsLike>;
  simulateTransaction(tx: Transaction): Promise<SimulationLike>;
  sendTransaction(tx: Transaction): Promise<SendTransactionLike>;
  getTransaction(hash: string): Promise<GetTransactionLike>;
}

/** A rebalancing instruction emitted by the Brain. */
export interface RebalanceSignal {
  /** Correlation id for logs; generated if omitted. */
  id?: string;
  /** Target contract (C...) — typically the Polaris vault or an adapter. */
  contractId: string;
  /** Contract function to invoke, e.g. "rebalance". */
  method: string;
  /** Invocation arguments as ScVals (already encoded by the Brain). */
  args?: xdr.ScVal[];
}

export interface RelayResult {
  ok: boolean;
  signalId: string;
  hash?: string;
  status: "SUCCESS" | "FAILED" | "ABORTED";
  attempts: number;
  ledger?: number;
  feeCharged?: string;
  error?: string;
}
