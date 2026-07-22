//! Groth16 verifier on Soroban (BLS12-381).
//!
//! Verifies zero-knowledge proofs that the Polaris agent's trade followed
//! the open-source strategy (see zkml/circuits/strategy.circom) before a
//! rebalance is allowed. Uses the protocol-22 BLS12-381 host functions, so
//! all heavy pairing arithmetic runs in the host at calibrated cost, keeping
//! the verification inside the Soroban instruction budget.
//!
//! The canonical Groth16 check
//!     e(A, B) = e(alpha, beta) * e(vk_x, gamma) * e(C, delta)
//! is verified as a single product of pairings
//!     e(A, B) * e(-alpha, beta) * e(vk_x, -gamma) * e(C, -delta) == 1
//! where -alpha, -gamma, and -delta are negated off-chain in the exported
//! verification key (zkml/scripts/export_soroban.js), so the contract only
//! performs one MSM and one pairing check.

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    crypto::bls12_381::{Fr, G1Affine, G2Affine},
    vec, BytesN, Env, Vec,
};

#[contracttype]
#[derive(Clone)]
pub struct VerificationKey {
    /// -alpha in G1 (negated off-chain).
    pub alpha_neg: BytesN<96>,
    /// beta in G2.
    pub beta: BytesN<192>,
    /// -gamma in G2 (negated off-chain).
    pub gamma_neg: BytesN<192>,
    /// -delta in G2 (negated off-chain).
    pub delta_neg: BytesN<192>,
    /// IC points (one more than the number of public inputs).
    pub ic: Vec<BytesN<96>>,
}

#[contracttype]
#[derive(Clone)]
pub struct Proof {
    pub a: BytesN<96>,
    pub b: BytesN<192>,
    pub c: BytesN<96>,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Vk,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    VkNotSet = 1,
    VkAlreadySet = 2,
    InputLengthMismatch = 3,
}

#[contract]
pub struct Groth16Verifier;

#[contractimpl]
impl Groth16Verifier {
    /// Store the circuit verification key (one-time initialization).
    pub fn set_vk(env: Env, vk: VerificationKey) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Vk) {
            return Err(Error::VkAlreadySet);
        }
        env.storage().instance().set(&DataKey::Vk, &vk);
        Ok(())
    }

    /// Verify a Groth16 proof against the stored verification key.
    ///
    /// `public_inputs` are the circuit's public signals as 32-byte big-endian
    /// scalars — for the strategy circuit: the candidate APYs, the intended
    /// target protocol index, the trade amount, and the vault total.
    ///
    /// Returns `true` for valid proofs and `false` for invalid ones.
    pub fn verify_proof(
        env: Env,
        proof: Proof,
        public_inputs: Vec<BytesN<32>>,
    ) -> Result<bool, Error> {
        let vk: VerificationKey = env
            .storage()
            .instance()
            .get(&DataKey::Vk)
            .ok_or(Error::VkNotSet)?;
        verify(&env, &vk, &proof, &public_inputs)
    }
}

/// Core Groth16 verification: one MSM + one 4-way pairing check.
pub fn verify(
    env: &Env,
    vk: &VerificationKey,
    proof: &Proof,
    public_inputs: &Vec<BytesN<32>>,
) -> Result<bool, Error> {
    if vk.ic.len() != public_inputs.len() + 1 {
        return Err(Error::InputLengthMismatch);
    }

    let bls = env.crypto().bls12_381();

    // vk_x = IC[0] + sum_i(input_i * IC[i+1]) as one multi-scalar mul.
    let mut points: Vec<G1Affine> = vec![env];
    let mut scalars: Vec<Fr> = vec![env];
    points.push_back(G1Affine::from_bytes(vk.ic.get_unchecked(0)));
    scalars.push_back(Fr::from_u256(soroban_sdk::U256::from_u32(env, 1)));
    for (i, input) in public_inputs.iter().enumerate() {
        points.push_back(G1Affine::from_bytes(vk.ic.get_unchecked(i as u32 + 1)));
        scalars.push_back(Fr::from_bytes(input));
    }
    let vk_x = bls.g1_msm(points, scalars);

    // e(A,B) * e(-alpha,beta) * e(vk_x,-gamma) * e(C,-delta) == 1
    let g1_points: Vec<G1Affine> = vec![
        env,
        G1Affine::from_bytes(proof.a.clone()),
        G1Affine::from_bytes(vk.alpha_neg.clone()),
        vk_x,
        G1Affine::from_bytes(proof.c.clone()),
    ];
    let g2_points: Vec<G2Affine> = vec![
        env,
        G2Affine::from_bytes(proof.b.clone()),
        G2Affine::from_bytes(vk.beta.clone()),
        G2Affine::from_bytes(vk.gamma_neg.clone()),
        G2Affine::from_bytes(vk.delta_neg.clone()),
    ];

    Ok(bls.pairing_check(g1_points, g2_points))
}

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_vectors;
