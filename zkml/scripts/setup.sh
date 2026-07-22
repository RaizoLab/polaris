#!/usr/bin/env bash
# One-time circuit compilation and Groth16 trusted setup (BLS12-381).
#
# Requires the circom 2 compiler on PATH (cargo install --git
# https://github.com/iden3/circom.git circom). Outputs are committed to
# build/ so provers and CI only need snarkjs.
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p build

echo "--- compiling circuit (curve: BLS12-381) ---"
circom circuits/strategy.circom --r1cs --wasm --prime bls12381 -o build

echo "--- powers of tau (dev ceremony; NOT for production) ---"
npx snarkjs powersoftau new bls12-381 12 build/pot12_0000.ptau -v
npx snarkjs powersoftau contribute build/pot12_0000.ptau build/pot12_0001.ptau \
  --name="polaris-dev" -v -e="polaris dev entropy $(date +%s)"
npx snarkjs powersoftau prepare phase2 build/pot12_0001.ptau build/pot12_final.ptau -v

echo "--- groth16 setup + zkey ---"
npx snarkjs groth16 setup build/strategy.r1cs build/pot12_final.ptau build/strategy_0000.zkey
npx snarkjs zkey contribute build/strategy_0000.zkey build/strategy_final.zkey \
  --name="polaris-dev" -v -e="polaris zkey entropy $(date +%s)"
npx snarkjs zkey export verificationkey build/strategy_final.zkey build/verification_key.json

rm -f build/pot12_0000.ptau build/pot12_0001.ptau build/strategy_0000.zkey
echo "--- done: build/strategy_final.zkey + build/verification_key.json ---"
