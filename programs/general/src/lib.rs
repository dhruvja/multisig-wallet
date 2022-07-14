use anchor_lang::prelude::*;
use anchor_spl::token::Mint;    

declare_id!("5urf7xSHXmvWP6oxLHHhW1aEefDw1D4Wq342ccRFBBv5");

const GENERAL_SEED: &'static [u8] = b"general";

#[program]
pub mod general {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {

        let parameters = &mut ctx.accounts.base_account;
        parameters.authority = ctx.accounts.authority.key();
        parameters.token_mint = ctx.accounts.token_mint.key();

        Ok(())
    }

    pub fn change_mint(ctx: Context<ChangeMint>, _base_bump: u8) -> Result<()> {

        let parameters = &mut ctx.accounts.base_account;
        parameters.token_mint = ctx.accounts.token_mint.key();

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, seeds = [GENERAL_SEED], bump, space = 32 + 32 + 32 + 1 + 1 + 8)]
    pub base_account: Account<'info, GeneralParameter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub token_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(base_bump: u8)]
pub struct ChangeMint<'info> {
    #[account(mut, seeds = [GENERAL_SEED], bump = base_bump, has_one = authority)]
    pub base_account: Account<'info,GeneralParameter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub token_mint: Account<'info, Mint>,
}

#[account]
pub struct GeneralParameter {
    pub authority: Pubkey, // 32
    pub token_mint: Pubkey, // 32
    pub min_percentage_amount_to_transfer: u8, // 1
    pub admin_wallet: Pubkey, // 32
    pub version: u8 //1
}
