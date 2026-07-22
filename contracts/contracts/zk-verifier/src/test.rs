#![cfg(test)]

extern crate std;

use super::test_vectors::*;
use super::{Groth16Verifier, Groth16VerifierClient, Proof, VerificationKey};
use soroban_sdk::{vec, BytesN, Env, Vec};

fn decode<const N: usize>(hex: &str) -> [u8; N] {
    assert_eq!(hex.len(), N * 2, "hex string length mismatch");
    let mut out = [0u8; N];
    let bytes = hex.as_bytes();
    for i in 0..N {
        let hi = (bytes[2 * i] as char).to_digit(16).unwrap() as u8;
        let lo = (bytes[2 * i + 1] as char).to_digit(16).unwrap() as u8;
        out[i] = (hi << 4) | lo;
    }
    out
}

fn g1(env: &Env, hex: &str) -> BytesN<96> {
    BytesN::from_array(env, &decode::<96>(hex))
}

fn g2(env: &Env, hex: &str) -> BytesN<192> {
    BytesN::from_array(env, &decode::<192>(hex))
}

fn scalar(env: &Env, hex: &str) -> BytesN<32> {
    BytesN::from_array(env, &decode::<32>(hex))
}

fn load_vk(env: &Env) -> VerificationKey {
    let mut ic: Vec<BytesN<96>> = vec![env];
    for point in VK_IC {
        ic.push_back(g1(env, point));
    }
    VerificationKey {
        alpha_neg: g1(env, VK_ALPHA_NEG),
        beta: g2(env, VK_BETA),
        gamma_neg: g2(env, VK_GAMMA_NEG),
        delta_neg: g2(env, VK_DELTA_NEG),
        ic,
    }
}

fn load_proof(env: &Env) -> Proof {
    Proof {
        a: g1(env, PROOF_A),
        b: g2(env, PROOF_B),
        c: g1(env, PROOF_C),
    }
}

fn load_inputs(env: &Env) -> Vec<BytesN<32>> {
    let mut inputs: Vec<BytesN<32>> = vec![env];
    for input in PUBLIC_INPUTS {
        inputs.push_back(scalar(env, input));
    }
    inputs
}

fn setup(env: &Env) -> Groth16VerifierClient<'_> {
    let contract_id = env.register(Groth16Verifier, ());
    let client = Groth16VerifierClient::new(env, &contract_id);
    client.set_vk(&load_vk(env));
    client
}

#[test]
fn valid_proof_verifies_true() {
    let env = Env::default();
    let client = setup(&env);

    let verified = client.verify_proof(&load_proof(&env), &load_inputs(&env));
    assert!(verified, "real snarkjs proof must verify");
}

#[test]
fn tampered_public_input_returns_false() {
    let env = Env::default();
    let client = setup(&env);

    // Claim a different trade amount (public input index 5) than was proven.
    let mut inputs = load_inputs(&env);
    let mut tampered = decode::<32>(PUBLIC_INPUTS[5]);
    tampered[31] ^= 0x01;
    inputs.set(5, BytesN::from_array(&env, &tampered));

    let verified = client.verify_proof(&load_proof(&env), &inputs);
    assert!(!verified, "proof must not verify for a different amount");
}

#[test]
fn mismatched_proof_points_return_false() {
    let env = Env::default();
    let client = setup(&env);

    // A and C are both valid G1 points; swapping them breaks the pairing.
    let proof = load_proof(&env);
    let swapped = Proof {
        a: proof.c.clone(),
        b: proof.b.clone(),
        c: proof.a.clone(),
    };

    let verified = client.verify_proof(&swapped, &load_inputs(&env));
    assert!(!verified, "swapped proof points must not verify");
}

#[test]
fn wrong_input_count_is_rejected() {
    let env = Env::default();
    let client = setup(&env);

    let mut inputs = load_inputs(&env);
    inputs.pop_back();

    let result = client.try_verify_proof(&load_proof(&env), &inputs);
    assert!(result.is_err(), "input count must match the circuit");
}

#[test]
fn vk_can_only_be_set_once() {
    let env = Env::default();
    let client = setup(&env);

    let result = client.try_set_vk(&load_vk(&env));
    assert!(result.is_err(), "second set_vk must fail");
}

#[test]
fn verification_stays_within_soroban_budget() {
    // Env::default() enforces the network CPU budget (100M instructions);
    // exceeding it would trap. Also print the actual cost for visibility.
    let env = Env::default();
    let client = setup(&env);

    env.cost_estimate().budget().reset_default();
    let verified = client.verify_proof(&load_proof(&env), &load_inputs(&env));
    assert!(verified);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();
    std::println!("groth16 verify cost: cpu={cpu} mem={mem}");
    assert!(
        cpu < 100_000_000,
        "verification must fit the Soroban tx budget, used {cpu}"
    );
}
