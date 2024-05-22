use anchor_lang::prelude::*;

#[account]
pub struct PresaleState {
    pub price_per_node: u64,
    pub max_allocation: u16,
    pub presale_start_at: i64,
    pub presale_end_at: i64,
    pub total_presale_amount: u64,
}

impl PresaleState {
    pub const SPACE: usize = 8 * 2 + 8 * 2 + 2;
}

#[account]
pub struct PoolState {
    pub max_allocation: u16,
    pub total_nodes: u64,
    pub total_tokens: u64,
    pub tokens_per_node: u64,
    pub reward_per_node: u8,
}

impl PoolState {
    pub const SPACE: usize = 8 * 3 + 2 + 1;
}

#[account]
pub struct UserStakeEntry {
    pub claimable_amount: u16,
    pub staked_amount: u16,
    pub last_staked_at: i64,
}

impl UserStakeEntry {
    pub const SPACE: usize = 2 + 2 + 8;
}