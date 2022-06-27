use anchor_lang::prelude::*;
use general::program::General;
use general::{self, GeneralParameter};
use project::program::Project;
use project::{self, ProjectParameter};
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};

declare_id!("F5s5fXeXWgWf8L42NerMVxRHUwWsQhgM5tguedzvNwjC");

const TRANSFER_SEED: &'static [u8] = b"transfer";
const GENERAL_SEED: &'static [u8] = b"general";
const PROJECT_SEED: &'static [u8] = b"project";
const POOL_SEED: &'static [u8] = b"pool";

#[program]
pub mod transfer {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, transfer_id: String, _general_bump: u8, base_bump: u8, amount: u32, receiver: Pubkey) -> Result<()> {

        let parameters = &mut ctx.accounts.base_account;
        let general_parameters = &mut ctx.accounts.general_account;

        if general_parameters.token_mint == ctx.accounts.token_mint.key() {
            msg!("Mint is matching");

            let bump_vector = base_bump.to_le_bytes();
                let inner = vec![
                    TRANSFER_SEED,
                    transfer_id.as_bytes()[..18].as_ref(),
                    transfer_id.as_bytes()[18..].as_ref(),
                    bump_vector.as_ref(),
                ];
                let outer = vec![inner.as_slice()];

                // Below is the actual instruction that we are going to send to the Token program.
                let transfer_instruction = Transfer {
                    from: ctx.accounts.wallet_to_withdraw_from.to_account_info(),
                    to: ctx.accounts.project_pool_wallet.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                };
                let cpi_ctx = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    transfer_instruction,
                    outer.as_slice(), //signer PDA
                );

                let amount_in_32 = amount as u64;

                // The `?` at the end will cause the function to return early in case of an error.
                // This pattern is common in Rust.
                anchor_spl::token::transfer(cpi_ctx, amount_in_32)?;
        }

        parameters.amount = amount;
        parameters.receiver = receiver;
        parameters.authority = general_parameters.authority;



        Ok(())
    }

    pub fn sign_transfer(ctx: Context<SignTransfer>, _base_bump: u8, _general_bump: u8, _project_bump: u8, _transfer_id: String, _project_id: String) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(transfer_id: String, general_bump: u8)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, seeds = [TRANSFER_SEED, transfer_id.as_bytes()[..18].as_ref(), transfer_id.as_bytes()[18..].as_ref()], bump, space = 400)]
    pub base_account: Account<'info, TransferParameter>,
    #[account(mut, seeds = [GENERAL_SEED], bump = general_bump, seeds::program = general_program.key())]
    pub general_account: Account<'info, GeneralParameter>,
    #[account(
        init, payer = authority,
        seeds = [POOL_SEED, transfer_id.as_bytes()[..18].as_ref(), transfer_id.as_bytes()[18..].as_ref()],
        bump,
        token::mint=token_mint,
        token::authority=base_account,
    )]
    pub project_pool_wallet: Account<'info, TokenAccount>,
    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub wallet_to_withdraw_from: Account<'info, TokenAccount>,
    pub general_program: Program<'info, General>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(base_bump: u8, general_bump: u8, project_bump: u8, transfer_id: String, project_id: String)]
pub struct SignTransfer<'info> {
    #[account(mut, seeds = [TRANSFER_SEED, transfer_id.as_bytes()[..18].as_ref(), transfer_id.as_bytes()[18..].as_ref()], bump = base_bump)]
    pub base_account: Account<'info, TransferParameter>,
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = project_bump, seeds::program = project_program.key())]
    pub project_account: Account<'info, ProjectParameter>,
    #[account(mut, seeds = [GENERAL_SEED], bump = general_bump, seeds::program = project_program.key())]
    pub general_account: Account<'info, GeneralParameter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub general_program: Program<'info, General>,
    pub project_program: Program<'info, Project>,
}

#[account]
pub struct TransferParameter {
    pub authority: Pubkey, // 32
    pub amount: u32, // 4
    pub signers: Vec<Pubkey>, // 32 * 10
    pub receiver: Pubkey, // 32
    pub state: bool // 1
}
