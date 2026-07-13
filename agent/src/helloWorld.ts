/**
 * Sign and submit a "Hello World" payment on Stellar.
 *
 * Creates a 0.0000001 XLM (or configured) payment with memo text "Hello World",
 * signs with STELLAR_SECRET_KEY, and submits via Horizon.
 */
import {
  Asset,
  Horizon,
  Memo,
  Operation,
  TransactionBuilder,
} from "@stellar/stellar-sdk";
import {
  keypairFromEnv,
  loadAgentEnv,
  networkPassphrase,
  type AgentEnv,
} from "./config.js";

export interface HelloWorldResult {
  hash: string;
  network: string;
  source: string;
  destination: string;
  amount: string;
  memo: string;
  horizonUrl: string;
}

export async function submitHelloWorld(
  env: AgentEnv = loadAgentEnv()
): Promise<HelloWorldResult> {
  const keypair = keypairFromEnv(env);
  const source = keypair.publicKey();
  const destination = env.helloWorldDestination || source;
  const amount = env.helloWorldAmount;
  const memoText = "Hello World";

  const server = new Horizon.Server(env.horizonUrl);
  const account = await server.loadAccount(source);

  const tx = new TransactionBuilder(account, {
    fee: "100000",
    networkPassphrase: networkPassphrase(env.network),
  })
    .addOperation(
      Operation.payment({
        destination,
        asset: Asset.native(),
        amount,
      })
    )
    .addMemo(Memo.text(memoText))
    .setTimeout(180)
    .build();

  tx.sign(keypair);

  const response = await server.submitTransaction(tx);
  const hash =
    typeof response === "object" && response !== null && "hash" in response
      ? String((response as { hash: string }).hash)
      : String(response);

  return {
    hash,
    network: env.network,
    source,
    destination,
    amount,
    memo: memoText,
    horizonUrl: env.horizonUrl,
  };
}

async function main(): Promise<void> {
  const result = await submitHelloWorld();
  console.log(JSON.stringify({ ok: true, ...result }, null, 2));
}

const isDirectRun =
  process.argv[1]?.endsWith("helloWorld.ts") ||
  process.argv[1]?.endsWith("helloWorld.js");

if (isDirectRun) {
  main().catch((err) => {
    console.error(
      JSON.stringify(
        {
          ok: false,
          error: err instanceof Error ? err.message : String(err),
        },
        null,
        2
      )
    );
    process.exit(1);
  });
}
