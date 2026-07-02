use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Burn, MintTo, Transfer};

declare_id!("CiKNKPpdC55EpnVD5nDF5kSHVUHu1Q3kiKUstdsHPmtV");

// ============================================================================
// CONSTANTS
// ============================================================================

pub const ROUND_DURATION: i64 = 30;
pub const SETTLER_INCENTIVE: u64 = 50_000_000;
pub const MOTHERLODE_MIN: u64 = 100;
pub const MOTHERLODE_MAX: u64 = 1000;
pub const DEFAULT_STAKING_APR: u16 = 2000;
pub const MAX_POOL_MEMBERS: usize = 25;
pub const FEE_WARCHEST_BPS: u64 = 500;
pub const FEE_MOTHERLODE_BPS: u64 = 400;
pub const FEE_ADMIN_BPS: u64 = 100;
pub const FEE_AUTOMINER_BPS: u64 = 100;
pub const WINNER_SHARE_BPS: u64 = 8900;
pub const CRANK_INCENTIVE: u64 = 10_000;
pub const MOTHERLODE_CLAIM_WINDOW: i64 = 3600;
pub const MOTHERLODE_EXPIRY_ROUNDS: u64 = 10_000;
pub const MAX_ADMIN_MINT_SILVER: u64 = 50_000_000_000_000;
pub const ADMIN_WALLET: &str = "G1MfDRETA6zuCSHV7vkB82HFL5XCz2Pb9rKy1JZ9PCmk";
pub const WARCHEST_WALLET: &str = "2YaT2cNFDHTg8YcjGbjgyUFDLYSdKk4fh9qY9Q2YMkg3";
pub const STAKING_FUNDER: &str = "AJh7Vgc1osD9DPzDfiKWbysMpGfbSJFXeU85616c8tBV";
pub const METADATA_PROGRAM_ID: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
pub const EMISSIONS: [u64; 5] = [1_000_000_000, 2_000_000_000, 4_000_000_000, 8_000_000_000, 16_000_000_000];
pub const THRESHOLDS: [u64; 5] = [0, 15_000_000_000, 50_000_000_000, 100_000_000_000, 200_000_000_000];

// ============================================================================
// ERRORS
// ============================================================================

#[error_code]
pub enum SilverMiningError {
    #[msg("Unauthorized")] Unauthorized,
    #[msg("Invalid amount")] InvalidAmount,
    #[msg("Insufficient balance")] InsufficientBalance,
    #[msg("Insufficient stake")] InsufficientStake,
    #[msg("Round not finalized")] RoundNotFinalized,
    #[msg("Round already finalized")] RoundAlreadyFinalized,
    #[msg("Betting closed")] BettingClosed,
    #[msg("No blocks selected")] NoBlocksSelected,
    #[msg("Already claimed")] AlreadyClaimed,
    #[msg("Nothing to claim")] NothingToClaim,
    #[msg("Pool full")] PoolFull,
    #[msg("Already in pool")] AlreadyInPool,
    #[msg("Not in pool")] NotInPool,
    #[msg("Pool not found")] PoolNotFound,
    #[msg("Invalid fee")] InvalidFee,
    #[msg("Invalid mine level")] InvalidMineLevel,
    #[msg("Mine not unlocked")] MineNotUnlocked,
    #[msg("Motherlode not triggered")] MotherlodeNotTriggered,
    #[msg("Nothing to refine")] NothingToRefine,
    #[msg("Overflow")] Overflow,
    #[msg("Invalid parameter")] InvalidParameter,
    #[msg("Already initialized")] AlreadyInitialized,
    #[msg("Daily withdrawal limit exceeded")] DailyLimitExceeded,
    #[msg("AutoMiner not enabled")] AutoMinerNotEnabled,
    #[msg("AutoMiner already exists")] AutoMinerAlreadyExists,
    #[msg("Pool must bet all 5 blocks")] PoolMustBetAllBlocks,
    #[msg("Round has not ended yet")] RoundNotEnded,
    #[msg("Protocol is paused")] ProtocolPaused,
    #[msg("AutoMiner insufficient balance for bet")] AutoMinerInsufficientBalance,
    #[msg("AutoMiner already bet this round")] AutoMinerAlreadyBetThisRound,
}

// ============================================================================
// EVENTS
// ============================================================================

#[event] pub struct ProtocolInitialized { pub authority: Pubkey }
#[event] pub struct MinerInitialized { pub owner: Pubkey }
#[event] pub struct BetPlaced { pub miner: Pubkey, pub round: u64, pub total_sol: u64 }
#[event] pub struct RoundFinalized { pub round: u64, pub winning_block: u8, pub total_pot: u64, pub is_solo: bool }
#[event] pub struct SolClaimed { pub miner: Pubkey, pub amount: u64 }
#[event] pub struct SilverClaimed { pub miner: Pubkey, pub amount: u64, pub is_solo: bool }
#[event] pub struct PoolCreated { pub pool: Pubkey }
#[event] pub struct PoolJoined { pub pool: Pubkey, pub member: Pubkey }
#[event] pub struct PoolLeft { pub pool: Pubkey, pub member: Pubkey }
#[event] pub struct MotherlodeTriggered { pub winner: Pubkey, pub amount: u64 }
#[event] pub struct Refined { pub miner: Pubkey, pub amount: u64, pub redistributed: u64 }
#[event] pub struct RedistributionClaimed { pub miner: Pubkey, pub amount: u64 }
#[event] pub struct Staked { pub miner: Pubkey, pub amount: u64 }
#[event] pub struct Unstaked { pub miner: Pubkey, pub amount: u64 }
#[event] pub struct StakingRewardsClaimed { pub miner: Pubkey, pub amount: u64 }
#[event] pub struct AutoMinerSetup { pub owner: Pubkey, pub mine_level: u8 }
#[event] pub struct AutoMinerDeposit { pub owner: Pubkey, pub amount: u64 }
#[event] pub struct AutoMinerWithdraw { pub owner: Pubkey, pub amount: u64 }
#[event] pub struct AutoMinerBetPlaced { pub owner: Pubkey, pub round: u64, pub total_sol: u64, pub cranker: Pubkey }
#[event] pub struct FeesDistributed { pub round: u64, pub warchest: u64, pub motherlode: u64, pub admin: u64, pub autominer_treasury: u64 }

// ============================================================================
// STATE
// ============================================================================

#[account]
#[derive(Default)]
pub struct Config {
    pub authority: Pubkey,
    pub silver_mint: Pubkey,
    pub unrefined_mint: Pubkey,
    pub current_round: u64,
    pub round_start_time: i64,
    pub total_unrefined_supply: u64,
    pub total_silver_supply: u64,
    pub total_staked: u64,
    pub total_pools: u64,
    pub motherlode_balance: u64,
    pub motherlode_target: u64,
    pub staking_apr: u16,
    pub autominer_treasury: u64,
    pub redistribution_pool: u64,
    pub total_unrefined_holders: u64,
    pub config_bump: u8,
    pub silver_bump: u8,
    pub unrefined_bump: u8,
    pub initialized: bool,
    pub paused: bool,
    // v2 fields (occupy former 64-byte padding — zeros = safe defaults)
    pub admin_minted_silver: u64,
    pub motherlode_round_number: u64,
    pub motherlode_prize: u64,
    pub motherlode_winner_key: Pubkey,
    pub motherlode_best_score: u64,
}

impl Config {
    pub const SIZE: usize = 8 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8
        + 2 + 8 + 8 + 8 + 1 + 1 + 1 + 1 + 1
        + 8 + 8 + 8 + 32 + 8;
}

#[account]
pub struct Pool {
    pub creator: Pubkey,
    pub mine_level: u8,
    pub fee_bps: u16,
    pub member_count: u8,
    pub members: [Pubkey; 25],
    pub active: bool,
    pub bump: u8,
}

