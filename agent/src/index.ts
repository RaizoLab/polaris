import { createPolarisAgent } from "./agent.js";
import { submitHelloWorld } from "./helloWorld.js";
import { loadAgentEnv } from "./config.js";

async function main(): Promise<void> {
  const env = loadAgentEnv();
  const agent = await createPolarisAgent(env);

  console.log(
    JSON.stringify(
      {
        status: "initialized",
        network: env.network,
        publicKey: agent.publicKey,
        horizonUrl: env.horizonUrl,
        rpcUrl: env.rpcUrl,
        kitReady: agent.kit !== null,
      },
      null,
      2
    )
  );

  const result = await submitHelloWorld(env);
  console.log(JSON.stringify({ status: "hello_world_submitted", ...result }, null, 2));
}

main().catch((err) => {
  console.error(
    JSON.stringify(
      {
        status: "error",
        error: err instanceof Error ? err.message : String(err),
      },
      null,
      2
    )
  );
  process.exit(1);
});
