# Threat Model: Admin Risk

**Document:** `docs/threat-model/admin-risk.md`  
**Version:** 1.0 · 2026-07

---

## The authority key

The `authority` field in `Config` is set at `initialize` and controls
all admin instructions. It is a standard Solana keypair — not a
multisig, not a timelock, not a DAO.

---

## What the authority CAN do

| Capability | Instruction | Impact |
|-----------|-------------|--------|
| Pause all betting and finalization | `pause` | No new rounds, no new bets |
| Unpause the protocol | `unpause` | Resumes normal operation |
| Change staking APR | `update_staking_apr` | Affects future rewards only |
| Mint up to 50,000 SILVER | `admin_mint_silver` | Bounded on-chain; cap enforced |
| Reset a stuck Motherlode | `admin_reset_motherlode` | Returns prize to jackpot pool |
| Withdraw Motherlode fees | `withdraw_motherlode_fees` | Withdraws `motherlode_balance` only |
| Withdraw AutoMiner treasury | `withdraw_autominer_treasury` | Withdraws AutoMiner fee pool |
| Create token metadata | `create_token_metadata` | One-time metadata setup |

---

## What the authority CANNOT do

| Action | Why not |
|--------|---------|
| Drain Round PDAs (user bets) | Round PDAs are owned by the System Program. The authority has no signing authority over them. |
| Drain AutoMiner PDAs | Same — user AutoMiner accounts are independent PDAs. |
| Drain Miner accounts | Miner accounts hold no SOL beyond rent. |
| Change who can claim a bet | Claim instructions validate the claimer against the Bet's `miner` field. |
| Redirect Motherlode winnings | `collect_motherlode` validates `winner.key() == config.motherlode_winner_key`. |
| Mint SILVER beyond 50,000 | `admin_minted_silver` is checked against `MAX_ADMIN_MINT_SILVER` on every call. |
| Mint UNREFINED | There is no admin UNREFINED mint instruction. UNREFINED is only minted via `claim_bet_silver` and `claim_silver`. |

---

## Honest risk assessment

**A compromised authority key can:**
- Pause the protocol indefinitely, preventing new rounds and claims.
- Drain the `motherlode_balance` and `autominer_treasury` (both are
  under admin withdrawal instructions).
- Mint up to 50,000 additional SILVER, diluting existing holders.

**The maximum financial impact of a full authority key compromise:**
- `motherlode_balance` (variable — depends on accumulated jackpot)
- `autominer_treasury` (variable — depends on fee accumulation)
- SILVER dilution from 50,000 admin mint

**User SOL in active rounds and AutoMiner deposits is not reachable
by the authority key.** The core non-custodial property holds even
under authority compromise.

---

## v2 mitigations

The following are not implemented in v1 but are documented for v2:

- **Multisig authority:** Replace the single keypair with a 2-of-3
  or 3-of-5 multisig using Squads or a native Solana multisig.
- **Timelock on parameter changes:** Add a delay between proposing
  and executing APR changes or fee withdrawals.
- **Fee withdrawal cap:** Limit per-transaction withdrawal from
  `motherlode_balance` and `autominer_treasury`.