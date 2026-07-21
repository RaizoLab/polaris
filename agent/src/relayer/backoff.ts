/** Exponential backoff with jitter for transaction retries. */

export interface BackoffOptions {
  baseDelayMs?: number;
  maxDelayMs?: number;
}

export const DEFAULT_BACKOFF: Required<BackoffOptions> = {
  baseDelayMs: 1_000,
  maxDelayMs: 30_000,
};

/**
 * Delay before retry number `attempt` (0-based): base * 2^attempt, capped at
 * maxDelayMs, with 50-100% jitter to avoid thundering-herd resubmission.
 */
export function computeBackoffMs(
  attempt: number,
  options: BackoffOptions = {},
  random: () => number = Math.random
): number {
  const { baseDelayMs, maxDelayMs } = { ...DEFAULT_BACKOFF, ...options };
  const exponential = Math.min(maxDelayMs, baseDelayMs * 2 ** attempt);
  return Math.floor(exponential * (0.5 + random() * 0.5));
}
