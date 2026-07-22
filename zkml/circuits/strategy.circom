pragma circom 2.0.0;

include "../node_modules/circomlib/circuits/comparators.circom";

/*
 * Polaris strategy proof.
 *
 * Proves that a trade emitted by the agent followed the open-source
 * strategy, without trusting the agent:
 *
 *   1. The chosen target protocol has the maximum expected APY among the
 *      candidate protocols scored by the model.
 *   2. The trade amount respects the max-allocation cap:
 *      amount * 10000 <= vault_total * MAX_WEIGHT_BPS.
 *
 * Public signals (visible to the on-chain verifier):
 *   - apys[N]     model-scored APYs per candidate protocol, in basis points
 *   - chosen      index of the intended target protocol
 *   - amount      trade amount (stroops)
 *   - vaultTotal  total vault balance (stroops)
 *
 * Private witness:
 *   - selector[N] one-hot encoding of the chosen protocol
 */
template StrategyCheck(N, MAX_WEIGHT_BPS) {
    signal input apys[N];
    signal input chosen;
    signal input amount;
    signal input vaultTotal;
    signal input selector[N];

    // --- selector is one-hot and consistent with `chosen` ---
    var selSum = 0;
    var chosenAcc = 0;
    for (var i = 0; i < N; i++) {
        selector[i] * (selector[i] - 1) === 0; // each flag is 0 or 1
        selSum += selector[i];
        chosenAcc += selector[i] * i;
    }
    selSum === 1;
    chosen === chosenAcc;

    // --- chosen APY is the maximum ---
    // Dot product selector . apys via intermediate signals (each constraint
    // must be a single quadratic form in R1CS).
    signal partial[N];
    partial[0] <== selector[0] * apys[0];
    for (var i = 1; i < N; i++) {
        partial[i] <== partial[i - 1] + selector[i] * apys[i];
    }
    signal chosenApy;
    chosenApy <== partial[N - 1];

    component geq[N];
    for (var i = 0; i < N; i++) {
        geq[i] = GreaterEqThan(32); // APYs are < 2^32 bps
        geq[i].in[0] <== chosenApy;
        geq[i].in[1] <== apys[i];
        geq[i].out === 1;
    }

    // --- allocation cap: amount / vaultTotal <= MAX_WEIGHT_BPS / 10000 ---
    component cap = LessEqThan(96); // stroop amounts fit well below 2^96
    cap.in[0] <== amount * 10000;
    cap.in[1] <== vaultTotal * MAX_WEIGHT_BPS;
    cap.out === 1;
}

component main {public [apys, chosen, amount, vaultTotal]} = StrategyCheck(4, 6000);
