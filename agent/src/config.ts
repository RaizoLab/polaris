import { config as loadDotenv } from "dotenv";
import { Keypair, Networks } from "@stellar/stellar-sdk";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

loadDotenv({ path: resolve(process.cwd(), ".env") });
if (existsSync(resolve(process.cwd(), "../.env"))) {
  loadDotenv({ path: resolve(process.cwd(), "../.env") });
}

export type StellarNetworkName = "testnet" | "mainnet";

export interface AgentEnv {
  network: StellarNetworkName;
  horizonUrl: string;
  rpcUrl: string;
  secretKey: string;
  helloWorldDestination?: string;
  helloWorldAmount: string;
}

function required(name: string, value: string | undefined): string {
  if (!value || !value.trim()) {
    throw new Error(
      `Missing required environment variable ${name}. Copy agent/.env.example to agent/.env and set it.`
    );
  }
  return value.trim();
}

function networkName(): StellarNetworkName {
  const raw = (process.env.STELLAR_NETWORK ?? "testnet").toLowerCase();
  if (raw !== "testnet" && raw !== "mainnet") {
    throw new Error(`STELLAR_NETWORK must be "testnet" or "mainnet", got "${raw}"`);
  }
  return raw;
}

const DEFAULTS = {
  testnet: {
    horizonUrl: "https://horizon-testnet.stellar.org",
    rpcUrl: "https://soroban-testnet.stellar.org",
  },
  mainnet: {
    horizonUrl: "https://horizon.stellar.org",
    rpcUrl: "https://mainnet.sorobanrpc.com",
  },
} as const;

export function loadAgentEnv(): AgentEnv {
  const network = networkName();
  const defaults = DEFAULTS[network];

  return {
    network,
    horizonUrl: process.env.STELLAR_HORIZON_URL?.trim() || defaults.horizonUrl,
    rpcUrl: process.env.STELLAR_RPC_URL?.trim() || defaults.rpcUrl,
    secretKey: required("STELLAR_SECRET_KEY", process.env.STELLAR_SECRET_KEY),
    helloWorldDestination: process.env.HELLO_WORLD_DESTINATION?.trim() || undefined,
    helloWorldAmount: process.env.HELLO_WORLD_AMOUNT?.trim() || "0.0000001",
  };
}

export function networkPassphrase(network: StellarNetworkName): string {
  return network === "mainnet" ? Networks.PUBLIC : Networks.TESTNET;
}

export function keypairFromEnv(env: AgentEnv): Keypair {
  try {
    return Keypair.fromSecret(env.secretKey);
  } catch {
    throw new Error("STELLAR_SECRET_KEY is not a valid Stellar secret key (must start with S).");
  }
}