impl Default for Pool {
    fn default() -> Self {
        Self {
            creator: Pubkey::default(), mine_level: 0, fee_bps: 0,
            member_count: 0, members: [Pubkey::default(); 25],
            active: false, bump: 0,
        }
    }
}

impl Pool {
    pub const SIZE: usize = 8 + 32 + 1 + 2 + 1 + (32 * 25) + 1 + 1 + 32;

    pub fn is_member(&self, pubkey: &Pubkey) -> bool {
        for i in 0..self.member_count as usize {
            if self.members[i] == *pubkey { return true; }
        }
        false
    }
}

#[account]
#[derive(Default)]
pub struct Round {
    pub round_number: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub finalized: bool,
    pub winning_block: u8,
    pub is_solo: bool,
    pub solo_winner: Pubkey,
    pub solo_seed: u64,
    pub solo_best_score: u64,
    pub total_pot: u64,
    pub block_totals: [u64; 5],
    pub winner_pot: u64,
    pub bump: u8,
}

impl Round {
    pub const SIZE: usize = 8 + 8 + 8 + 8 + 1 + 1 + 1 + 32 + 8 + 8 + 8 + (8 * 5) + 8 + 1 + 32;
}

#[account]
#[derive(Default)]
pub struct Bet {
    pub miner: Pubkey,
    pub round: u64,
    pub mine_level: u8,
    pub blocks: [bool; 5],
    pub sol_per_block: u64,
    pub total_sol: u64,
    pub claimed: bool,
    pub silver_claimed: bool,
    pub bump: u8,
}

impl Bet {
    pub const SIZE: usize = 8 + 32 + 8 + 1 + 5 + 8 + 8 + 1 + 1 + 1 + 32;
}

#[account]
#[derive(Default)]
pub struct Miner {
    pub owner: Pubkey,
    pub current_mine: u8,
    pub total_sol_won: u64,
    pub pool: Pubkey,
    pub is_in_pool: bool,
    pub staked_amount: u64,
    pub pending_rewards: u64,
    pub last_stake_time: i64,
    pub pending_unrefined: u64,
    pub last_redistribution_claim: u64,
    pub bump: u8,
}

impl Miner {
    pub const SIZE: usize = 8 + 32 + 1 + 8 + 32 + 1 + 8 + 8 + 8 + 8 + 8 + 1 + 32;

    pub fn can_mine(&self, level: u8) -> bool { level <= self.current_mine }

    pub fn check_unlock(&mut self, unrefined_balance: u64) {
        for (level, &threshold) in THRESHOLDS.iter().enumerate() {
            if unrefined_balance >= threshold && level as u8 > self.current_mine {
                self.current_mine = level as u8;
            }
        }
    }
}

#[account]
#[derive(Default)]
pub struct AutoMiner {
    pub owner: Pubkey,
    pub enabled: bool,
    pub mine_level: u8,
    pub auto_reload: bool,
    pub balance: u64,
    pub sol_per_block: u64,
    pub daily_withdrawn: u64,
    pub last_withdrawal_day: i64,
    pub total_bets_placed: u64,
    pub total_winnings: u64,
    pub bump: u8,
}

impl AutoMiner {
    pub const SIZE: usize = 8 + 32 + 1 + 1 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 1 + 32;
}

// ============================================================================
// HELPERS
// ============================================================================

fn calculate_staking_rewards(staked: u64, elapsed: u64, apr: u16) -> u64 {
    let annual = (staked as u128) * (apr as u128) / 10_000;
    (annual * (elapsed as u128) / 31_536_000) as u64
}

fn generate_random(slot: u64, timestamp: i64, round: u64, seed: &[u8]) -> u64 {
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(&slot.to_le_bytes());
    v.extend_from_slice(&timestamp.to_le_bytes());
    v.extend_from_slice(&round.to_le_bytes());
    v.extend_from_slice(seed);
    v.extend_from_slice(&slot.wrapping_mul(0x517cc1b727220a95).to_le_bytes());
    v.extend_from_slice(&(timestamp as u64).wrapping_mul(0x9e3779b97f4a7c15).to_le_bytes());
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in v.iter() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    h ^= h >> 33; h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33; h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33; h
}

// ============================================================================
// INSTRUCTION ACCOUNTS
// ============================================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)] pub authority: Signer<'info>,
    #[account(init, payer = authority, space = Config::SIZE, seeds = [b"config"], bump)]
    pub config: Account<'info, Config>,
    #[account(init, payer = authority, seeds = [b"silver"], bump, mint::decimals = 9, mint::authority = config)]
    pub silver_mint: Account<'info, Mint>,
    #[account(init, payer = authority, seeds = [b"unrefined"], bump, mint::decimals = 9, mint::authority = config)]
    pub unrefined_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitializeRound<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(init, payer = payer, space = Round::SIZE,
              seeds = [b"round", config.current_round.to_le_bytes().as_ref()], bump)]
    pub round: Account<'info, Round>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeMiner<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(init, payer = owner, space = Miner::SIZE,
              seeds = [b"miner", owner.key().as_ref()], bump)]
    pub miner: Account<'info, Miner>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)] pub bettor: Signer<'info>,
    #[account(mut, seeds = [b"miner", bettor.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"round", config.current_round.to_le_bytes().as_ref()], bump = round.bump)]
    pub round: Account<'info, Round>,
    #[account(init, payer = bettor, space = Bet::SIZE,
              seeds = [b"bet", bettor.key().as_ref(), config.current_round.to_le_bytes().as_ref()], bump)]
    pub bet: Account<'info, Bet>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinalizeRound<'info> {
    #[account(mut)] pub settler: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"round", config.current_round.to_le_bytes().as_ref()], bump = round.bump)]
    pub round: Account<'info, Round>,
    /// CHECK: Warchest
    #[account(mut, address = WARCHEST_WALLET.parse::<Pubkey>().unwrap())]
    pub warchest: AccountInfo<'info>,
    /// CHECK: Admin
    #[account(mut, address = ADMIN_WALLET.parse::<Pubkey>().unwrap())]
    pub admin_wallet: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimSol<'info> {
    #[account(mut)] pub claimer: Signer<'info>,
    #[account(mut, seeds = [b"miner", claimer.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"round", bet.round.to_le_bytes().as_ref()], bump = round.bump)]
    pub round: Account<'info, Round>,
    #[account(mut, seeds = [b"bet", claimer.key().as_ref(), bet.round.to_le_bytes().as_ref()],
              bump = bet.bump, constraint = !bet.claimed @ SilverMiningError::AlreadyClaimed)]
    pub bet: Account<'info, Bet>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimSilver<'info> {
    #[account(mut)] pub claimer: Signer<'info>,
    #[account(mut, seeds = [b"miner", claimer.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"unrefined"], bump = config.unrefined_bump)]
    pub unrefined_mint: Account<'info, Mint>,
    #[account(mut)] pub claimer_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimBetSilver<'info> {
    #[account(mut)] pub claimer: Signer<'info>,
    #[account(mut, seeds = [b"miner", claimer.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"round", bet.round.to_le_bytes().as_ref()], bump = round.bump)]
    pub round: Account<'info, Round>,
    #[account(mut, seeds = [b"bet", claimer.key().as_ref(), bet.round.to_le_bytes().as_ref()],
              bump = bet.bump, constraint = !bet.silver_claimed @ SilverMiningError::AlreadyClaimed)]
    pub bet: Account<'info, Bet>,
    #[account(mut, seeds = [b"unrefined"], bump = config.unrefined_bump)]
    pub unrefined_mint: Account<'info, Mint>,
    #[account(mut)] pub claimer_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

