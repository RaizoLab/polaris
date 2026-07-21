/**
 * Background relayer service: an async signal queue the Brain pushes into,
 * plus a run loop that relays each signal in order and reports results.
 */

import type { RebalanceSignal, RelayResult } from "./types.js";
import type { TransactionRelayer } from "./relayer.js";

/** Unbounded in-memory queue bridging the Brain (producer) and relayer (consumer). */
export class SignalQueue implements AsyncIterable<RebalanceSignal> {
  private buffer: RebalanceSignal[] = [];
  private waiters: Array<(value: IteratorResult<RebalanceSignal>) => void> = [];
  private closed = false;

  push(signal: RebalanceSignal): void {
    if (this.closed) throw new Error("SignalQueue is closed");
    const waiter = this.waiters.shift();
    if (waiter) {
      waiter({ value: signal, done: false });
    } else {
      this.buffer.push(signal);
    }
  }

  close(): void {
    this.closed = true;
    for (const waiter of this.waiters.splice(0)) {
      waiter({ value: undefined, done: true });
    }
  }

  [Symbol.asyncIterator](): AsyncIterator<RebalanceSignal> {
    return {
      next: (): Promise<IteratorResult<RebalanceSignal>> => {
        const buffered = this.buffer.shift();
        if (buffered !== undefined) {
          return Promise.resolve({ value: buffered, done: false });
        }
        if (this.closed) {
          return Promise.resolve({ value: undefined, done: true });
        }
        return new Promise((resolve) => this.waiters.push(resolve));
      },
    };
  }
}

/** Relay every signal from the queue until it is closed. */
export async function runRelayerService(
  relayer: TransactionRelayer,
  signals: AsyncIterable<RebalanceSignal>,
  onResult: (result: RelayResult) => void = () => {}
): Promise<RelayResult[]> {
  const results: RelayResult[] = [];
  for await (const signal of signals) {
    const result = await relayer.submitSignal(signal);
    results.push(result);
    onResult(result);
  }
  return results;
}
