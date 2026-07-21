/**
 * Transaction relayer — the Polaris execution engine.
 *
 * Consumes rebalance signals from the Brain and drives each one through the
 * full submission lifecycle against Soroban RPC:
 *
 *   fresh sequence -> dynamic fee bid -> simulate (resource estimate)
 *   -> pad resource fee -> sign -> send -> poll status
 *
 * Failed attempts retry with exponential backoff (and an escalated fee bid);
 * on-chain failures and simulation errors abort immediately since a resubmit
 * would fail identically. Transactions carry tight timebounds so an attempt
 * that outlives its polling window can never land later and double-execute.
 */

import {
  Keypair,
  Operation,
  TransactionBuilder,
  type Transaction,
} from "@stellar/stellar-sdk";
import { computeBackoffMs, type BackoffOptions } from "./backoff.js";
import type { FeeManager } from "./feeManager.js";
import type {
  GetTransactionLike,
  RebalanceSignal,
  RelayResult,
  SorobanRpcLike,
} from "./types.js";

export class RelayerError extends Error {
  constructor(message: string, readonly retryable: boolean) {
    super(message);
    this.name = "RelayerError";
  }
}

export interface RelayerOptions extends BackoffOptions {
  /** Total attempts per signal (initial + retries). */
  maxAttempts?: number;
  /** Transaction validity window (timebounds), seconds. */
  txTimeoutSeconds?: number;
  /** Delay between status polls. */
  pollIntervalMs?: number;
  /** Give up polling after this long (kept > txTimeoutSeconds). */
  pollTimeoutMs?: number;
}

const DEFAULT_OPTIONS: Required<RelayerOptions> = {
  maxAttempts: 4,
  baseDelayMs: 1_000,
  maxDelayMs: 30_000,
  txTimeoutSeconds: 60,
  pollIntervalMs: 2_000,
  pollTimeoutMs: 90_000,
};

export interface RelayerDeps {
  server: SorobanRpcLike;
  feeManager: FeeManager;
  keypair: Keypair;
  networkPassphrase: string;
  /** Injectable for tests; defaults to real timers. */
  sleep?: (ms: number) => Promise<void>;
  /** Injectable clock for tests. */
  now?: () => number;
  log?: (message: string, context?: Record<string, unknown>) => void;
}

interface AttemptSuccess {
  hash: string;
  ledger?: number;
  feeCharged: string;
}

export class TransactionRelayer {
  private readonly opts: Required<RelayerOptions>;
  private readonly sleep: (ms: number) => Promise<void>;
  private readonly now: () => number;
  private readonly log: (message: string, context?: Record<string, unknown>) => void;

  constructor(private readonly deps: RelayerDeps, options: RelayerOptions = {}) {
    this.opts = { ...DEFAULT_OPTIONS, ...options };
    this.sleep = deps.sleep ?? ((ms) => new Promise((r) => setTimeout(r, ms)));
    this.now = deps.now ?? Date.now;
    this.log = deps.log ?? (() => {});
  }

  /** Submit one Brain signal, retrying transient failures with backoff. */
  async submitSignal(signal: RebalanceSignal): Promise<RelayResult> {
    const signalId = signal.id ?? `${signal.method}-${this.now()}`;
    let lastError = "unknown error";

    for (let attempt = 0; attempt < this.opts.maxAttempts; attempt++) {
      try {
        const outcome = await this.attemptOnce(signal, attempt);
        this.log("relay success", { signalId, attempt, hash: outcome.hash });
        return {
          ok: true,
          signalId,
          status: "SUCCESS",
          attempts: attempt + 1,
          hash: outcome.hash,
          ledger: outcome.ledger,
          feeCharged: outcome.feeCharged,
        };
      } catch (err) {
        const retryable = err instanceof RelayerError ? err.retryable : true;
        lastError = err instanceof Error ? err.message : String(err);
        this.log("relay attempt failed", { signalId, attempt, retryable, error: lastError });

        if (!retryable) {
          return {
            ok: false,
            signalId,
            status: "FAILED",
            attempts: attempt + 1,
            error: lastError,
          };
        }
        if (attempt < this.opts.maxAttempts - 1) {
          await this.sleep(computeBackoffMs(attempt, this.opts));
        }
      }
    }

    return {
      ok: false,
      signalId,
      status: "ABORTED",
      attempts: this.opts.maxAttempts,
      error: `Exhausted ${this.opts.maxAttempts} attempts: ${lastError}`,
    };
  }

