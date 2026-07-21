/**
 * Polaris relayer service entry point.
 *
 * Listens for Brain signals as JSON lines on stdin and relays each one to
 * Soroban with dynamic fees, retries, and status monitoring. Example:
 *
 *   echo '{"contractId":"C...","method":"rebalance"}' | npm run relayer
 *
 * Any process (the Python brain, a cron job, another service) can pipe
 * signals in; results are emitted as JSON lines on stdout.
 */

import { createInterface } from "node:readline";
import { rpc } from "@stellar/stellar-sdk";
import { keypairFromEnv, loadAgentEnv, networkPassphrase } from "./config.js";
import {
  FeeManager,
  SignalQueue,
  TransactionRelayer,
  runRelayerService,
  type RebalanceSignal,
} from "./relayer/index.js";

function parseSignal(line: string): RebalanceSignal | null {
  const trimmed = line.trim();
  if (!trimmed) return null;
  try {
    const parsed = JSON.parse(trimmed) as Partial<RebalanceSignal>;
    if (typeof parsed.contractId !== "string" || typeof parsed.method !== "string") {
      throw new Error("signal requires contractId and method");
    }
    return { id: parsed.id, contractId: parsed.contractId, method: parsed.method };
  } catch (err) {
    console.error(
      JSON.stringify({ ok: false, error: `Ignored malformed signal: ${String(err)}`, line: trimmed })
    );
    return null;
  }
}

async function main(): Promise<void> {
  const env = loadAgentEnv();
  const server = new rpc.Server(env.rpcUrl, { allowHttp: env.rpcUrl.startsWith("http://") });
  const relayer = new TransactionRelayer({
    server,
    feeManager: new FeeManager(server),
    keypair: keypairFromEnv(env),
    networkPassphrase: networkPassphrase(env.network),
    log: (message, context) => console.error(JSON.stringify({ level: "info", message, ...context })),
  });

  const queue = new SignalQueue();
  const stdin = createInterface({ input: process.stdin });
  stdin.on("line", (line) => {
    const signal = parseSignal(line);
    if (signal) queue.push(signal);
  });
  stdin.on("close", () => queue.close());

  console.error(
    JSON.stringify({
      level: "info",
      message: "Polaris relayer listening for signals on stdin (JSON lines)",
      network: env.network,
      rpcUrl: env.rpcUrl,
    })
  );

  await runRelayerService(relayer, queue, (result) => {
    console.log(JSON.stringify(result));
  });
}

main().catch((err) => {
  console.error(JSON.stringify({ ok: false, error: err instanceof Error ? err.message : String(err) }));
  process.exit(1);
});
