import assert from "node:assert/strict";
import test from "node:test";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildWitnessInput, prove, verify } from "../scripts/prove.js";
import { exportProof, exportVk } from "../scripts/export_soroban.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const sampleInput = JSON.parse(readFileSync(resolve(root, "inputs/sample_trade.json"), "utf8"));

test("witness input derives a one-hot selector from the chosen index", () => {
  const witness = buildWitnessInput(sampleInput);
  assert.deepEqual(witness.selector, ["0", "0", "1", "0"]);
  assert.equal(witness.chosen, "2");
});

test("prover outputs a valid .proof and public.json for a given input", async (t) => {
  const outDir = resolve(root, "build/test-out");
  const { proof, publicSignals, proofPath, publicPath } = await prove(sampleInput, outDir);

  assert.ok(existsSync(proofPath), "strategy.proof must be written");
  assert.ok(existsSync(publicPath), "public.json must be written");
  assert.equal(proof.protocol, "groth16");
  assert.equal(proof.curve, "bls12381");

  const ok = await verify(proof, publicSignals);
  assert.equal(ok, true, "proof must verify against the verification key");
});

test("public signals include the intended target protocol and amount", async () => {
  const { publicSignals } = await prove(sampleInput, resolve(root, "build/test-out"));
  // Order: [apys[0..3], chosen, amount, vaultTotal]
  assert.equal(publicSignals[4], String(sampleInput.chosen));
  assert.equal(publicSignals[5], String(sampleInput.amount));
  assert.equal(publicSignals[6], String(sampleInput.vaultTotal));
});

test("strategy violations cannot be proven", async () => {
  // Chosen protocol (index 0, 480bps) is not the max (index 2, 1120bps).
  const notMax = { ...sampleInput, chosen: 0 };
  await assert.rejects(() => prove(notMax, resolve(root, "build/test-out")), /Assert Failed|Error/);

  // Amount exceeds the 60% max-weight cap.
  const overCap = { ...sampleInput, amount: "6100000000" };
  await assert.rejects(() => prove(overCap, resolve(root, "build/test-out")), /Assert Failed|Error/);
});

test("soroban export produces host-compatible byte encodings", async () => {
  const vk = exportVk(JSON.parse(readFileSync(resolve(root, "build/verification_key.json"), "utf8")));
  const { proof, publicSignals } = await prove(sampleInput, resolve(root, "build/test-out"));
  const exported = exportProof(proof, publicSignals);

  const hex = /^[0-9a-f]+$/;
  assert.equal(exported.a.length, 192, "G1 = 96 bytes");
  assert.equal(exported.b.length, 384, "G2 = 192 bytes");
  assert.equal(exported.c.length, 192);
  assert.equal(vk.alpha_neg.length, 192);
  assert.equal(vk.beta.length, 384);
  assert.equal(vk.gamma_neg.length, 384);
  assert.equal(vk.delta_neg.length, 384);
  assert.equal(vk.ic.length, publicSignals.length + 1, "IC = public inputs + 1");
  for (const scalar of exported.public_inputs) {
    assert.equal(scalar.length, 64, "Fr = 32 bytes");
    assert.match(scalar, hex);
  }
});
