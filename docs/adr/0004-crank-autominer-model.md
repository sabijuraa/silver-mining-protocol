# ADR-0004: Crank-based AutoMiner over on-chain scheduler

**Status:** Accepted  
**Date:** 2026-07  
**Author:** Saba Shahzadi

---

## Context

Players want automated betting without staying online. Options:

**Option A — On-chain timer / scheduler**
A dedicated scheduler program executes bets at fixed intervals.
Solana does not have a native scheduler — this would require a
third-party protocol (e.g., Clockwork, now deprecated).

**Option B — Off-chain cron + client key**
A centralised server holds a delegated signing key and submits
bets on behalf of users. Simple to implement; fully custodial.

**Option C — Permissionless crank**
Users pre-deposit SOL into a PDA. Anyone may call `crank_autominer`
to execute a bet, receiving the Bet account rent + a small tip
from the AutoMiner balance as reimbursement.

---

## Decision

Use permissionless crank (Option C).

---

## Rationale

**Non-custodial by design.** The cranker receives a reimbursement
but never touches the user's principal balance. The cranker cannot
withdraw AutoMiner funds — only the owner can do that. The crank
only places a bet at the configured `sol_per_block` amount.

**Permissionless means no single point of failure.** Any bot
operator, MEV searcher, or Solana node running a simple polling
loop can act as a cranker. If one cranker goes offline, another
fills the gap. This is the same model used by Chainlink keepers,
Euler liquidation bots, and many other DeFi automation systems.

**Economically self-sustaining.** The cranker earns:
```
crank_revenue = rent.minimum_balance(Bet::SIZE) + CRANK_INCENTIVE
             ≈ ~0.00151 SOL + 0.00001 SOL per round
```
At 2,880 rounds per day and 50 AutoMiners, a cranker earns
approximately 0.22 SOL per day. This covers server costs for
a lightweight bot.

---

## Consequences

- If no cranker is running, AutoMiner bets are not placed.
  Users are notified in the frontend that AutoMiner requires
  an active cranker network.
- The `auto_reload` field is reserved for a future feature where
  AutoMiner winnings are automatically re-deposited to the balance.
  This is not implemented in v1.
- `daily_withdrawn` rate limiting is tracked but the enforcement
  logic is not fully implemented in v1 (the field is written but
  not read as a withdrawal gate). This is a v2 hardening.

---