/// FIX #1: Pool uses Box<Account> to avoid BPF stack overflow
#[derive(Accounts)]
pub struct CreatePool<'info> {
    #[account(mut)] pub creator: Signer<'info>,
    #[account(mut, seeds = [b"miner", creator.key().as_ref()], bump = miner.bump,
              constraint = !miner.is_in_pool @ SilverMiningError::AlreadyInPool)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(init, payer = creator, space = Pool::SIZE,
              seeds = [b"pool", config.total_pools.to_le_bytes().as_ref()], bump)]
    pub pool: Box<Account<'info, Pool>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinPool<'info> {
    #[account(mut)] pub member: Signer<'info>,
    #[account(mut, seeds = [b"miner", member.key().as_ref()], bump = miner.bump,
              constraint = !miner.is_in_pool @ SilverMiningError::AlreadyInPool)]
    pub miner: Account<'info, Miner>,
    #[account(mut)] pub pool: Box<Account<'info, Pool>>,
}

#[derive(Accounts)]
pub struct LeavePool<'info> {
    #[account(mut)] pub member: Signer<'info>,
    #[account(mut, seeds = [b"miner", member.key().as_ref()], bump = miner.bump,
              constraint = miner.is_in_pool @ SilverMiningError::NotInPool)]
    pub miner: Account<'info, Miner>,
    #[account(mut)] pub pool: Box<Account<'info, Pool>>,
}

#[derive(Accounts)]
pub struct Refine<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(mut, seeds = [b"miner", owner.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"unrefined"], bump = config.unrefined_bump)]
    pub unrefined_mint: Account<'info, Mint>,
    #[account(mut, seeds = [b"silver"], bump = config.silver_bump)]
    pub silver_mint: Account<'info, Mint>,
    #[account(mut)] pub owner_unrefined: Account<'info, TokenAccount>,
    #[account(mut)] pub owner_silver: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimRedistribution<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(mut, seeds = [b"miner", owner.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"silver"], bump = config.silver_bump)]
    pub silver_mint: Account<'info, Mint>,
    #[account(mut)] pub owner_silver: Account<'info, TokenAccount>,
    #[account(mut)] pub owner_unrefined: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(mut, seeds = [b"miner", owner.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut)] pub owner_silver: Account<'info, TokenAccount>,
    #[account(mut)] pub staking_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(mut, seeds = [b"miner", owner.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut)] pub owner_silver: Account<'info, TokenAccount>,
    #[account(mut)] pub staking_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimStakingRewards<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(mut, seeds = [b"miner", owner.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"silver"], bump = config.silver_bump)]
    pub silver_mint: Account<'info, Mint>,
    #[account(mut)] pub owner_silver: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TriggerMotherlode<'info> {
    #[account(mut)] pub winner: Signer<'info>,
    #[account(mut, seeds = [b"miner", winner.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
}

// ============================================================================
// AUTOMINER ACCOUNTS
// ============================================================================

#[derive(Accounts)]
pub struct SetupAutoMiner<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(seeds = [b"miner", owner.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(init, payer = owner, space = AutoMiner::SIZE,
              seeds = [b"autominer", owner.key().as_ref()], bump)]
    pub autominer: Account<'info, AutoMiner>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositAutoMiner<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(mut, seeds = [b"autominer", owner.key().as_ref()], bump = autominer.bump,
              constraint = autominer.owner == owner.key() @ SilverMiningError::Unauthorized)]
    pub autominer: Account<'info, AutoMiner>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawAutoMiner<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(mut, seeds = [b"autominer", owner.key().as_ref()], bump = autominer.bump,
              constraint = autominer.owner == owner.key() @ SilverMiningError::Unauthorized)]
    pub autominer: Account<'info, AutoMiner>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateAutoMiner<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(seeds = [b"miner", owner.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"autominer", owner.key().as_ref()], bump = autominer.bump,
              constraint = autominer.owner == owner.key() @ SilverMiningError::Unauthorized)]
    pub autominer: Account<'info, AutoMiner>,
}

#[derive(Accounts)]
pub struct DisableAutoMiner<'info> {
    #[account(mut)] pub owner: Signer<'info>,
    #[account(mut, seeds = [b"autominer", owner.key().as_ref()], bump = autominer.bump,
              constraint = autominer.owner == owner.key() @ SilverMiningError::Unauthorized)]
    pub autominer: Account<'info, AutoMiner>,
}

#[derive(Accounts)]
pub struct CrankAutoMiner<'info> {
    #[account(mut)] pub cranker: Signer<'info>,
    /// CHECK: AutoMiner owner
    pub autominer_owner: AccountInfo<'info>,
    #[account(seeds = [b"miner", autominer_owner.key().as_ref()], bump = miner.bump)]
    pub miner: Account<'info, Miner>,
    #[account(mut, seeds = [b"autominer", autominer_owner.key().as_ref()], bump = autominer.bump,
              constraint = autominer.owner == autominer_owner.key() @ SilverMiningError::Unauthorized,
              constraint = autominer.enabled @ SilverMiningError::AutoMinerNotEnabled)]
    pub autominer: Account<'info, AutoMiner>,
    #[account(seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"round", config.current_round.to_le_bytes().as_ref()], bump = round.bump)]
    pub round: Account<'info, Round>,
    #[account(init, payer = cranker, space = Bet::SIZE,
              seeds = [b"bet", autominer_owner.key().as_ref(), config.current_round.to_le_bytes().as_ref()], bump)]
    pub bet: Account<'info, Bet>,
    pub system_program: Program<'info, System>,
}

// ============================================================================
// v2 ACCOUNTS: METADATA, ADMIN MINT, COLLECT MOTHERLODE
// ============================================================================

#[derive(Accounts)]
pub struct CreateTokenMetadata<'info> {
    #[account(mut)] pub authority: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.config_bump,
              constraint = authority.key() == config.authority @ SilverMiningError::Unauthorized)]
    pub config: Account<'info, Config>,
    /// CHECK: Metaplex metadata PDA — validated in instruction
    #[account(mut)] pub metadata: AccountInfo<'info>,
    /// CHECK: Must be silver_mint or unrefined_mint — validated in instruction
    pub mint: AccountInfo<'info>,
    /// CHECK: Metaplex Token Metadata program
    #[account(address = METADATA_PROGRAM_ID.parse::<Pubkey>().unwrap())]
    pub token_metadata_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AdminMintSilver<'info> {
    #[account(mut)] pub authority: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump,
              constraint = authority.key() == config.authority @ SilverMiningError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"silver"], bump = config.silver_bump)]
    pub silver_mint: Account<'info, Mint>,
    #[account(mut)] pub destination: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

/// Permissionless: anyone can call after the 1-hour window.
/// SOL goes to the winner, not the caller.
#[derive(Accounts)]
pub struct CollectMotherlode<'info> {
    #[account(mut)] pub caller: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    #[account(seeds = [b"round", config.motherlode_round_number.to_le_bytes().as_ref()],
              bump = round.bump)]
    pub round: Account<'info, Round>,
    /// CHECK: Must match config.motherlode_winner_key
    #[account(mut)] pub winner: AccountInfo<'info>,
    #[account(mut, seeds = [b"miner", winner.key().as_ref()], bump = winner_miner.bump)]
    pub winner_miner: Account<'info, Miner>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    #[account(mut)] pub authority: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
}

#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    #[account(mut)] pub authority: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.config_bump)]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
}

// ============================================================================
// PROGRAM
// ============================================================================

