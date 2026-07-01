# Threat Model: Randomness

**Document:** `docs/threat-model/randomness.md`  
**Version:** 1.0 · 2026-07

---

## Overview

The Silver Mining Protocol uses pseudo-randomness to determine the
winning block each round. This document describes the randomness model,
the attack surface it creates, and the honest assessment of the risk
at current protocol scale.

---

## How randomness is generated

```rust
fn generate_random(slot: u64, timestamp: i64, round: u64, seed: &[u8]) -> u64 {
    // 1. Collect entropy inputs into a byte buffer
    // 2. Apply FNV-1a hash (offset basis: 0xcbf29ce484222325)
    // 3. Apply three rounds of XOR-shift-multiply (avalanche mixing)
    // 4. Return u64
}
```

Called three times in `finalize_round`:

| Call | Seed | Determines |
|------|------|-----------|
| 1 | `b"winning_block"` | Winning block (output mod 5) |
| 2 | `b"solo_split"` | Solo or proportional UNREFINED distribution (output mod 2) |
| 3 | `b"solo_seed"` | Entropy for solo raffle score calculation |

---

## Entropy inputs and their predictability

| Input | Who knows it before finalization? |
|-------|----------------------------------|
| `clock.slot` | Everyone — slot number is public |
| `clock.unix_timestamp` | Everyone — deterministic from slot leader schedule |
| `round.round_number` | Everyone — sequential, fully known |
| `seed` string | Everyone — hardcoded in the program |

**All inputs are publicly knowable before finalization.** The output
is not secret it is deterministic from public inputs. Anyone with
the slot number and timestamp can compute the winning block before
submitting `finalize_round`.

---

## The attacker: a scheduled block producer

The only entity with meaningful leverage is a validator scheduled to
produce the finalization slot. Their capabilities:

1. **Pre-compute the outcome:** knowing the slot, timestamp, and round
   number, they can compute `winning_block` before the block is finalised.
2. **Decide not to produce the block:** if the outcome is unfavourable
   (e.g., they have a large bet on the losing block), they can skip
   block production. The next validator produces the block with a
   different slot and timestamp, changing the outcome.
3. **Time a large bet:** a validator with advance knowledge of the
   outcome can place a last-second bet on the winning block.

---

## Attack cost analysis

### Skipping block production

| Variable | Value |
|----------|-------|
| Block reward (approximate) | ~0.5 SOL |
| Probability of being scheduled for a specific slot | Very low (~1 in 1,500 validators) |
| Expected gain from manipulation | (bet_on_losing_block − expected_loss) |

A validator skipping a block sacrifices ~0.5 SOL in block reward.
For this to be rational, the expected gain from changing the outcome
must exceed 0.5 SOL.

At typical round pots of 0.1–2 SOL and winner share of 89%, the
maximum gain from manipulating a single round is approximately 0.9 SOL.
The expected gain is lower because the validator must already have
a bet placed on the block they want to win, at the moment they
discover the outcome is wrong.

**At current scale, the attack is marginally profitable at best.**

### Last-second bet with foreknowledge

More practical: a validator or someone monitoring the slot leader
schedule places a large bet on the winning block in the final seconds
of the round, after computing the deterministic outcome.

This does not require skipping a block. It only requires:
1. Knowing the slot leader schedule (public).
2. Computing the hash before submitting the bet.
3. Having enough SOL to place a meaningful bet.

**This attack is possible and unmitigated in v1.**

The mitigation would be a commit-reveal scheme or VRF, both of which
are incompatible with 30-second rounds at current scale. See ADR-0001.

---

## Residual risk disclosure

The following attack vectors are **not mitigated** in v1:

| Vector | Attacker | Cost | Mitigation |
|--------|----------|------|-----------|
| Pre-compute winning block, place large last-second bet | Any player with access to slot leader schedule | SOL bet size only | None in v1 |
| Skip block production to re-roll outcome | Scheduled validator | ~0.5 SOL block reward | None in v1 |
| Front-run finalize_round with a favourable bet | Any player watching mempool | Transaction fee only | None in v1 |

---

## Risk assessment at current scale

**Low risk** — the economic incentive for manipulation is low relative
to the cost at current average round pots. The protocol is appropriate
for recreational betting at the SOL amounts typical of current usage.

**Risk increases with pot size.** If average round pots grow to > 5 SOL
consistently, the last-second foreknowledge attack becomes reliably
profitable with no capital risk (only the bet amount, which wins).
At that scale, a VRF migration or round duration increase should be
evaluated.

---

## User disclosure

Users are informed of this limitation in:
- `docs/specs/v1.0.md` §14
- `docs/threat-model/randomness.md` (this document)
- `SECURITY.md`
- The README `Security` section

The word "VRF" does not appear in the program, the frontend, or any
marketing material for v1.