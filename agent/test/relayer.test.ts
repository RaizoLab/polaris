import assert from "node:assert/strict";
import test from "node:test";
import {
  Account,
  Keypair,
  Networks,
  SorobanDataBuilder,
  StrKey,
} from "@stellar/stellar-sdk";
import { computeBackoffMs } from "../src/relayer/backoff.js";
import { FeeManager, classifyCongestion } from "../src/relayer/feeManager.js";
import { TransactionRelayer } from "../src/relayer/relayer.js";
import { SignalQueue, runRelayerService } from "../src/relayer/service.js";
import type {
  FeeStatsLike,
  GetTransactionLike,
  RebalanceSignal,
  SendTransactionLike,
  SimulationLike,
  SorobanRpcLike,
} from "../src/relayer/types.js";

const CONTRACT_ID = StrKey.encodeContract(Buffer.alloc(32));
const SIGNAL: RebalanceSignal = { id: "sig-1", contractId: CONTRACT_ID, method: "rebalance" };

const FEE_STATS: FeeStatsLike = {
  sorobanInclusionFee: { p50: "100", p70: "200", p90: "500", p95: "800", p99: "2000", max: "5000" },
};

function makeFakeServer(overrides: Partial<SorobanRpcLike> = {}): SorobanRpcLike & {
  sentFees: string[];
} {
  const keypair = Keypair.random();
  const sentFees: string[] = [];
  const base: SorobanRpcLike = {
    getAccount: async () => new Account(keypair.publicKey(), "1"),
    getFeeStats: async () => FEE_STATS,
    simulateTransaction: async (): Promise<SimulationLike> => ({
      minResourceFee: "1000",
      transactionData: new SorobanDataBuilder(),
    }),
    sendTransaction: async (tx): Promise<SendTransactionLike> => {
      sentFees.push(tx.fee);
      return { status: "PENDING", hash: "a".repeat(64) };
    },
    getTransaction: async (): Promise<GetTransactionLike> => ({ status: "SUCCESS", ledger: 42 }),
  };
  return Object.assign(base, overrides, { sentFees });
}

function makeRelayer(server: SorobanRpcLike, options = {}) {
  return new TransactionRelayer(
    {
      server,
      feeManager: new FeeManager(server),
      keypair: Keypair.random(),
      networkPassphrase: Networks.TESTNET,
      sleep: async () => {},
    },
    { baseDelayMs: 1, maxDelayMs: 2, ...options }
  );
}

// --- Backoff ---

test("backoff grows exponentially and caps at maxDelayMs", () => {
  const noJitter = () => 1; // upper jitter bound => full exponential value
  assert.equal(computeBackoffMs(0, { baseDelayMs: 1000, maxDelayMs: 30000 }, noJitter), 1000);
  assert.equal(computeBackoffMs(1, { baseDelayMs: 1000, maxDelayMs: 30000 }, noJitter), 2000);
  assert.equal(computeBackoffMs(3, { baseDelayMs: 1000, maxDelayMs: 30000 }, noJitter), 8000);
  assert.equal(computeBackoffMs(10, { baseDelayMs: 1000, maxDelayMs: 30000 }, noJitter), 30000);
});

test("backoff jitter stays within 50-100% of the exponential delay", () => {
  const low = computeBackoffMs(2, { baseDelayMs: 1000, maxDelayMs: 30000 }, () => 0);
  assert.equal(low, 2000); // 4000 * 0.5
  for (let i = 0; i < 20; i++) {
    const jittered = computeBackoffMs(2, { baseDelayMs: 1000, maxDelayMs: 30000 });
    assert.ok(jittered >= 2000 && jittered <= 4000);
  }
});

// --- Fee manager ---

test("fee bid follows congestion percentiles and escalates on retries", async () => {
  const manager = new FeeManager({ getFeeStats: async () => FEE_STATS });
  const first = await manager.recommendInclusionFee(0);
  const second = await manager.recommendInclusionFee(1);
  const third = await manager.recommendInclusionFee(2);

  assert.equal(first.inclusionFee, "220"); // ceil(p70 200 * 1.1)
  assert.equal(second.inclusionFee, "550"); // ceil(p90 500 * 1.1)
  assert.equal(third.inclusionFee, "2200"); // ceil(p99 2000 * 1.1)
  assert.equal(first.percentileUsed, "p70");
  assert.equal(third.percentileUsed, "p99");
});

test("fee bid is clamped between network base fee and the cap", async () => {
  const cheap = new FeeManager({
    getFeeStats: async () => ({ sorobanInclusionFee: { p70: "1" } }),
  });
  assert.equal((await cheap.recommendInclusionFee(0)).inclusionFee, "100");

  const spiked = new FeeManager(
    { getFeeStats: async () => ({ sorobanInclusionFee: { p70: "99999999" } }) },
    { capStroops: 2_000_000 }
  );
  assert.equal((await spiked.recommendInclusionFee(0)).inclusionFee, "2000000");
});

test("fee manager falls back to escalating static bids when fee stats are down", async () => {
  const manager = new FeeManager({
    getFeeStats: async () => {
      throw new Error("rpc down");
    },
  });
  const first = await manager.recommendInclusionFee(0);
  const third = await manager.recommendInclusionFee(2);
  assert.equal(first.percentileUsed, "fallback");
  assert.ok(Number(third.inclusionFee) > Number(first.inclusionFee));
});

