# ADR-0003: Manual instruction building over Anchor client on frontend

**Status:** Accepted  
**Date:** 2026-07  
**Author:** Saba Shahzadi

---

## Context

The frontend needs to interact with the Anchor program. Two approaches:

**Option A — Anchor client (`@coral-xyz/anchor`)**
Import the generated IDL, construct an `AnchorProvider` and `Program`
instance, call typed methods. The Anchor client handles discriminator
encoding, argument serialisation, and account resolution.

**Option B — Manual instruction building**
Import only `@solana/web3.js` and `@solana/spl-token`. Derive PDAs
manually. Encode discriminators and arguments with Borsh. Build
`TransactionInstruction` objects directly.

---

## Decision

Use manual instruction building (Option B).

---

## Rationale

**Bundle size.** `@coral-xyz/anchor` adds ~400KB to the bundle.
The frontend is a game with a real-time countdown — load time matters.
Manual instruction building requires only `@solana/web3.js` (already
required) and a small Borsh utility.

**Full transaction control.** The Anchor client abstracts transaction
construction in ways that make it difficult to add compute budget
instructions, set priority fees, or batch multiple instructions in a
single transaction. Manual building gives full control over the
`ComputeBudgetProgram.setComputeUnitLimit` and priority fee instructions
that Solana requires for reliable confirmation under load.

**No IDL version coupling.** The Anchor client expects the IDL to match
the deployed program exactly. During development, IDL mismatches
produce cryptic errors. Manual discriminator constants (defined in
`utils/idl.ts`) are explicit and do not change unless we explicitly
update them.

**Discriminators are stable once deployed.** Anchor discriminators are
`sha256("global:<instruction_name>")[0..8]`. Once the program is
deployed, these are permanent. Hardcoding them in `DISCRIMINATORS` is
safe and explicit.

---

## Consequences

- All discriminators and account layouts must be manually maintained
  in `utils/idl.ts`. A program upgrade that adds or changes an
  instruction requires manual updates to this file.
- Error decoding requires a manual error code map (implemented in
  the `parseError` function in `useProgram.ts`).
- PDA derivation is explicit — any bug in a seed ordering is caught
  immediately as a transaction failure rather than silently using
  the wrong account.

---





