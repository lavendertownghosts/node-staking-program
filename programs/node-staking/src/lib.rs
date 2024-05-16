mod error; use error::ErrorCode;
mod access_control; use access_control::*;

use {
    anchor_lang::prelude::*,
    solana_program::{pubkey, pubkey::Pubkey},
};

declare_id!("AXqBSUwjqjcjyvmo1P3ziVZiSXFiLw5cRJo2abbLPaBa");

pub static POOL_AUTHORITY: Pubkey = pubkey!("6NkVPy6o8q4Rg3nPS54mwt1hegpdfbrW7ra9Zo2RLHGg");

#[program]
pub mod node_staking {
    use super::*;

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        tokens_per_node: u64,           // number of tokens to purchase node
        reward_per_node: u8,            // reward of each node to user per day
        max_allocation: u64,            // limit number of nodes purchased by each wallet
    ) -> Result<()> {
        let pool_state = &mut ctx.accounts.pool_state;

        pool_state.total_nodes = 0;
        pool_state.tokens_per_node = tokens_per_node;
        pool_state.reward_per_node = reward_per_node;
        pool_state.max_allocation = max_allocation;

        Ok(())
    }

    pub fn initialize_presale(
        ctx: Context<InitializePresale>,
        price_per_node: u64,            // when presale, sol to purchase node
        max_allocation: u64,            // when presale, limit number of nodes purchased by each wallet
        presale_start_at: i64,         
        presale_end_at: i64,
        total_presale_amount: u64,
    ) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        presale.price_per_node = price_per_node;
        presale.max_allocation = max_allocation;
        presale.presale_start_at = presale_start_at;
        presale.presale_end_at = presale_end_at;
        presale.total_presale_amount = total_presale_amount;
        presale.sold_nodes = 0;
        
        let pool_state = &mut ctx.accounts.pool_state;
        pool_state.total_nodes = total_presale_amount;

        Ok(())
    }

    pub fn mint_nodes(ctx: Context<MintNodes>, amount: u64) -> Result<()> {
        let pool_state = &mut ctx.accounts.pool_state;
        pool_state.total_nodes = pool_state.total_nodes.checked_add(amount).ok_or(ErrorCode::AmountOverflow)?;

        Ok(())
    }

    pub fn initialize_user_stake(ctx: Context<InitializeUserStake>) -> Result<()> {
        Ok(())
    }

    #[access_control(round_presale(&ctx.accounts.presale, &ctx.accounts.clock))]
    pub fn sell_nodes_at_presale(ctx: Context<PresaleNodes>, amount: u64) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePresale<'info> {
    #[account(
        init, 
        payer = pool_authority, 
        space = 8 + 8 + 8 + 8 + 8 + 8 + 8,
        seeds = [b"pool_state"],
        bump
    )]
    pub presale: Account<'info, PresaleState>, 
    #[account(
        mut,
        constraint = pool_authority.key() == POOL_AUTHORITY
        @ ErrorCode::InvalidPoolAuthority
    )]
    pub pool_authority: Signer<'info>,
    pub pool_state: Account<'info, PoolState>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct PresaleState {
    pub price_per_node: u64,
    pub max_allocation: u64,
    pub presale_start_at: i64,
    pub presale_end_at: i64,
    pub total_presale_amount: u64,
    pub sold_nodes: u64
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init, 
        payer = pool_authority, 
        space = 8 + 41,
        seeds = [b"presale_state"],
        bump
    )]
    pub pool_state: Account<'info, PoolState>,
    #[account(
        mut,
        constraint = pool_authority.key() == POOL_AUTHORITY
        @ ErrorCode::InvalidPoolAuthority
    )]
    pub pool_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct PoolState {
    pub max_allocation: u64,
    pub total_nodes: u64,
    pub total_tokens: u64,
    pub tokens_per_node: u64,
    pub reward_per_node: u8,
}

#[derive(Accounts)]
pub struct MintNodes<'info> {
    #[account(
        mut,
        seeds = [b"pool_state"],
        bump
    )]
    pub pool_state: Account<'info, PoolState>,
    #[account(
        constraint = pool_authority.key() == POOL_AUTHORITY
        @ ErrorCode::InvalidPoolAuthority
    )]
    pub pool_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitializeUserStake<'info> {
    #[account(
        init,
        payer = user,
        space = 14 + 16,
        seeds = [user.key().as_ref()],
        bump
    )]
    pub user_stake_entry: Account<'info, UserStakeEntry>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[account]
pub struct UserStakeEntry {
    stakes_number: u16,
    stakes: Vec<StakeInfo>
}

#[account]
pub struct StakeInfo {
    amount: u64,
    stake_date: i64,
}

#[derive(Accounts)]
pub struct PresaleNodes<'info> {
    #[account(
        mut,
        seeds = [user.key().as_ref()],
        bump,
        realloc = 14 + user_stake_entry.stakes_number as usize,
        realloc::payer = user,
        realloc::zero = false,
    )]
    pub user_stake_entry: Account<'info, UserStakeEntry>,
    #[account(mut)]
    pub pool_state: Account<'info, PoolState>,
    pub presale_state: Account<'info, PresaleState>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}