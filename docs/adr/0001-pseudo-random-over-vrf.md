# ADR-0001: Pseudo-random finalization over a VRF

**Status:** Accepted  
**Date:** 2026-07  
**Author:** Saba Shahzadi

---

## Context

The protocol needs a source of randomness to determine the winning block
each round. Two credible options exist on Solana:

**Option A — Pseudo-random (FNV hash of on-chain data)**
Combine `clock.slot`, `clock.unix_timestamp`, and `round_number` as
entropy inputs, apply FNV-1a hashing with avalanche mixing, and use
the result directly. Deterministic given the inputs; no external
dependency.

**Option B — Verifiable Random Function (VRF)**
Use Switchboard VRF or a similar oracle service. The VRF produces a
cryptographically provable random output that cannot be predicted or
manipulated by any single party, including the requesting program.

---

## Decision

Use pseudo-random (Option A).

---

## Rationale

**The round duration makes VRF mechanically incompatible.**

Switchboard VRF operates on a request-then-fulfil model:

```
Transaction 1: request_randomness (pays oracle fee, records commitment)
    ↓  [oracle off-chain computation — typically 1–3 slots]
Transaction 2: oracle fulfils request, callback fires
```

With 30-second rounds, the finalization window opens the moment
`clock.unix_timestamp >= end_time`. A VRF fulfil transaction arriving
in the next slot is fine in theory. In practice:

- The oracle fulfil transaction must land in the same block or the
  next block as the finalization. Network congestion on Solana
  makes this non-deterministic.
- If the fulfil fails (oracle offline, congestion), the round is
  stuck waiting for randomness. The protocol has no fallback path.
- Each VRF request costs oracle fees (~0.01–0.1 SOL per round).
  At 2,880 rounds per day this is economically unsustainable.

**The attack cost makes pseudo-random acceptable at current scale.**

The entity with influence over the random output is a validator
scheduled to produce the finalization slot. Their options:

1. Observe the block-in-progress and calculate `winning_block`.
2. If the outcome is unfavourable, delay or skip producing the block.
3. The next validator produces the block — with a different slot and
   timestamp, producing a different outcome.

This attack requires:
- Being scheduled to produce the specific finalization slot.
- Being willing to sacrifice the block reward (~0.5 SOL).
- The expected profit from the manipulation exceeding 0.5 SOL.

At typical bet sizes of 0.01–0.1 SOL per block and 5–50 concurrent
bettors, the average round pot is 0.05–2.5 SOL. The winning pot
is 89% of the losing side's bets — typically 0.02–1.1 SOL. A validator
who skips a block to re-roll randomness sacrifices ~0.5 SOL for an
expected gain of ~0.5–1 SOL on a single round. The expected value of
the attack is marginal at best.

---

## Consequences

- Round outcomes are not provably fair. Players must trust that validators
  are not manipulating outcomes.
- The limitation is disclosed in the protocol spec (§14), the threat model
  (`docs/threat-model/randomness.md`), and the README `Security` section.
- The word "VRF" does not appear anywhere in the program code or marketing
  materials. All references use "pseudo-random" or "hash-based randomness."
- If average round pots grow to > 5 SOL, the attack becomes economically
  rational for a validator. At that scale, migrating to a two-transaction
  VRF model (or increasing round duration to 120–300 seconds to accommodate
  VRF latency) should be evaluated. This is a v2 decision point.