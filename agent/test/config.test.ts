import assert from "node:assert/strict";
import test from "node:test";
import { Keypair } from "@stellar/stellar-sdk";
import { networkPassphrase } from "../src/config.js";

test("networkPassphrase maps testnet and mainnet", () => {
  assert.match(networkPassphrase("testnet"), /Test SDF Network/);
  assert.match(networkPassphrase("mainnet"), /Public Global Stellar Network/);
});

test("secret key validation shape for Hello World flow", () => {
  const kp = Keypair.random();
  assert.equal(kp.publicKey().startsWith("G"), true);
  assert.equal(kp.secret().startsWith("S"), true);
});
