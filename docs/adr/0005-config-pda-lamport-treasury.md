# ADR-0005: Config PDA as lamport treasury

**Status:** Accepted  
**Date:** 2026-07  
**Author:** Fiza

---

## Context

The Motherlode jackpot and AutoMiner treasury need to hold SOL.
Two structural options:

**Option A — Separate vault PDAs**
Create a `MotherlodeVault` PDA and an `AutoMinerTreasury` PDA.
Each holds lamports. Instructions transfer in and out explicitly.

**Option B — Config PDA holds lamports above rent minimum**
Track the treasury balances as fields in `Config`. The Config PDA
lamport balance = rent_exempt_minimum + motherlode_balance +
motherlode_prize + autominer_treasury.

---

## Decision

Use Config PDA as lamport treasury (Option B).

---

## Rationale

**Fewer accounts per instruction.** Every instruction that touches
the Motherlode or AutoMiner treasury would need to pass the
corresponding vault account. Solana's account limit per transaction
(64 accounts) becomes relevant for complex instructions. Using the
Config PDA — which is already required by almost every instruction —
avoids adding two more accounts everywhere.

**Simpler account initialisation.** No additional PDA accounts need
to be created during protocol setup. The Config PDA is initialised
once with enough lamports to be rent-exempt, and the treasury
accumulates naturally as fees flow in.

---

## Consequences

- The Config PDA's lamport balance must always be >= rent_exempt_minimum
  + all tracked treasury balances. Any instruction that withdraws
  must verify this invariant is maintained.
- On-chain inspection is less obvious: to see the Motherlode balance,
  a user must read the `Config` account data, not just check a
  dedicated vault account's lamport balance. The frontend handles
  this transparently.
- A bug that allows lamports to leave the Config PDA without updating
  the tracking fields would create an undetectable desync. The
  protocol mitigates this by always updating fields and lamports in
  the same instruction, never separately.

---
---

