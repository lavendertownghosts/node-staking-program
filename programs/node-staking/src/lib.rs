use anchor_lang::prelude::*;

declare_id!("AXqBSUwjqjcjyvmo1P3ziVZiSXFiLw5cRJo2abbLPaBa");

#[program]
pub mod node_staking {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
