mod account; use account::*;
mod error; use error::ErrorCode;
mod access_control; use access_control::*;
mod helper; use helper::*;

use {
    anchor_lang::prelude::*,
    solana_program::{pubkey, pubkey::Pubkey},
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
        metadata::{
            create_metadata_accounts_v3,
            mpl_token_metadata::types::DataV2,
            CreateMetadataAccountsV3,
            Metadata as Metaplex,
        },
    }
};

declare_id!("4dtYh4bYBJ8P2ssASw5izqLjfTEJYMDHCmuhAipcr6vc");

pub static POOL_AUTHORITY: Pubkey = pubkey!("EMZQyHyda9aXWqJsJYDUHCEbE5kibagRkNxY8TbPndYx");

pub static VAULT_AUTHORITY: Pubkey = pubkey!("6JvsMVc9rwY9AG63qsqrfoDcNPgRmx9JfMHMHaX7TRoS");

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
        
        let pool_state = &mut ctx.accounts.pool_state;
        pool_state.total_nodes = total_presale_amount;

        Ok(())
    }

    pub fn initialize_token (ctx: Context<InitializeToken>) -> Result<()> {
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        let token_data: DataV2 = DataV2 {
            name: "Solana Node Staking Token".to_string(),
            symbol: "NST".to_string(),
            uri: "https://ipfs.io/ipfs/QmQ5m5WQPrgDGU24KmPJsCiMzAWyEaZZuohxssVeZP8LVH".to_string(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let metadata_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(), 
            CreateMetadataAccountsV3 {
                payer: ctx.accounts.pool_authority.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                metadata: ctx.accounts.metadata.to_account_info(),
                mint_authority: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            }, 
            &signer
        );

        create_metadata_accounts_v3(
            metadata_ctx,
            token_data,
            false,
            true,
            None,
        )?;

        Ok(())
    }

    pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.mint.to_account_info(), 
                MintTo {
                    authority: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.token_vault.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info()
                }, 
                &signer
            ), 
            amount,
        )?;

        Ok(())
    }

    pub fn mint_nodes(ctx: Context<MintNodes>, amount: u64) -> Result<()> {
        let pool_state = &mut ctx.accounts.pool_state;
        pool_state.total_nodes = pool_state.total_nodes.checked_add(amount).ok_or(ErrorCode::AmountOverflow)?;

        Ok(())
    }

    pub fn initialize_user_stake(ctx: Context<InitializeUserStake>) -> Result<()> {
        let user_stake_entry = &mut ctx.accounts.user_stake_entry;
        user_stake_entry.claimable_amount = 0;
        user_stake_entry.staked_amount = 0;

        Ok(())
    }

    #[access_control(round_presale(&ctx.accounts.presale_state, &ctx.accounts.clock))]
    pub fn sell_nodes_at_presale(ctx: Context<PresaleNodes>, amount: u16) -> Result<()> {
        let user_stake_entry = &mut ctx.accounts.user_stake_entry;
        let presale_state = &mut ctx.accounts.presale_state;
        let pool_state = &mut ctx.accounts.pool_state;
        let user_lamports = **ctx.accounts.user.to_account_info().try_borrow_lamports()?;
        let needed_lamports = presale_state.price_per_node.checked_mul(amount.into()).ok_or(ErrorCode::UnableCalculatingNodesPrice)?;

        require!(user_stake_entry.staked_amount + amount < presale_state.max_allocation, 
            ErrorCode::StakesAmountOverflow
        );
        require!(pool_state.total_nodes >= amount.into(), 
            ErrorCode::LackNodes
        );
        require!(user_lamports > needed_lamports, ErrorCode::InsufficientBalanceForPresale);

        let user = &mut ctx.accounts.user;
        let presale_valut = &mut ctx.accounts.presale_vault;

        send_lamports(user.to_account_info(), presale_valut.to_account_info(), needed_lamports)?;

        user_stake_entry.staked_amount = user_stake_entry.staked_amount.checked_add(amount).ok_or(ErrorCode::UserAmountOverflow)?;

        user_stake_entry.last_staked_at = presale_state.presale_end_at;

        pool_state.total_nodes = pool_state.total_nodes.checked_sub(amount.into()).ok_or(ErrorCode::LackNodes)?;

        Ok(())
    }

    pub fn withdraw_cap(ctx: Context<WithdrawCap>) -> Result<()> {
        ctx.accounts.send_lamports_from_vault_to_owner()
    }
}

#[derive(Accounts)]
pub struct InitializePresale<'info> {
    #[account(
        init, 
        payer = pool_authority, 
        space = 8 + PresaleState::SPACE,
        seeds = [b"presale_state"],
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
    #[account(
        mut,
        seeds = [b"pool_state"],
        bump,
    )]
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
        seeds = [b"pool_state"],
        bump,
    )]
    pub pool_state: Account<'info, PoolState>,
    #[account(
        init,
        seeds = [b"mint"],
        bump,
        payer = pool_authority,
        mint::decimals = 18,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = pool_authority,
        associated_token::mint = mint,
        associated_token::authority = pool_authority
    )]
    pub token_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = pool_authority.key() == POOL_AUTHORITY
        @ ErrorCode::InvalidPoolAuthority
    )]
    pub pool_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeToken<'info> {
    #[account(mut)]
    /// CHECK: This is not dangerous because we are interacting with the metadata account managed by the Metadata program
    pub metadata: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = pool_authority.key() == POOL_AUTHORITY
        @ ErrorCode::InvalidPoolAuthority
    )]
    pub pool_authority: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metaplex>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = pool_authority
    )]
    pub token_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = pool_authority.key() == POOL_AUTHORITY
        @ ErrorCode::InvalidPoolAuthority
    )]
    pub pool_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintNodes<'info> {
    #[account(
        mut,
        seeds = [b"pool_state"],
        bump,
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
        // realloc = 8 + UserStakeEntry::SPACE + StakeInfo::SPACE * (user_stake_entry.stakes_number as usize + 1),
        // realloc::payer = user,
        // realloc::zero = false,
    )]
    pub user_stake_entry: Account<'info, UserStakeEntry>,
    #[account(
        mut,
        seeds = [b"presale_vault"],
        bump,
    )]
    pub presale_vault: Account<'info, PresaleVault>,
    #[account(
        mut,
        seeds = [b"pool_state"],
        bump,
    )]
    pub pool_state: Account<'info, PoolState>,
    pub presale_state: Account<'info, PresaleState>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct WithdrawCap<'info> {
    #[account(
        mut,
        seeds = [b"presale_vault"],
        bump
    )]
    pub presale_vault: Account<'info, PresaleVault>,
    #[account(
        mut,
        constraint = withdrawer.key() == VAULT_AUTHORITY
        @ ErrorCode::InvalidVaultAuthority
    )]
    pub withdrawer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> WithdrawCap<'info> {
    pub fn send_lamports_from_vault_to_owner(&mut self) -> Result<()> {
        let presale_vault = self.presale_vault.to_account_info();
        let presale_vault_data_len = presale_vault.try_data_len()?;
        let presale_vault_minimum_rent_exempt_balance = self.rent.minimum_balance(presale_vault_data_len);
        let all_presale_vault_lamports = **presale_vault.try_borrow_lamports()?;
        let available_lamports = all_presale_vault_lamports - presale_vault_minimum_rent_exempt_balance;

        **presale_vault.try_borrow_mut_lamports()? -= available_lamports;
        **self.withdrawer.try_borrow_mut_lamports()? += available_lamports;

        Ok(())
    }
}