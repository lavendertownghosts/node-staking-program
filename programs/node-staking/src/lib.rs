mod account; use account::*;
mod error; use error::ErrorCode;
mod access_control; use access_control::*;
mod helper; use helper::*;

use {
    anchor_lang::prelude::*,
    solana_program::{pubkey, pubkey::Pubkey},
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer},
        metadata::{
            create_metadata_accounts_v3,
            mpl_token_metadata::types::DataV2,
            CreateMetadataAccountsV3,
            Metadata as Metaplex,
        },
    }
};

declare_id!("C5F4J8RkHdWRtNkQsAtzFVDECLv7sncRVhukvVxfBpcs");

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
        treasury_to_selling: f32,
    ) -> Result<()> {
        let pool_state = &mut ctx.accounts.pool_state;

        pool_state.total_nodes = 0;
        pool_state.tokens_per_node = tokens_per_node;
        pool_state.reward_per_node = reward_per_node;
        pool_state.max_allocation = max_allocation;
        pool_state.treasury_to_selling = treasury_to_selling;

        Ok(())
    }

    pub fn initialize_selling_vault(_ctx: Context<InitializeSellingVault>) -> Result<()> {
        Ok(())
    }

    pub fn initialize_presale(
        ctx: Context<InitializePresale>,
        price_per_node: u64,            // when presale, sol to purchase node
        max_allocation: u16,            // when presale, limit number of nodes purchased by each wallet
        presale_start_at: i64,         
        presale_end_at: i64,
        total_presale_amount: u16,
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
            uri: "https://ipfs.io/ipfs/QmRtzvCek4tv3u9r1zjEm3wZbuT8MQtaaoAGonZK1CATkB".to_string(),
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
        let treasury_to_selling = ctx.accounts.pool_state.treasury_to_selling;
        let treasury_amount = (treasury_to_selling / (treasury_to_selling + 1.0)) * (amount as f32);
        let treasury_amount = treasury_amount as u64;
        let selling_amount = amount.checked_sub(treasury_amount).ok_or(ErrorCode::UnableCalculatingSellingTokens)?;

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.mint.to_account_info(), 
                MintTo {
                    authority: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.treasury_vault.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info()
                }, 
                &signer
            ), 
            treasury_amount,
        )?;

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.mint.to_account_info(), 
                MintTo {
                    authority: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.selling_vault.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info()
                }, 
                &signer
            ), 
            selling_amount,
        )?;

        Ok(())
    }

    pub fn mint_nodes(ctx: Context<MintNodes>, amount: u16) -> Result<()> {
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

    #[access_control(round_staking(&ctx.accounts.presale_state, &ctx.accounts.clock))]
    pub fn create_nodes(ctx: Context<CreateNodes>, amount: u16) -> Result<()> {
        let pool_nodes_amount = ctx.accounts.pool_state.total_nodes;
        let tokens_per_node = ctx.accounts.pool_state.tokens_per_node;
        let needed_tokens = tokens_per_node.checked_mul(amount.into()).ok_or(ErrorCode::UnableCalculatingNodesPrice)?;
        let user_token_balance = ctx.accounts.user_token_account.amount;

        require!(pool_nodes_amount >= amount, ErrorCode::LackNodes);
        require!(user_token_balance >= needed_tokens, ErrorCode::LackUserTokenBalance);
        require!(amount > ctx.accounts.pool_state.max_allocation, ErrorCode::UserAmountOverflow);

        let treasury_to_selling = ctx.accounts.pool_state.treasury_to_selling;
        let treasury_amount = (treasury_to_selling / (treasury_to_selling + 1.0)) * (needed_tokens as f32);
        let treasury_amount = treasury_amount as u64;

        let seeds = &[
            "pool_state".as_bytes(),
            &[ctx.bumps.pool_state]
        ];

        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(), 
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.selling_vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                }
            ),
            needed_tokens.into()
        )?;

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                Transfer {
                    from: ctx.accounts.selling_vault.to_account_info(),
                    to: ctx.accounts.treasury_vault.to_account_info(),
                    authority: ctx.accounts.pool_state.to_account_info(),
                },
                &[&seeds[..]]
            ),
            treasury_amount.into()
        )?;

        let user_stake_entry = &mut ctx.accounts.user_stake_entry;
        let user_nodes = user_stake_entry.staked_amount;

        if user_nodes == 0 {
            user_stake_entry.staked_amount = amount;
            user_stake_entry.claimable_amount = 0;
            user_stake_entry.last_staked_at = ctx.accounts.clock.unix_timestamp;
        } else {
            user_stake_entry.staked_amount = user_nodes.checked_add(amount).ok_or(ErrorCode::UnavailableCaculateSum)?;
            let user_staked_amount = user_stake_entry.staked_amount;
            let user_claimable_amount = user_stake_entry.claimable_amount;
            let last_staked_period = ctx.accounts.clock.unix_timestamp - user_stake_entry.last_staked_at;
            let additional_claim = user_staked_amount as f32 * 0.5 * ctx.accounts.selling_mint.decimals as f32 * last_staked_period as f32 / 86400 as f32;
            let additional_claim = additional_claim as u64;
            user_stake_entry.claimable_amount = user_claimable_amount.checked_add(additional_claim).ok_or(ErrorCode::UnavailableCaculateSum)?;
            user_stake_entry.staked_amount = user_staked_amount.checked_add(amount).ok_or(ErrorCode::UnavailableCaculateSum)?;
            user_stake_entry.last_staked_at = ctx.accounts.clock.unix_timestamp;
        }

        let pool_state = &mut ctx.accounts.pool_state;
        pool_state.total_nodes = pool_nodes_amount.checked_sub(amount).ok_or(ErrorCode::UnavailableCaculateSub)?;

        Ok(())
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
    pub treasury_vault: Account<'info, TokenAccount>,
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
pub struct InitializeSellingVault<'info> {
    #[account(
        seeds = [b"pool_state"],
        bump,
    )]
    pub pool_state: Account<'info, PoolState>,
    #[account(
        seeds = [b"mint"],
        bump,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = pool_authority,
        associated_token::mint = mint,
        associated_token::authority = pool_state
    )]
    pub selling_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = pool_authority.key() == POOL_AUTHORITY
        @ ErrorCode::InvalidPoolAuthority
    )]
    pub pool_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>
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
        seeds = [b"pool_state"],
        bump,
    )]
    pub pool_state: Account<'info, PoolState>,
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
    pub treasury_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = pool_state
    )]
    pub selling_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = pool_authority.key() == POOL_AUTHORITY
        @ ErrorCode::InvalidPoolAuthority
    )]
    pub pool_authority: Signer<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
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

#[derive(Accounts)]
pub struct CreateNodes<'info> {
    #[account(
        seeds = [b"presale_state"],
        bump,
    )]
    pub presale_state: Account<'info, PresaleState>,
    #[account(
        seeds = [b"pool_state"],
        bump,
        has_one = selling_mint,
        has_one = selling_vault,
    )]
    pub pool_state: Account<'info, PoolState>,
    #[account(
        seeds = [b"mint"],
        bump,
    )]
    pub selling_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = selling_mint,
        associated_token::authority = POOL_AUTHORITY,
    )]
    pub treasury_vault: Account<'info, TokenAccount>,
    #[account(
        associated_token::mint = selling_mint,
        associated_token::authority = pool_state,
    )]
    pub selling_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = user_token_account.owner == user.key(),
        constraint = user_token_account.mint == selling_mint.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [user.key().as_ref()],
        bump,
    )]
    pub user_stake_entry: Account<'info, UserStakeEntry>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}