/**
 * Generate a Groth16 proof that a trade followed the Polaris strategy.
 *
 * Usage: node scripts/prove.js [input.json] [outDir]
 *
 * input.json shape (all values are strings or numbers):
 *   {
 *     "apys": ["480", "240", "1120", "360"],  // candidate APYs in bps
 *     "chosen": 2,                             // intended target protocol index
 *     "amount": "500000000",                   // trade amount (stroops)
 *     "vaultTotal": "10000000000"              // vault balance (stroops)
 *   }
 *
 * Outputs <outDir>/strategy.proof and <outDir>/public.json. The public
 * signals include the intended target protocol and amount, in this order:
 *   [apys[0..3], chosen, amount, vaultTotal]
 */

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as snarkjs from "snarkjs";

const root = dirname(dirname(fileURLToPath(import.meta.url)));

export function buildWitnessInput(input) {
  const { apys, chosen, amount, vaultTotal } = input;
  if (!Array.isArray(apys) || apys.length !== 4) {
    throw new Error("input.apys must be an array of 4 basis-point values");
  }
  const chosenIdx = Number(chosen);
  const selector = apys.map((_, i) => (i === chosenIdx ? "1" : "0"));
  return {
    apys: apys.map(String),
    chosen: String(chosen),
    amount: String(amount),
    vaultTotal: String(vaultTotal),
    selector,
  };
}

export async function prove(input, outDir = resolve(root, "build")) {
  const wasm = resolve(root, "build/strategy_js/strategy.wasm");
  const zkey = resolve(root, "build/strategy_final.zkey");

  const { proof, publicSignals } = await snarkjs.groth16.fullProve(
    buildWitnessInput(input),
    wasm,
    zkey
  );

  mkdirSync(outDir, { recursive: true });
  const proofPath = resolve(outDir, "strategy.proof");
  const publicPath = resolve(outDir, "public.json");
  writeFileSync(proofPath, JSON.stringify(proof, null, 2));
  writeFileSync(publicPath, JSON.stringify(publicSignals, null, 2));
  return { proof, publicSignals, proofPath, publicPath };
}

export async function verify(proof, publicSignals) {
  const vk = JSON.parse(readFileSync(resolve(root, "build/verification_key.json"), "utf8"));
  return snarkjs.groth16.verify(vk, publicSignals, proof);
}

const isDirectRun = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isDirectRun) {
  const inputPath = process.argv[2] ?? resolve(root, "inputs/sample_trade.json");
  const outDir = process.argv[3] ?? resolve(root, "build");
  const input = JSON.parse(readFileSync(inputPath, "utf8"));
  prove(input, outDir)
    .then(async ({ proof, publicSignals, proofPath, publicPath }) => {
      const ok = await verify(proof, publicSignals);
      console.log(JSON.stringify({ ok, proofPath, publicPath, publicSignals }, null, 2));
      process.exit(ok ? 0 : 1);
    })
    .catch((err) => {
      console.error(JSON.stringify({ ok: false, error: err.message }));
      process.exit(1);
    });
}
