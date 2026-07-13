/**
 * Initialize Polaris agent via the Stellar AI Agent Kit.
 *
 * stellar-agent-kit currently targets mainnet DeFi helpers; for testnet Hello World
 * payments we use @stellar/stellar-sdk directly (see helloWorld.ts) while still
 * constructing / initializing the kit when on mainnet.
 */
import { StellarAgentKit } from "stellar-agent-kit";
import { loadAgentEnv, type AgentEnv } from "./config.js";

export interface PolarisAgent {
  env: AgentEnv;
  kit: StellarAgentKit | null;
  publicKey: string;
}

export async function createPolarisAgent(
  env: AgentEnv = loadAgentEnv()
): Promise<PolarisAgent> {
  const { Keypair } = await import("@stellar/stellar-sdk");
  const keypair = Keypair.fromSecret(env.secretKey);

  let kit: StellarAgentKit | null = null;
  if (env.network === "mainnet") {
    kit = new StellarAgentKit(env.secretKey, "mainnet");
    await kit.initialize();
  }

  return {
    env,
    kit,
    publicKey: keypair.publicKey(),
  };
}
