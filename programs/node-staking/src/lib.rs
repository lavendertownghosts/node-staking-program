mod account; use account::*;
mod error; use error::ErrorCode;
mod access_control; use access_control::*;
mod helper; use helper::*;

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
        max_allocation: u16,            // limit number of nodes purchased by each wallet
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
        max_allocation: u16,            // when presale, limit number of nodes purchased by each wallet
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
        let user_stake_entry = &mut ctx.accounts.user_stake_entry;
        user_stake_entry.stakes_number = 0;

        let stake_info = StakeInfo {
            amount: 0,
            stake_date: Clock::get().unwrap().unix_timestamp,
        };

        user_stake_entry.stakes.push(stake_info);

        Ok(())
    }

    #[access_control(round_presale(&ctx.accounts.presale_state, &ctx.accounts.clock))]
    pub fn sell_nodes_at_presale(ctx: Context<PresaleNodes>, amount: u16) -> Result<()> {
        let user_stake_entry = &mut ctx.accounts.user_stake_entry;
        let presale_state = &mut ctx.accounts.presale_state;
        let pool_state = &mut ctx.accounts.pool_state;
        let user_lamports = **ctx.accounts.user.to_account_info().try_borrow_lamports()?;
        let needed_lamports = presale_state.price_per_node.checked_mul(amount.into()).ok_or(ErrorCode::UnableCalculatingNodesPrice)?;

        require!(user_stake_entry.stakes_number + amount < presale_state.max_allocation, 
            ErrorCode::StakesAmountOverflow
        );
        require!(pool_state.total_nodes > amount.into(), 
            ErrorCode::LackNodes
        );
        require!(user_lamports > needed_lamports, ErrorCode::InsufficientBalanceForPresale);

        let user = &mut ctx.accounts.user;
        let presale_valut = &mut ctx.accounts.presale_vault;

        send_lamports(user.to_account_info(), presale_valut.to_account_info(), needed_lamports)?;

        let stake_info = StakeInfo {
            amount,
            stake_date: presale_state.presale_end_at,
        };

        user_stake_entry.stakes_number = user_stake_entry.stakes_number.checked_add(amount).ok_or(ErrorCode::UserAmountOverflow)?;

        user_stake_entry.stakes.push(stake_info);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePresale<'info> {
    #[account(
        init, 
        payer = pool_authority, 
        space = 8 + PresaleState::SPACE,
        seeds = [b"pool_state"],
        bump
    )]
    pub presale: Account<'info, PresaleState>, 
    #[account(
        init,
        payer = pool_authority,
        space = 8,
        seeds = [b"presale_vault"],
        bump
    )]
    pub presale_valut: Account<'info, PresaleVault>,
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
pub struct PresaleVault{

}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init, 
        payer = pool_authority, 
        space = 8 + PoolState::SPACE,
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
        space = 8 + UserStakeEntry::SPACE,
        seeds = [user.key().as_ref()],
        bump
    )]
    pub user_stake_entry: Account<'info, UserStakeEntry>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct PresaleNodes<'info> {
    #[account(
        mut,
        seeds = [user.key().as_ref()],
        bump,
        realloc = 8 + UserStakeEntry::SPACE + StakeInfo::SPACE * (user_stake_entry.stakes_number as usize + 1),
        realloc::payer = user,
        realloc::zero = false,
    )]
    pub user_stake_entry: Account<'info, UserStakeEntry>,
    #[account(
        mut,
        seeds = [b"presale_vault"],
        bump,
    )]
    pub presale_vault: Account<'info, PresaleVault>,
    #[account(mut)]
    pub pool_state: Account<'info, PoolState>,
    pub presale_state: Account<'info, PresaleState>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}