  private async attemptOnce(signal: RebalanceSignal, attempt: number): Promise<AttemptSuccess> {
    const { server, feeManager, keypair, networkPassphrase } = this.deps;

    // Fresh sequence number every attempt (guards against txBadSeq on retry).
    const account = await wrapNetwork(
      () => server.getAccount(keypair.publicKey()),
      "loading account"
    );
    const fee = await feeManager.recommendInclusionFee(attempt);

    const draft = new TransactionBuilder(account, {
      fee: fee.inclusionFee,
      networkPassphrase,
    })
      .addOperation(
        Operation.invokeContractFunction({
          contract: signal.contractId,
          function: signal.method,
          args: signal.args ?? [],
        })
      )
      .setTimeout(this.opts.txTimeoutSeconds)
      .build();

    // Fresh simulation every attempt: resources are re-estimated against
    // current ledger state, then padded, preventing out-of-gas mid-rebalance.
    const sim = await wrapNetwork(() => server.simulateTransaction(draft), "simulating");
    if ("error" in sim) {
      throw new RelayerError(`Simulation failed (would fail on-chain): ${sim.error}`, false);
    }

    const paddedResourceFee = feeManager.padResourceFee(sim.minResourceFee);
    // TransactionBuilder adds sorobanData's resource fee to the inclusion fee
    // itself, so pass only the inclusion bid here.
    const tx = TransactionBuilder.cloneFrom(draft, {
      fee: fee.inclusionFee,
      sorobanData: sim.transactionData.setResourceFee(paddedResourceFee).build(),
    }).build();
    tx.sign(keypair);
    const totalFee = BigInt(tx.fee);

    this.log("submitting", {
      attempt,
      inclusionFee: fee.inclusionFee,
      percentile: fee.percentileUsed,
      congestion: fee.congestion,
      resourceFee: paddedResourceFee.toString(),
    });

    const sent = await wrapNetwork(() => server.sendTransaction(tx as Transaction), "sending");
    switch (sent.status) {
      case "PENDING":
      case "DUPLICATE":
        return this.pollStatus(sent.hash, totalFee.toString());
      case "TRY_AGAIN_LATER":
        throw new RelayerError("RPC backpressure (TRY_AGAIN_LATER)", true);
      case "ERROR":
        // Covers txBadSeq / txInsufficientFee races; the next attempt rebuilds
        // with a fresh sequence and a higher fee percentile.
        throw new RelayerError("sendTransaction rejected (ERROR)", true);
    }
  }

  /** Monitor transaction status via Soroban RPC until it resolves. */
  private async pollStatus(hash: string, feeCharged: string): Promise<AttemptSuccess> {
    const deadline = this.now() + this.opts.pollTimeoutMs;

    while (this.now() < deadline) {
      const result: GetTransactionLike = await wrapNetwork(
        () => this.deps.server.getTransaction(hash),
        "polling status"
      );
      if (result.status === "SUCCESS") {
        return { hash, ledger: result.ledger, feeCharged };
      }
      if (result.status === "FAILED") {
        throw new RelayerError(`Transaction ${hash} failed on-chain`, false);
      }
      await this.sleep(this.opts.pollIntervalMs);
    }

    // Timebounds have expired by now, so the attempt cannot land later;
    // resubmitting with a fresh sequence is safe.
    throw new RelayerError(`Timed out waiting for transaction ${hash}`, true);
  }
}

/** Network/transport errors are always retryable. */
async function wrapNetwork<T>(fn: () => Promise<T>, action: string): Promise<T> {
  try {
    return await fn();
  } catch (err) {
    if (err instanceof RelayerError) throw err;
    const message = err instanceof Error ? err.message : String(err);
    throw new RelayerError(`Network error while ${action}: ${message}`, true);
  }
}
