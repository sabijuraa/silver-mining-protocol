# Threat Model: Attack Matrix

**Document:** `docs/threat-model/attack-matrix.md`  
**Version:** 1.0 · 2026-07

---

## Summary

Every known attack vector against the Silver Mining Protocol, with
status (mitigated / unmitigated / partially mitigated), the
mitigation mechanism, and the residual risk.

---

## Attack Matrix

| # | Attack | Actor | Status | Mitigation | Residual Risk |
|---|--------|-------|--------|-----------|--------------|
| A1 | Pre-compute winning block using slot + timestamp | Any player | **Unmitigated** | No VRF in v1 (ADR-0001) | Any player with the slot leader schedule can compute the outcome before placing a bet. Low impact at current pot sizes. |
| A2 | Validator skips block to re-roll unfavourable outcome | Scheduled validator | **Partially mitigated** | Block skip costs ~0.5 SOL block reward | Economically marginal at current scale; becomes rational if pot > ~5 SOL. |
| A3 | Double-claim SOL rewards | Winner | **Mitigated** | `bet.claimed` flag; Anchor `constraint = !bet.claimed` | None. Flag set atomically with transfer. |
| A4 | Double-claim UNREFINED rewards | Winner | **Mitigated** | `bet.silver_claimed` flag; separate from SOL claim | None. Independent flag, same atomic pattern. |
| A5 | Claim SOL from a round that is not finalized | Any bettor | **Mitigated** | `require!(round.finalized)` guard in `claim_sol` | None. |
| A6 | Place bet after round end_time | Any bettor | **Mitigated** | `require!(clock.unix_timestamp < round.end_time)` | None. |
| A7 | Finalize a round before it ends | Anyone | **Mitigated** | `require!(clock.unix_timestamp >= round.end_time)` | None. |
| A8 | Double-finalize a round | Anyone | **Mitigated** | `require!(!round.finalized)` | None. |
| A9 | Reentrancy via CPI callback | Any contract | **Mitigated** | Solana account model prevents reentrancy; no CPI callbacks in instruction path | None. Solana's execution model does not support reentrancy the way EVM does. |
| A10 | Overflow in fee calculation | N/A (input-triggered) | **Mitigated** | All arithmetic uses `.checked_*` with `Overflow` error | None. Every arithmetic path is checked. |
| A11 | Authority key compromise | External attacker | **Partially mitigated** | Authority cannot touch user Round PDAs or AutoMiner balances | Attacker can pause protocol, drain motherlode_balance and autominer_treasury, admin-mint up to 50k SILVER. |
| A12 | False PDA passed for WARCHEST or ADMIN wallet | Malicious caller | **Mitigated** | Anchor `address = WARCHEST_WALLET.parse::<Pubkey>().unwrap()` constraint | None. Address is verified at deserialization. |
| A13 | Crank AutoMiner without authorization | Random cranker | **By design** | Cranking is permissionless. Cranker can only place a bet at `sol_per_block`, not withdraw. | AutoMiner funds cannot be extracted via crank. |
| A14 | Drain AutoMiner balance via unauthorized withdrawal | Third party | **Mitigated** | `autominer.owner == owner.key()` constraint on `withdraw_autominer` | None. Owner check enforced by Anchor. |
| A15 | Motherlode winner manipulation — claim with losing bet | Non-winner | **Mitigated** | `claim_sol` requires `bet.blocks[winning_block] == true` | None. |
| A16 | Motherlode collect before claim window closes | Anyone | **Mitigated** | `require!(clock.unix_timestamp >= round.end_time + MOTHERLODE_CLAIM_WINDOW)` | None. 1-hour window enforced. |
| A17 | Collect Motherlode for wrong winner | Anyone | **Mitigated** | `require!(ctx.accounts.winner.key() == config.motherlode_winner_key)` | None. Winner key validated. |
| A18 | Grief the protocol by never calling initialize_round | Anyone | **Partially mitigated** | Anyone can call initialize_round (permissionless) | If no one calls it, the round doesn't start. The frontend automatically calls it. If the frontend is down, a manual transaction is needed. |
| A19 | Admin mint beyond 50,000 SILVER | Authority | **Mitigated** | `config.admin_minted_silver + amount <= MAX_ADMIN_MINT_SILVER` checked on every call | None. Cap is permanent and on-chain. |
| A20 | Self-liquidation via pool (claim pool fee from own bet) | Pool creator | **Unmitigated** | Pool fee is an off-chain social agreement; not enforced on-chain | Low risk. Pool fee BPS is transparent and members choose to join. |
| A21 | SPL token mint authority takeover | Program upgrade | **Mitigated** | Both mints use Config PDA as authority. Config PDA authority requires the `authority` signer + correct PDA seeds. | Authority key compromise allows admin mint (bounded by cap). |
| A22 | Front-run finalize_round to place a winning bet | Anyone watching mempool | **Unmitigated** | Solana has no mempool in the traditional sense, but block leaders can see pending txns | Partially mitigated by Solana's architecture. The round end_time gate limits the window. |

---

## Risk summary

**Critical (would cause direct user fund loss):** None identified.
The non-custodial design ensures user SOL in Round PDAs is only
accessible via the claim instructions, which validate ownership.

**High (significant financial impact):** A1 (foreknowledge) if pot
sizes grow to > 5 SOL per round consistently. A11 (authority
compromise) for the motherlode and autominer treasury balances.

**Medium (protocol disruption):** A18 (round griefing) — low
probability given frontend automation and permissionless crankers.

**Low (at current scale):** A2 (validator manipulation) — marginal
economics.

---

## Not in scope

- Social engineering attacks on the authority key holder.
- Phishing attacks on users (frontend impersonation).
- Solana runtime bugs or validator consensus failures.
- Smart contract bugs in `@solana/spl-token` or the Anchor framework.