#[program]
pub mod silver_mining {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let c = &mut ctx.accounts.config;
        require!(!c.initialized, SilverMiningError::AlreadyInitialized);
        let clock = Clock::get()?;
        c.authority = ctx.accounts.authority.key();
        c.silver_mint = ctx.accounts.silver_mint.key();
        c.unrefined_mint = ctx.accounts.unrefined_mint.key();
        c.current_round = 1;
        c.round_start_time = clock.unix_timestamp;
        c.total_unrefined_supply = 0;
        c.total_silver_supply = 0;
        c.total_staked = 0;
        c.total_pools = 0;
        c.motherlode_balance = 0;
        c.autominer_treasury = 0;
        c.redistribution_pool = 0;
        c.total_unrefined_holders = 0;
        let r = generate_random(clock.slot, clock.unix_timestamp, 0, b"motherlode_init");
        c.motherlode_target = (r % (MOTHERLODE_MAX - MOTHERLODE_MIN + 1)) + MOTHERLODE_MIN;
        c.staking_apr = DEFAULT_STAKING_APR;
        c.config_bump = ctx.bumps.config;
        c.silver_bump = ctx.bumps.silver_mint;
        c.unrefined_bump = ctx.bumps.unrefined_mint;
        c.initialized = true;
        c.paused = false;
        c.admin_minted_silver = 0;
        c.motherlode_round_number = 0;
        c.motherlode_prize = 0;
        c.motherlode_winner_key = Pubkey::default();
        c.motherlode_best_score = 0;
        emit!(ProtocolInitialized { authority: ctx.accounts.authority.key() });
        Ok(())
    }

    pub fn initialize_round(ctx: Context<InitializeRound>) -> Result<()> {
        let r = &mut ctx.accounts.round;
        let clock = Clock::get()?;
        r.round_number = ctx.accounts.config.current_round;
        r.start_time = clock.unix_timestamp;
        r.end_time = clock.unix_timestamp + ROUND_DURATION;
        r.finalized = false;
        r.winning_block = 0;
        r.is_solo = false;
        r.solo_winner = Pubkey::default();
        r.total_pot = 0;
        r.block_totals = [0; 5];
        r.winner_pot = 0;
        r.bump = ctx.bumps.round;
        Ok(())
    }

    pub fn initialize_miner(ctx: Context<InitializeMiner>) -> Result<()> {
        let m = &mut ctx.accounts.miner;
        m.owner = ctx.accounts.owner.key();
        m.current_mine = 0;
        m.total_sol_won = 0;
        m.pool = Pubkey::default();
        m.is_in_pool = false;
        m.staked_amount = 0;
        m.pending_rewards = 0;
        m.last_stake_time = 0;
        m.pending_unrefined = 0;
        m.last_redistribution_claim = 0;
        m.bump = ctx.bumps.miner;
        emit!(MinerInitialized { owner: ctx.accounts.owner.key() });
        Ok(())
    }

    pub fn place_bet(ctx: Context<PlaceBet>, mine_level: u8, blocks: [bool; 5], sol_per_block: u64) -> Result<()> {
        let config = &ctx.accounts.config;
        let round = &mut ctx.accounts.round;
        let bet = &mut ctx.accounts.bet;
        let miner = &ctx.accounts.miner;
        let clock = Clock::get()?;
        require!(!config.paused, SilverMiningError::ProtocolPaused);
        require!(clock.unix_timestamp < round.end_time, SilverMiningError::BettingClosed);
        require!(mine_level < 5, SilverMiningError::InvalidMineLevel);
        require!(miner.can_mine(mine_level), SilverMiningError::MineNotUnlocked);
        if miner.is_in_pool {
            require!(blocks.iter().all(|&b| b), SilverMiningError::PoolMustBetAllBlocks);
        }
        let block_count = blocks.iter().filter(|&&b| b).count() as u64;
        require!(block_count > 0, SilverMiningError::NoBlocksSelected);
        require!(sol_per_block > 0, SilverMiningError::InvalidAmount);
        let total_sol = sol_per_block.checked_mul(block_count).ok_or(SilverMiningError::Overflow)?;
        system_program::transfer(
            CpiContext::new(ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.bettor.to_account_info(),
                    to: round.to_account_info(),
                }),
            total_sol,
        )?;
        round.total_pot = round.total_pot.checked_add(total_sol).ok_or(SilverMiningError::Overflow)?;
        for (i, &selected) in blocks.iter().enumerate() {
            if selected {
                round.block_totals[i] = round.block_totals[i]
                    .checked_add(sol_per_block).ok_or(SilverMiningError::Overflow)?;
            }
        }
        bet.miner = ctx.accounts.bettor.key();
        bet.round = config.current_round;
        bet.mine_level = mine_level;
        bet.blocks = blocks;
        bet.sol_per_block = sol_per_block;
        bet.total_sol = total_sol;
        bet.claimed = false;
        bet.silver_claimed = false;
        bet.bump = ctx.bumps.bet;
        emit!(BetPlaced { miner: ctx.accounts.bettor.key(), round: bet.round, total_sol });
        Ok(())
    }

    // ========================================================================
    // FINALIZE ROUND + AUTO MOTHERLODE
    // ========================================================================

    pub fn finalize_round(ctx: Context<FinalizeRound>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let round = &mut ctx.accounts.round;
        let clock = Clock::get()?;
        require!(!config.paused, SilverMiningError::ProtocolPaused);
        require!(clock.unix_timestamp >= round.end_time, SilverMiningError::RoundNotEnded);
        require!(!round.finalized, SilverMiningError::RoundAlreadyFinalized);

        // Reclaim expired uncollected motherlode (no entrants within 10k rounds)
        if config.motherlode_prize > 0
            && config.motherlode_winner_key == Pubkey::default()
            && config.current_round > config.motherlode_round_number.saturating_add(MOTHERLODE_EXPIRY_ROUNDS)
        {
            config.motherlode_balance = config.motherlode_balance
                .checked_add(config.motherlode_prize).ok_or(SilverMiningError::Overflow)?;
            config.motherlode_prize = 0;
        }

        let total_pot = round.total_pot;
        if total_pot > 0 {
            let r1 = generate_random(clock.slot, clock.unix_timestamp, round.round_number, b"winning_block");
            let r2 = generate_random(clock.slot.wrapping_add(1), clock.unix_timestamp, round.round_number, b"solo_split");
            let r3 = generate_random(clock.slot.wrapping_add(2), clock.unix_timestamp, round.round_number, b"solo_seed");
            round.winning_block = (r1 % 5) as u8;
            round.is_solo = (r2 % 2) == 0;
            round.solo_seed = r3;
            round.solo_best_score = u64::MAX;
            round.solo_winner = Pubkey::default();

            let wf = total_pot.checked_mul(FEE_WARCHEST_BPS).ok_or(SilverMiningError::Overflow)? / 10000;
            let mf = total_pot.checked_mul(FEE_MOTHERLODE_BPS).ok_or(SilverMiningError::Overflow)? / 10000;
            let af = total_pot.checked_mul(FEE_ADMIN_BPS).ok_or(SilverMiningError::Overflow)? / 10000;
            let tf = total_pot.checked_mul(FEE_AUTOMINER_BPS).ok_or(SilverMiningError::Overflow)? / 10000;
            let ws = total_pot.checked_mul(WINNER_SHARE_BPS).ok_or(SilverMiningError::Overflow)? / 10000;
            round.winner_pot = ws;

            // Warchest (5%)
            **round.to_account_info().try_borrow_mut_lamports()? -= wf;
            **ctx.accounts.warchest.try_borrow_mut_lamports()? += wf;

            // Admin (1%)
            **round.to_account_info().try_borrow_mut_lamports()? -= af;
            **ctx.accounts.admin_wallet.try_borrow_mut_lamports()? += af;

            // Motherlode (4%) -> Config PDA
            **round.to_account_info().try_borrow_mut_lamports()? -= mf;
            **config.to_account_info().try_borrow_mut_lamports()? += mf;
            config.motherlode_balance = config.motherlode_balance
                .checked_add(mf).ok_or(SilverMiningError::Overflow)?;

            // AutoMiner treasury (1%) -> Config PDA
            **round.to_account_info().try_borrow_mut_lamports()? -= tf;
            **config.to_account_info().try_borrow_mut_lamports()? += tf;
            config.autominer_treasury = config.autominer_treasury
                .checked_add(tf).ok_or(SilverMiningError::Overflow)?;

            // Settler incentive
            let inc = std::cmp::min(SETTLER_INCENTIVE, wf / 10);
            if inc > 0 {
                **ctx.accounts.warchest.try_borrow_mut_lamports()? -= inc;
                **ctx.accounts.settler.try_borrow_mut_lamports()? += inc;
            }

            emit!(FeesDistributed {
                round: round.round_number, warchest: wf, motherlode: mf,
                admin: af, autominer_treasury: tf,
            });

            // --- AUTO MOTHERLODE: lock jackpot for single-winner raffle ---
            if config.current_round >= config.motherlode_target
                && config.motherlode_balance > 0
                && config.motherlode_prize == 0
            {
                let ml = config.motherlode_balance;
                config.motherlode_prize = ml;
                config.motherlode_round_number = round.round_number;
                config.motherlode_winner_key = Pubkey::default();
                config.motherlode_best_score = u64::MAX;
                config.motherlode_balance = 0;

                let mr = generate_random(clock.slot, clock.unix_timestamp, config.current_round, b"new_motherlode");
                let trig = (mr % (MOTHERLODE_MAX - MOTHERLODE_MIN + 1)) + MOTHERLODE_MIN;
                config.motherlode_target = config.current_round
                    .checked_add(1).ok_or(SilverMiningError::Overflow)?
                    .checked_add(trig).ok_or(SilverMiningError::Overflow)?;

                emit!(MotherlodeTriggered { winner: Pubkey::default(), amount: ml });
            }
        }

        round.finalized = true;
        config.current_round = config.current_round.checked_add(1).ok_or(SilverMiningError::Overflow)?;
        config.round_start_time = clock.unix_timestamp;
        emit!(RoundFinalized {
            round: round.round_number, winning_block: round.winning_block,
            total_pot, is_solo: round.is_solo,
        });
        Ok(())
    }

    // ========================================================================
    // CLAIMS
    // ========================================================================

    /// Claim SOL winnings. Also enters claimer into motherlode raffle.
    pub fn claim_sol(ctx: Context<ClaimSol>) -> Result<()> {
        let round = &ctx.accounts.round;
        let bet = &mut ctx.accounts.bet;
        let miner = &mut ctx.accounts.miner;
        let config = &mut ctx.accounts.config;
        require!(round.finalized, SilverMiningError::RoundNotFinalized);
        require!(!bet.claimed, SilverMiningError::AlreadyClaimed);
        let won = bet.blocks[round.winning_block as usize];
        require!(won, SilverMiningError::NothingToClaim);
        let wbt = round.block_totals[round.winning_block as usize];
        require!(wbt > 0, SilverMiningError::NothingToClaim);
        let share = (bet.sol_per_block as u128)
            .checked_mul(round.winner_pot as u128).ok_or(SilverMiningError::Overflow)?
            .checked_div(wbt as u128).ok_or(SilverMiningError::Overflow)?;
        let winnings = share as u64;
        require!(winnings > 0, SilverMiningError::NothingToClaim);
        bet.claimed = true;
        miner.total_sol_won = miner.total_sol_won.checked_add(winnings).ok_or(SilverMiningError::Overflow)?;
        **ctx.accounts.round.to_account_info().try_borrow_mut_lamports()? -= winnings;
        **ctx.accounts.claimer.try_borrow_mut_lamports()? += winnings;
        emit!(SolClaimed { miner: ctx.accounts.claimer.key(), amount: winnings });

        // --- MOTHERLODE RAFFLE ENTRY ---
        // Weighted random: score = hash / bet_size. Lower = wins.
        // Bigger bet = proportionally better chance, but ONE winner takes all.
        if bet.round == config.motherlode_round_number && config.motherlode_prize > 0 {
            let h = generate_random(
                round.solo_seed, 0, bet.round,
                ctx.accounts.claimer.key().as_ref(),
            );
            let score = h / std::cmp::max(bet.sol_per_block, 1);
            if score < config.motherlode_best_score {
                config.motherlode_best_score = score;
                config.motherlode_winner_key = ctx.accounts.claimer.key();
            }
        }
        Ok(())
    }

    pub fn claim_bet_silver(ctx: Context<ClaimBetSilver>) -> Result<()> {
        let round = &mut ctx.accounts.round;
        let bet = &mut ctx.accounts.bet;
        let miner = &mut ctx.accounts.miner;
        let config = &mut ctx.accounts.config;
        require!(round.finalized, SilverMiningError::RoundNotFinalized);
        require!(!bet.silver_claimed, SilverMiningError::AlreadyClaimed);
        let won = bet.blocks[round.winning_block as usize];
        require!(won, SilverMiningError::NothingToClaim);
        let emission = EMISSIONS[bet.mine_level as usize];
        let wbt = round.block_totals[round.winning_block as usize];
        let silver_amount = if wbt == 0 {
            0
        } else if round.is_solo {
            let ch = generate_random(round.solo_seed, 0, 0, ctx.accounts.claimer.key().as_ref());
            let ws = ch / std::cmp::max(bet.sol_per_block, 1);
            if ws < round.solo_best_score {
                round.solo_best_score = ws;
                round.solo_winner = ctx.accounts.claimer.key();
            }
            if round.solo_winner == ctx.accounts.claimer.key() { emission } else { 0 }
        } else {
            (emission as u128)
                .checked_mul(bet.sol_per_block as u128).ok_or(SilverMiningError::Overflow)?
                .checked_div(wbt as u128).ok_or(SilverMiningError::Overflow)? as u64
        };
        bet.silver_claimed = true;
        if silver_amount > 0 {
            let seeds = &[b"config".as_ref(), &[config.config_bump]];
            token::mint_to(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    MintTo {
                        mint: ctx.accounts.unrefined_mint.to_account_info(),
                        to: ctx.accounts.claimer_ata.to_account_info(),
                        authority: config.to_account_info(),
                    },
                    &[&seeds[..]],
                ),
                silver_amount,
            )?;
            miner.check_unlock(ctx.accounts.claimer_ata.amount + silver_amount);
            config.total_unrefined_supply = config.total_unrefined_supply
                .checked_add(silver_amount).ok_or(SilverMiningError::Overflow)?;
        }
        emit!(SilverClaimed {
            miner: ctx.accounts.claimer.key(), amount: silver_amount, is_solo: round.is_solo,
        });
        Ok(())
    }

    pub fn claim_silver(ctx: Context<ClaimSilver>) -> Result<()> {
        let miner = &mut ctx.accounts.miner;
        let config = &mut ctx.accounts.config;
        let amount = miner.pending_unrefined;
        require!(amount > 0, SilverMiningError::NothingToClaim);
        let seeds = &[b"config".as_ref(), &[config.config_bump]];
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.unrefined_mint.to_account_info(),
                    to: ctx.accounts.claimer_ata.to_account_info(),
                    authority: config.to_account_info(),
                },
                &[&seeds[..]],
            ),
            amount,
        )?;
        miner.pending_unrefined = 0;
        config.total_unrefined_supply = config.total_unrefined_supply
            .checked_add(amount).ok_or(SilverMiningError::Overflow)?;
        emit!(SilverClaimed { miner: ctx.accounts.claimer.key(), amount, is_solo: false });
        Ok(())
    }

    // ========================================================================
    // POOLS
    // ========================================================================

    pub fn create_pool(ctx: Context<CreatePool>, fee_bps: u16, mine_level: u8) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let miner = &mut ctx.accounts.miner;
        let pool = &mut ctx.accounts.pool;
        require!(fee_bps <= 500, SilverMiningError::InvalidFee);
        require!(mine_level < 5, SilverMiningError::InvalidMineLevel);
        require!(miner.can_mine(mine_level), SilverMiningError::MineNotUnlocked);
        pool.creator = ctx.accounts.creator.key();
        pool.mine_level = mine_level;
        pool.fee_bps = fee_bps;
        pool.member_count = 1;
        pool.members[0] = ctx.accounts.creator.key();
        pool.active = true;
        pool.bump = ctx.bumps.pool;
        miner.pool = ctx.accounts.pool.key();
        miner.is_in_pool = true;
        config.total_pools = config.total_pools.checked_add(1).ok_or(SilverMiningError::Overflow)?;
        emit!(PoolCreated { pool: ctx.accounts.pool.key() });
        Ok(())
    }

    pub fn join_pool(ctx: Context<JoinPool>) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        let miner = &mut ctx.accounts.miner;
        require!(pool.active, SilverMiningError::PoolNotFound);
        require!((pool.member_count as usize) < MAX_POOL_MEMBERS, SilverMiningError::PoolFull);
        require!(miner.can_mine(pool.mine_level), SilverMiningError::MineNotUnlocked);
        let idx = pool.member_count as usize;
        pool.members[idx] = ctx.accounts.member.key();
        pool.member_count += 1;
        miner.pool = ctx.accounts.pool.key();
        miner.is_in_pool = true;
        emit!(PoolJoined { pool: ctx.accounts.pool.key(), member: ctx.accounts.member.key() });
        Ok(())
    }

    pub fn leave_pool(ctx: Context<LeavePool>) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        let miner = &mut ctx.accounts.miner;
        let mk = ctx.accounts.member.key();
        require!(pool.creator != mk, SilverMiningError::Unauthorized);
        let mut found: Option<usize> = None;
        for i in 0..pool.member_count as usize {
            if pool.members[i] == mk { found = Some(i); break; }
        }
        let idx = found.ok_or(SilverMiningError::NotInPool)?;
        let last = (pool.member_count - 1) as usize;
        pool.members[idx] = pool.members[last];
        pool.members[last] = Pubkey::default();
        pool.member_count -= 1;
        miner.pool = Pubkey::default();
        miner.is_in_pool = false;
        emit!(PoolLeft { pool: ctx.accounts.pool.key(), member: mk });
        Ok(())
    }

    // ========================================================================
    // REFINING & REDISTRIBUTION
    // ========================================================================

    pub fn refine(ctx: Context<Refine>) -> Result<()> {
        let miner = &mut ctx.accounts.miner;
        let config = &mut ctx.accounts.config;
        let amount = ctx.accounts.owner_unrefined.amount;
        require!(amount > 0, SilverMiningError::NothingToRefine);
        let refiner_amount = amount.checked_mul(90).ok_or(SilverMiningError::Overflow)? / 100;
        let redistribution = amount.checked_sub(refiner_amount).ok_or(SilverMiningError::Overflow)?;
        token::burn(
            CpiContext::new(ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.unrefined_mint.to_account_info(),
                    from: ctx.accounts.owner_unrefined.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                }),
            amount,
        )?;
        let seeds = &[b"config".as_ref(), &[config.config_bump]];
        token::mint_to(
            CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.silver_mint.to_account_info(),
                    to: ctx.accounts.owner_silver.to_account_info(),
                    authority: config.to_account_info(),
                },
                &[&seeds[..]]),
            refiner_amount,
        )?;
        config.redistribution_pool = config.redistribution_pool
            .checked_add(redistribution).ok_or(SilverMiningError::Overflow)?;
        config.total_silver_supply = config.total_silver_supply
            .checked_add(refiner_amount).ok_or(SilverMiningError::Overflow)?;
        config.total_unrefined_supply = config.total_unrefined_supply.saturating_sub(amount);
        miner.current_mine = 0;
        emit!(Refined { miner: ctx.accounts.owner.key(), amount: refiner_amount, redistributed: redistribution });
        Ok(())
    }

    pub fn claim_redistribution(ctx: Context<ClaimRedistribution>) -> Result<()> {
        let miner = &mut ctx.accounts.miner;
        let config = &mut ctx.accounts.config;
        let ub = ctx.accounts.owner_unrefined.amount;
        require!(ub > 0, SilverMiningError::NothingToClaim);
        require!(config.redistribution_pool > 0, SilverMiningError::NothingToClaim);
        require!(config.total_unrefined_supply > 0, SilverMiningError::NothingToClaim);
        let share = (ub as u128)
            .checked_mul(config.redistribution_pool as u128).ok_or(SilverMiningError::Overflow)?
            .checked_div(config.total_unrefined_supply as u128).ok_or(SilverMiningError::Overflow)? as u64;
        require!(share > 0, SilverMiningError::NothingToClaim);
        let seeds = &[b"config".as_ref(), &[config.config_bump]];
        token::mint_to(
            CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.silver_mint.to_account_info(),
                    to: ctx.accounts.owner_silver.to_account_info(),
                    authority: config.to_account_info(),
                },
                &[&seeds[..]]),
            share,
        )?;
        config.redistribution_pool = config.redistribution_pool.saturating_sub(share);
        config.total_silver_supply = config.total_silver_supply
            .checked_add(share).ok_or(SilverMiningError::Overflow)?;
        miner.last_redistribution_claim = Clock::get()?.unix_timestamp as u64;
        emit!(RedistributionClaimed { miner: ctx.accounts.owner.key(), amount: share });
        Ok(())
    }

    // ========================================================================
    // STAKING
    // ========================================================================

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        let miner = &mut ctx.accounts.miner;
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;
        require!(amount > 0, SilverMiningError::InvalidAmount);
        if miner.staked_amount > 0 {
            let elapsed = (clock.unix_timestamp - miner.last_stake_time) as u64;
            let pending = calculate_staking_rewards(miner.staked_amount, elapsed, config.staking_apr);
            miner.pending_rewards = miner.pending_rewards
                .checked_add(pending).ok_or(SilverMiningError::Overflow)?;
        }
        token::transfer(
            CpiContext::new(ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.owner_silver.to_account_info(),
                    to: ctx.accounts.staking_vault.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                }),
            amount,
        )?;
        miner.staked_amount = miner.staked_amount
            .checked_add(amount).ok_or(SilverMiningError::Overflow)?;
        miner.last_stake_time = clock.unix_timestamp;
        config.total_staked = config.total_staked
            .checked_add(amount).ok_or(SilverMiningError::Overflow)?;
        emit!(Staked { miner: ctx.accounts.owner.key(), amount });
        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        let miner = &mut ctx.accounts.miner;
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;
        require!(amount > 0, SilverMiningError::InvalidAmount);
        require!(miner.staked_amount >= amount, SilverMiningError::InsufficientStake);
        let elapsed = (clock.unix_timestamp - miner.last_stake_time) as u64;
        let pending = calculate_staking_rewards(miner.staked_amount, elapsed, config.staking_apr);
        miner.pending_rewards = miner.pending_rewards
            .checked_add(pending).ok_or(SilverMiningError::Overflow)?;
        let seeds = &[b"config".as_ref(), &[config.config_bump]];
        token::transfer(
            CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.staking_vault.to_account_info(),
                    to: ctx.accounts.owner_silver.to_account_info(),
                    authority: config.to_account_info(),
                },
                &[&seeds[..]]),
            amount,
        )?;
        miner.staked_amount = miner.staked_amount
            .checked_sub(amount).ok_or(SilverMiningError::Overflow)?;
        miner.last_stake_time = clock.unix_timestamp;
        config.total_staked = config.total_staked.saturating_sub(amount);
        emit!(Unstaked { miner: ctx.accounts.owner.key(), amount });
        Ok(())
    }

    pub fn claim_staking_rewards(ctx: Context<ClaimStakingRewards>) -> Result<()> {
        let miner = &mut ctx.accounts.miner;
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;
        let elapsed = (clock.unix_timestamp - miner.last_stake_time) as u64;
        let current = if miner.staked_amount > 0 {
            calculate_staking_rewards(miner.staked_amount, elapsed, config.staking_apr)
        } else { 0 };
        let total = miner.pending_rewards
            .checked_add(current).ok_or(SilverMiningError::Overflow)?;
        require!(total > 0, SilverMiningError::NothingToClaim);
        let seeds = &[b"config".as_ref(), &[config.config_bump]];
        token::mint_to(
            CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.silver_mint.to_account_info(),
                    to: ctx.accounts.owner_silver.to_account_info(),
                    authority: config.to_account_info(),
                },
                &[&seeds[..]]),
            total,
        )?;
        miner.pending_rewards = 0;
        miner.last_stake_time = clock.unix_timestamp;
        config.total_silver_supply = config.total_silver_supply
            .checked_add(total).ok_or(SilverMiningError::Overflow)?;
        emit!(StakingRewardsClaimed { miner: ctx.accounts.owner.key(), amount: total });
        Ok(())
    }

    // ========================================================================
    // MOTHERLODE
    // ========================================================================

    /// Manual trigger (backward compat). Now uses weighted-random system.
    pub fn trigger_motherlode(ctx: Context<TriggerMotherlode>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;
        require!(config.current_round >= config.motherlode_target, SilverMiningError::MotherlodeNotTriggered);
        require!(config.motherlode_balance > 0, SilverMiningError::NothingToClaim);
        require!(config.motherlode_prize == 0, SilverMiningError::AlreadyClaimed);
        let ml = config.motherlode_balance;
        config.motherlode_prize = ml;
        config.motherlode_round_number = config.current_round;
        config.motherlode_winner_key = Pubkey::default();
        config.motherlode_best_score = u64::MAX;
        config.motherlode_balance = 0;
        let r = generate_random(clock.slot, clock.unix_timestamp, config.current_round, b"new_motherlode");
        let trig = (r % (MOTHERLODE_MAX - MOTHERLODE_MIN + 1)) + MOTHERLODE_MIN;
        config.motherlode_target = config.current_round
            .checked_add(trig).ok_or(SilverMiningError::Overflow)?;
        emit!(MotherlodeTriggered { winner: Pubkey::default(), amount: ml });
        Ok(())
    }

    /// Pays ONE winner. Permissionless — anyone calls after 1-hour window.
    pub fn collect_motherlode(ctx: Context<CollectMotherlode>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let round = &ctx.accounts.round;
        let clock = Clock::get()?;
        require!(config.motherlode_prize > 0, SilverMiningError::NothingToClaim);
        require!(
            config.motherlode_winner_key != Pubkey::default(),
            SilverMiningError::MotherlodeNotTriggered
        );
        require!(
            clock.unix_timestamp >= round.end_time + MOTHERLODE_CLAIM_WINDOW,
            SilverMiningError::RoundNotEnded
        );
        require!(
            ctx.accounts.winner.key() == config.motherlode_winner_key,
            SilverMiningError::Unauthorized
        );
        let amount = config.motherlode_prize;
        **config.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.winner.try_borrow_mut_lamports()? += amount;
        let winner_miner = &mut ctx.accounts.winner_miner;
        winner_miner.total_sol_won = winner_miner.total_sol_won
            .checked_add(amount).ok_or(SilverMiningError::Overflow)?;
        config.motherlode_prize = 0;
        config.motherlode_winner_key = Pubkey::default();
        config.motherlode_best_score = 0;
        emit!(MotherlodeTriggered { winner: ctx.accounts.winner.key(), amount });
        Ok(())
    }

    // ========================================================================
    // AUTOMINER
    // ========================================================================

    pub fn setup_autominer(ctx: Context<SetupAutoMiner>, mine_level: u8, auto_reload: bool, sol_per_block: u64) -> Result<()> {
        let miner = &ctx.accounts.miner;
        let autominer = &mut ctx.accounts.autominer;
        require!(mine_level <= miner.current_mine, SilverMiningError::MineNotUnlocked);
        autominer.owner = ctx.accounts.owner.key();
        autominer.enabled = true;
        autominer.mine_level = mine_level;
        autominer.auto_reload = auto_reload;
        autominer.balance = 0;
        autominer.sol_per_block = sol_per_block;
        autominer.daily_withdrawn = 0;
        autominer.last_withdrawal_day = 0;
        autominer.total_bets_placed = 0;
        autominer.total_winnings = 0;
        autominer.bump = ctx.bumps.autominer;
        emit!(AutoMinerSetup { owner: ctx.accounts.owner.key(), mine_level });
        Ok(())
    }

    pub fn deposit_autominer(ctx: Context<DepositAutoMiner>, amount: u64) -> Result<()> {
        require!(amount > 0, SilverMiningError::InvalidAmount);
        system_program::transfer(
            CpiContext::new(ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.owner.to_account_info(),
                    to: ctx.accounts.autominer.to_account_info(),
                }),
            amount,
        )?;
        let autominer = &mut ctx.accounts.autominer;
        autominer.balance = autominer.balance.checked_add(amount).ok_or(SilverMiningError::Overflow)?;
        emit!(AutoMinerDeposit { owner: ctx.accounts.owner.key(), amount });
        Ok(())
    }

    pub fn withdraw_autominer(ctx: Context<WithdrawAutoMiner>, amount: u64) -> Result<()> {
        let autominer = &mut ctx.accounts.autominer;
        require!(amount > 0, SilverMiningError::InvalidAmount);
        require!(autominer.balance >= amount, SilverMiningError::InsufficientBalance);
        **autominer.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.owner.try_borrow_mut_lamports()? += amount;
        autominer.balance = autominer.balance.checked_sub(amount).ok_or(SilverMiningError::Overflow)?;
        emit!(AutoMinerWithdraw { owner: ctx.accounts.owner.key(), amount });
        Ok(())
    }

    pub fn update_autominer(ctx: Context<UpdateAutoMiner>, mine_level: u8, auto_reload: bool, sol_per_block: u64, enabled: bool) -> Result<()> {
        let miner = &ctx.accounts.miner;
        let autominer = &mut ctx.accounts.autominer;
        require!(mine_level <= miner.current_mine, SilverMiningError::MineNotUnlocked);
        autominer.mine_level = mine_level;
        autominer.auto_reload = auto_reload;
        autominer.sol_per_block = sol_per_block;
        autominer.enabled = enabled;
        Ok(())
    }

    pub fn disable_autominer(ctx: Context<DisableAutoMiner>) -> Result<()> {
        ctx.accounts.autominer.enabled = false;
        Ok(())
    }

    /// FIX #2: Crank reimburses cranker full Bet rent + tip from autominer balance.
    pub fn crank_autominer(ctx: Context<CrankAutoMiner>) -> Result<()> {
        let config = &ctx.accounts.config;
        let round = &mut ctx.accounts.round;
        let bet = &mut ctx.accounts.bet;
        let miner = &ctx.accounts.miner;
        let autominer = &mut ctx.accounts.autominer;
        let clock = Clock::get()?;
        require!(!config.paused, SilverMiningError::ProtocolPaused);
        require!(clock.unix_timestamp < round.end_time, SilverMiningError::BettingClosed);
        require!(autominer.enabled, SilverMiningError::AutoMinerNotEnabled);
        require!(autominer.mine_level <= miner.current_mine, SilverMiningError::MineNotUnlocked);

        let blocks = [true, true, true, true, true];
        let sol_per_block = autominer.sol_per_block;
        let total_sol = sol_per_block.checked_mul(5).ok_or(SilverMiningError::Overflow)?;

        // Reimburse cranker for Bet account rent + small tip
        let rent = Rent::get()?;
        let bet_rent = rent.minimum_balance(Bet::SIZE);
        let crank_reimbursement = bet_rent.checked_add(CRANK_INCENTIVE).ok_or(SilverMiningError::Overflow)?;
        let total_needed = total_sol.checked_add(crank_reimbursement).ok_or(SilverMiningError::Overflow)?;
        require!(autominer.balance >= total_needed, SilverMiningError::AutoMinerInsufficientBalance);

        // Bet amount -> Round
        **autominer.to_account_info().try_borrow_mut_lamports()? -= total_sol;
        **round.to_account_info().try_borrow_mut_lamports()? += total_sol;

        // Rent + tip -> Cranker
        **autominer.to_account_info().try_borrow_mut_lamports()? -= crank_reimbursement;
        **ctx.accounts.cranker.try_borrow_mut_lamports()? += crank_reimbursement;

        autominer.balance = autominer.balance
            .checked_sub(total_needed).ok_or(SilverMiningError::Overflow)?;
        autominer.total_bets_placed = autominer.total_bets_placed
            .checked_add(1).ok_or(SilverMiningError::Overflow)?;

        round.total_pot = round.total_pot
            .checked_add(total_sol).ok_or(SilverMiningError::Overflow)?;
        for i in 0..5 {
            round.block_totals[i] = round.block_totals[i]
                .checked_add(sol_per_block).ok_or(SilverMiningError::Overflow)?;
        }

        bet.miner = ctx.accounts.autominer_owner.key();
        bet.round = config.current_round;
        bet.mine_level = autominer.mine_level;
        bet.blocks = blocks;
        bet.sol_per_block = sol_per_block;
        bet.total_sol = total_sol;
        bet.claimed = false;
        bet.silver_claimed = false;
        bet.bump = ctx.bumps.bet;
        emit!(AutoMinerBetPlaced {
            owner: ctx.accounts.autominer_owner.key(),
            round: config.current_round, total_sol,
            cranker: ctx.accounts.cranker.key(),
        });
        Ok(())
    }

    // ========================================================================
    // TOKEN METADATA (admin-only, call once per mint)
    // ========================================================================

    pub fn create_token_metadata(
        ctx: Context<CreateTokenMetadata>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        let config = &ctx.accounts.config;
        let mint_key = ctx.accounts.mint.key();
        require!(
            mint_key == config.silver_mint || mint_key == config.unrefined_mint,
            SilverMiningError::InvalidParameter
        );
        let metadata_program_id = ctx.accounts.token_metadata_program.key();
        let (expected_metadata, _) = Pubkey::find_program_address(
            &[b"metadata", metadata_program_id.as_ref(), mint_key.as_ref()],
            &metadata_program_id,
        );
        require!(ctx.accounts.metadata.key() == expected_metadata, SilverMiningError::InvalidParameter);

        // Build CreateMetadataAccountV3 instruction (Borsh-encoded)
        let mut data = Vec::new();
        data.push(33u8); // discriminator
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name.as_bytes());
        data.extend_from_slice(&(symbol.len() as u32).to_le_bytes());
        data.extend_from_slice(symbol.as_bytes());
        data.extend_from_slice(&(uri.len() as u32).to_le_bytes());
        data.extend_from_slice(uri.as_bytes());
        data.extend_from_slice(&0u16.to_le_bytes()); // seller_fee_basis_points
        data.push(0u8); // creators: None
        data.push(0u8); // collection: None
        data.push(0u8); // uses: None
        data.push(1u8); // is_mutable
        data.push(0u8); // collection_details: None

        let accounts = vec![
            AccountMeta::new(ctx.accounts.metadata.key(), false),
            AccountMeta::new_readonly(mint_key, false),
            AccountMeta::new_readonly(config.key(), true),
            AccountMeta::new(ctx.accounts.authority.key(), true),
            AccountMeta::new_readonly(config.key(), false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
            AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
        ];

        let seeds = &[b"config".as_ref(), &[config.config_bump]];
        invoke_signed(
            &Instruction { program_id: metadata_program_id, accounts, data },
            &[
                ctx.accounts.metadata.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                config.to_account_info(),
                ctx.accounts.authority.to_account_info(),
                config.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ],
            &[&seeds[..]],
        )?;
        Ok(())
    }

    // ========================================================================
    // ADMIN MINT (hard-capped at 50k SILVER)
    // ========================================================================

    pub fn admin_mint_silver(ctx: Context<AdminMintSilver>, amount: u64) -> Result<()> {
        let config = &mut ctx.accounts.config;
        require!(amount > 0, SilverMiningError::InvalidAmount);
        let new_total = config.admin_minted_silver
            .checked_add(amount).ok_or(SilverMiningError::Overflow)?;
        require!(new_total <= MAX_ADMIN_MINT_SILVER, SilverMiningError::Overflow);
        let seeds = &[b"config".as_ref(), &[config.config_bump]];
        token::mint_to(
            CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.silver_mint.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    authority: config.to_account_info(),
                },
                &[&seeds[..]]),
            amount,
        )?;
        config.admin_minted_silver = new_total;
        config.total_silver_supply = config.total_silver_supply
            .checked_add(amount).ok_or(SilverMiningError::Overflow)?;
        Ok(())
    }

    // ========================================================================
    // ADMIN
    // ========================================================================

    pub fn pause(ctx: Context<AdminOnly>) -> Result<()> {
        require!(ctx.accounts.authority.key() == ctx.accounts.config.authority, SilverMiningError::Unauthorized);
        ctx.accounts.config.paused = true;
        Ok(())
    }

    pub fn unpause(ctx: Context<AdminOnly>) -> Result<()> {
        require!(ctx.accounts.authority.key() == ctx.accounts.config.authority, SilverMiningError::Unauthorized);
        ctx.accounts.config.paused = false;
        Ok(())
    }

    pub fn withdraw_motherlode_fees(ctx: Context<WithdrawFees>, amount: u64) -> Result<()> {
        require!(ctx.accounts.authority.key() == ctx.accounts.config.authority, SilverMiningError::Unauthorized);
        require!(amount > 0, SilverMiningError::InvalidAmount);
        require!(ctx.accounts.config.motherlode_balance >= amount, SilverMiningError::InsufficientBalance);
        **ctx.accounts.config.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.authority.try_borrow_mut_lamports()? += amount;
        ctx.accounts.config.motherlode_balance = ctx.accounts.config.motherlode_balance
            .checked_sub(amount).ok_or(SilverMiningError::Overflow)?;
        Ok(())
    }

    pub fn withdraw_autominer_treasury(ctx: Context<WithdrawFees>, amount: u64) -> Result<()> {
        require!(ctx.accounts.authority.key() == ctx.accounts.config.authority, SilverMiningError::Unauthorized);
        require!(amount > 0, SilverMiningError::InvalidAmount);
        require!(ctx.accounts.config.autominer_treasury >= amount, SilverMiningError::InsufficientBalance);
        **ctx.accounts.config.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.authority.try_borrow_mut_lamports()? += amount;
        ctx.accounts.config.autominer_treasury = ctx.accounts.config.autominer_treasury
            .checked_sub(amount).ok_or(SilverMiningError::Overflow)?;
        Ok(())
    }

    pub fn update_staking_apr(ctx: Context<AdminOnly>, new_apr: u16) -> Result<()> {
        require!(ctx.accounts.authority.key() == ctx.accounts.config.authority, SilverMiningError::Unauthorized);
        require!(new_apr <= 10000, SilverMiningError::InvalidParameter);
        ctx.accounts.config.staking_apr = new_apr;
        Ok(())
    }

    /// Admin: reset a stuck motherlode (e.g. prize round was already claimed before trigger)
    /// Moves prize back to balance and re-rolls target for a future round.
    pub fn admin_reset_motherlode(ctx: Context<AdminOnly>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        require!(ctx.accounts.authority.key() == config.authority, SilverMiningError::Unauthorized);
        require!(config.motherlode_prize > 0, SilverMiningError::NothingToClaim);
        // Move prize back to balance
        config.motherlode_balance = config.motherlode_balance
            .checked_add(config.motherlode_prize).ok_or(SilverMiningError::Overflow)?;
        config.motherlode_prize = 0;
        config.motherlode_winner_key = Pubkey::default();
        config.motherlode_best_score = u64::MAX;
        config.motherlode_round_number = 0;
        // Re-roll target for a future round
        let clock = Clock::get()?;
        let r = generate_random(clock.slot, clock.unix_timestamp, config.current_round, b"motherlode_reset");
        let trig = (r % (MOTHERLODE_MAX - MOTHERLODE_MIN + 1)) + MOTHERLODE_MIN;
        config.motherlode_target = config.current_round
            .checked_add(trig).ok_or(SilverMiningError::Overflow)?;
        Ok(())
    }
}