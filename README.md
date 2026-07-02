# Silver Mining Protocol

A decentralised prediction game on Solana. Every 30 seconds a round opens —
players stake SOL on one of five blocks, a pseudo-random function determines
the winner, and the winning block's stakers share 89% of the pot. Winning
bettors earn UNREFINED tokens scaled by mine level, which refine into SILVER
for staking yield.

**Live:** [silvermines.xyz](https://silvermines.xyz)  
**Program:** [`CiKNKPpdC55EpnVD5nDF5kSHVUHu1Q3kiKUstdsHPmtV`](https://solscan.io/account/CiKNKPpdC55EpnVD5nDF5kSHVUHu1Q3kiKUstdsHPmtV)
on Solscan  
**SILVER token:** [`ELXUqFZwPMmgQepCXSyq6YTRR3gXLWmB1Etr4g4stUHq`](https://solscan.io/token/ELXUqFZwPMmgQepCXSyq6YTRR3gXLWmB1Etr4g4stUHq)
on Solscan  
**Network:** Solana Mainnet 
---

## How it works

```
Bet SOL on 1–5 blocks
       │
       ├── Win → share 89% of losing bets + earn UNREFINED tokens
       │
       └── Lose → nothing returned; your SOL funds the winners

UNREFINED tokens
       │
       ├── Hold → unlock higher mine levels (2×–16× emission)
       │
       └── Refine → burn all UNREFINED, receive 90% as SILVER
                    (10% redistributed to other UNREFINED holders)

SILVER tokens
       │
       ├── Stake → earn 20% APR (lazily accrued, mint-based)
       │
       └── Accumulate → enter Motherlode jackpot raffle on winning rounds
```

**Motherlode jackpot:** 4% of every round pot accumulates. Every 100–1,000
rounds the jackpot locks and enters a 1-hour claim window. Winners from that
round enter a weighted-random raffle  larger bets give better odds, one
wallet takes all.

**AutoMiner:** pre-deposit SOL into a PDA. Any third-party cranker executes
your bet each round and receives the Bet account rent + a small tip.
Your SOL stays non-custodial — only you can withdraw it.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   silver_mining Anchor Program                  │
│             CiKNKPpdC55EpnVD5nDF5kSHVUHu1Q3kiKUstdsHPmtV       │
│                                                                 │
│  ┌─────────────┐  ┌──────────────────┐  ┌──────────────────┐   │
│  │ Round engine│  │   Token engine   │  │  AutoMiner crank │   │
│  │  26 instr.  │  │ UNREFINED/SILVER │  │ permissionless   │   │
│  └──────┬──────┘  └────────┬─────────┘  └──────┬───────────┘   │
│         └─────────────────┴──────────────────  │               │
│                       Config PDA ───────────────┘               │
│               (global state + lamport treasury)                 │
└───────────────────────────────────────────┬─────────────────────┘
                          CPI               │
              ┌───────────────────────────────────────────────┐
              │  SPL Token Program                            │
              │  SILVER mint (PDA)  ·  UNREFINED mint (PDA)  │
              └───────────────────────────────────────────────┘
```

**Accounts:** `Config` · `Round` · `Bet` · `Miner` · `Pool` · `AutoMiner`

All PDAs are canonical  derivable from known seeds without a lookup table.

---

## Stack

| Layer | Technology |
|-------|-----------|
| Program | Rust, Anchor 0.31.0, Solana 3.0.13 |
| Tokens | SPL Token  two PDA-controlled mints |
| Frontend | React 18, TypeScript, Vite, Tailwind CSS |
| Wallet | Solana Wallet Adapter (Phantom, Solflare) |
| State | Zustand |
| RPC | Manual instruction building  no Anchor client runtime |

---

## Design decisions

**No Anchor client at runtime.** Instructions are built manually from
discriminators and Borsh-encoded arguments. This removes ~400 KB from the
bundle, gives full control over compute budget instructions, and eliminates
IDL-version coupling. See [ADR-0003](docs/adr/0003-manual-instruction-building.md).

**Permissionless settlement.** Anyone can call `finalize_round` and
`crank_autominer`. Settlers receive 0.05 SOL from the warchest allocation.
Crankers receive Bet account rent plus a small tip. No round can be blocked
by a single party.

**Crank-based AutoMiner.** Users pre-deposit SOL into a PDA. Third-party
bots execute bets each round and are reimbursed from the AutoMiner balance.
The cranker can only trigger a bet — they cannot withdraw the balance.
See [ADR-0004](docs/adr/0004-crank-autominer-model.md).

**Two-token model.** UNREFINED is the high-emission participation reward.
SILVER is the refined store of value — lower supply, stakeable. The 10%
redistribution on refining creates passive yield for existing UNREFINED
holders without new SILVER inflation. See [ADR-0002](docs/adr/0002-two-token-model.md).

**Pseudo-random finalization.** Block outcomes use FNV-1a over
`slot + timestamp + round_number`. This is not a VRF — a validator producing
the finalization slot has partial influence over the outcome. At current bet
sizes the economic incentive to manipulate is negligible. This is documented
honestly, not hidden. See [ADR-0001](docs/adr/0001-pseudo-random-over-vrf.md)
and the [randomness threat model](docs/threat-model/randomness.md).

---

## Fee structure

| Destination | Share | Purpose |
|-------------|-------|---------|
| Winners | 89% | Proportional to stake on winning block |
| Warchest | 5% | Protocol operations + settler incentive |
| Motherlode | 4% | Progressive jackpot accumulation |
| Admin | 1% | Development |
| AutoMiner treasury | 1% | Cranker reimbursement pool |

---

## Documentation

```
docs/
├── specs/
│   └── v1.0.md              ← full protocol specification
├── adr/
│   ├── 0001-pseudo-random-over-vrf.md
│   ├── 0002-two-token-model.md
│   ├── 0003-manual-instruction-building.md
│   ├── 0004-crank-autominer-model.md
│   ├── 0005-config-pda-lamport-treasury.md
│   └── 0006-pool-all-blocks-constraint.md
└── threat-model/
    ├── randomness.md
    ├── admin-risk.md
    └── attack-matrix.md
```

---

## Security

**Not audited.** This protocol ran on Solana mainnet with real user funds
from February to June 2026. It has not been formally audited by a third party.

**Randomness.** Block outcomes are pseudo-random, not provably fair. See
the [randomness threat model](docs/threat-model/randomness.md) for the
full attack analysis and why it was accepted for this scale.

**Admin capabilities.** The authority key can pause the protocol, update
staking APR, and mint up to 50,000 SILVER (hard-capped on-chain). It cannot
drain user Round PDAs or AutoMiner balances. See
[admin risk](docs/threat-model/admin-risk.md).

**Non-custodial.** User SOL flows directly to Round PDAs and back to winners.
The program never holds user betting funds in a centrally controlled wallet.
AutoMiner deposits are withdrawable by the owner at any time.



---

## Local development

```bash
# Program
cd contracts
anchor build
anchor test

# Frontend
cd frontend
cp .env.example .env.local   # add your RPC endpoint
npm install
npm run dev
```

Requires: Rust stable, Solana CLI 3.0.13, Anchor CLI 0.31.0, Node 18+.


---

## Community

- Twitter: [@silversupplyo](https://x.com/silversupplyo?s=21)
- Discord: [discord.gg/9exmAHRHV](https://discord.gg/9exmAHRHV)
- Telegram: [t.me/silversupply](https://t.me/silversupply)

---