test("congestion classification from p90 inclusion fee", () => {
  assert.equal(classifyCongestion({ p90: "100" }), "low");
  assert.equal(classifyCongestion({ p90: "600" }), "medium");
  assert.equal(classifyCongestion({ p90: "5000" }), "high");
  assert.equal(classifyCongestion({}), "unknown");
});

test("resource fee padding guards against out-of-gas", () => {
  const manager = new FeeManager({ getFeeStats: async () => FEE_STATS });
  assert.equal(manager.padResourceFee("1000"), 1150n); // 15% headroom
  assert.equal(manager.padResourceFee(1n), 2n); // ceiling, never rounds down
});

// --- Relayer ---

test("relayer submits, monitors status via RPC, and reports success", async () => {
  const statuses: GetTransactionLike[] = [
    { status: "NOT_FOUND" },
    { status: "NOT_FOUND" },
    { status: "SUCCESS", ledger: 1234 },
  ];
  const server = makeFakeServer({
    getTransaction: async () => statuses.shift() ?? { status: "SUCCESS", ledger: 1234 },
  });
  const result = await makeRelayer(server).submitSignal(SIGNAL);

  assert.equal(result.ok, true);
  assert.equal(result.status, "SUCCESS");
  assert.equal(result.attempts, 1);
  assert.equal(result.ledger, 1234);
  assert.equal(statuses.length, 0, "should poll until status resolves");
});

test("total fee = inclusion bid + padded resource fee", async () => {
  const server = makeFakeServer();
  await makeRelayer(server).submitSignal(SIGNAL);
  // inclusion 220 (p70 * 1.1) + resource 1150 (1000 * 1.15)
  assert.deepEqual(server.sentFees, ["1370"]);
});

test("relayer retries transient RPC backpressure with backoff, then succeeds", async () => {
  let sends = 0;
  const server = makeFakeServer({
    sendTransaction: async (): Promise<SendTransactionLike> => {
      sends++;
      if (sends <= 2) return { status: "TRY_AGAIN_LATER", hash: "b".repeat(64) };
      return { status: "PENDING", hash: "b".repeat(64) };
    },
  });
  const result = await makeRelayer(server).submitSignal(SIGNAL);

  assert.equal(result.ok, true);
  assert.equal(result.attempts, 3);
});

test("relayer escalates the fee percentile on each retry", async () => {
  let sends = 0;
  const fees: string[] = [];
  const server = makeFakeServer({
    sendTransaction: async (tx): Promise<SendTransactionLike> => {
      sends++;
      fees.push(tx.fee);
      if (sends <= 2) return { status: "TRY_AGAIN_LATER", hash: "c".repeat(64) };
      return { status: "PENDING", hash: "c".repeat(64) };
    },
  });
  await makeRelayer(server).submitSignal(SIGNAL);
  // resource fee constant at 1150; inclusion climbs 220 -> 550 -> 2200
  assert.deepEqual(fees, ["1370", "1700", "3350"]);
});

test("simulation errors abort without retries (would fail on-chain)", async () => {
  let simulations = 0;
  const server = makeFakeServer({
    simulateTransaction: async (): Promise<SimulationLike> => {
      simulations++;
      return { error: "host function panicked" };
    },
  });
  const result = await makeRelayer(server).submitSignal(SIGNAL);

  assert.equal(result.ok, false);
  assert.equal(result.status, "FAILED");
  assert.equal(simulations, 1, "must not retry a deterministic failure");
  assert.match(result.error ?? "", /Simulation failed/);
});

test("on-chain FAILED status aborts without retries", async () => {
  const server = makeFakeServer({
    getTransaction: async (): Promise<GetTransactionLike> => ({ status: "FAILED" }),
  });
  const result = await makeRelayer(server).submitSignal(SIGNAL);

  assert.equal(result.ok, false);
  assert.equal(result.status, "FAILED");
  assert.equal(result.attempts, 1);
});

test("relayer gives up after maxAttempts and reports ABORTED", async () => {
  let sends = 0;
  const server = makeFakeServer({
    sendTransaction: async (): Promise<SendTransactionLike> => {
      sends++;
      return { status: "TRY_AGAIN_LATER", hash: "d".repeat(64) };
    },
  });
  const result = await makeRelayer(server, { maxAttempts: 3 }).submitSignal(SIGNAL);

  assert.equal(result.ok, false);
  assert.equal(result.status, "ABORTED");
  assert.equal(result.attempts, 3);
  assert.equal(sends, 3);
});

// --- Background service ---

test("service drains queued Brain signals in order and reports results", async () => {
  const server = makeFakeServer();
  const relayer = makeRelayer(server);
  const queue = new SignalQueue();
  const seen: string[] = [];

  queue.push({ id: "first", contractId: CONTRACT_ID, method: "rebalance" });
  queue.push({ id: "second", contractId: CONTRACT_ID, method: "rebalance" });
  queue.close();

  const results = await runRelayerService(relayer, queue, (r) => seen.push(r.signalId));
  assert.deepEqual(seen, ["first", "second"]);
  assert.ok(results.every((r) => r.ok));
});
