use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};
use general::program::General;
use general::{self, GeneralParameter};
use project::program::Project;
use project::{self, ProjectParameter};

declare_id!("F5s5fXeXWgWf8L42NerMVxRHUwWsQhgM5tguedzvNwjC");

const TRANSFER_SEED: &'static [u8] = b"transfer";
const GENERAL_SEED: &'static [u8] = b"general";
const PROJECT_SEED: &'static [u8] = b"project";
const POOL_SEED: &'static [u8] = b"pool";

#[program]
pub mod transfer {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        transfer_id: String,
        project_id: String,
        _general_bump: u8,
        _pool_bump: u8,
        _project_bump: u8,
        base_bump: u8,
        amount: u32,
        receiver: Pubkey,
    ) -> Result<()> {
        // let parameters = &mut ctx.accounts.base_account;
        // let general_parameters = &mut ctx.accounts.general_account;

        // if general_parameters.token_mint == ctx.accounts.token_mint.key() {
        //     msg!("Mint is matching");

        //     let bump_vector = base_bump.to_le_bytes();
        //     let inner = vec![
        //         TRANSFER_SEED,
        //         transfer_id.as_bytes()[..18].as_ref(),
        //         transfer_id.as_bytes()[18..].as_ref(),
        //         bump_vector.as_ref(),
        //     ];
        //     let outer = vec![inner.as_slice()];

        //     // Below is the actual instruction that we are going to send to the Token program.
        //     let transfer_instruction = Transfer {
        //         from: ctx.accounts.project_pool_account.to_account_info(),
        //         to: ctx.accounts.wallet_to_withdraw_from.to_account_info(),
        //         authority: ctx.accounts.project_account.to_account_info(),
        //     };
        //     let cpi_ctx = CpiContext::new_with_signer(
        //         ctx.accounts.token_program.to_account_info(),
        //         transfer_instruction,
        //         outer.as_slice(), //signer PDA
        //     );

        //     let amount_in_32 = amount as u64;

        //     // The `?` at the end will cause the function to return early in case of an error.
        //     // This pattern is common in Rust.
        //     // anchor_spl::token::transfer(cpi_ctx, amount_in_32)?;

        //     parameters.amount = amount;
        //     parameters.receiver = receiver;
        //     parameters.authority = general_parameters.authority;
        // }

        Ok(())
    }

    pub fn update_state(
        ctx: Context<UpdateState>,
        _base_bump: u8,
        _transfer_id: String,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;

        parameters.state = true;

        Ok(())
    }

    pub fn sign_transfer(
        ctx: Context<SignTransfer>,
        base_bump: u8,
        _general_bump: u8,
        _project_bump: u8,
        _pool_bump: u8,
        transfer_id: String,
        _project_id: String,
    ) -> Result<()> {
        let project_parameters = &mut ctx.accounts.project_account;
        let transfer_parameters = &mut ctx.accounts.base_account;

        let mut index: usize = usize::MAX;

        for i in 0..project_parameters.signatories.len() {
            if project_parameters.signatories[i].key == ctx.accounts.authority.key() {
                index = i;
                break;
            }
        }

        if index == usize::MAX {
            return Err(error!(ErrorCode::InvalidSigner));
        }

        for i in 0..transfer_parameters.signers.len() {
            if transfer_parameters.signers[i] == ctx.accounts.authority.key() {
                return Err(error!(ErrorCode::RepeatedSignature));
            }
        }

        transfer_parameters
            .signers
            .push(ctx.accounts.authority.key());

        if transfer_parameters.signers.len() >= project_parameters.threshold.try_into().unwrap() {
            if transfer_parameters.receiver != ctx.accounts.wallet_to_withdraw_from.key() {
                return Err(error!(ErrorCode::InvalidReciever));
            } else {
                msg!("transfering the amount to the reciever");

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
                    from: ctx.accounts.project_pool_account.to_account_info(),
                    to: ctx.accounts.wallet_to_withdraw_from.to_account_info(),
                    authority: ctx.accounts.project_account.to_account_info(),
                };
                let cpi_ctx = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    transfer_instruction,
                    outer.as_slice(), //signer PDA
                );

                let amount_in_64 = 10;

                // The `?` at the end will cause the function to return early in case of an error.
                // This pattern is common in Rust.
                anchor_spl::token::transfer(cpi_ctx, amount_in_64)?;
            }
        }
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(transfer_id: String, project_id: String, general_bump: u8, pool_bump: u8, project_bump: u8)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, seeds = [TRANSFER_SEED, transfer_id.as_bytes()[..18].as_ref(), transfer_id.as_bytes()[18..].as_ref()], bump, space = 450)]
    pub base_account: Account<'info, TransferParameter>,
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = project_bump, seeds::program = project_program.key())]
    pub project_account: Account<'info, ProjectParameter>,
    #[account(mut, seeds = [GENERAL_SEED], bump = general_bump, seeds::program = general_program.key())]
    pub general_account: Account<'info, GeneralParameter>,
    #[account(
        mut,
        seeds = [POOL_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()],
        bump = pool_bump,
        seeds::program = project_program.key(),
        token::mint=token_mint,
        token::authority=project_account,
    )]
    pub project_pool_account: Account<'info, TokenAccount>,
    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub wallet_to_withdraw_from: Account<'info, TokenAccount>,
    pub general_program: Program<'info, General>,
    pub project_program: Program<'info, Project>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(base_bump: u8, transfer_id: String)]
pub struct UpdateState<'info> {
    #[account(mut, seeds = [TRANSFER_SEED, transfer_id.as_bytes()[..18].as_ref(), transfer_id.as_bytes()[18..].as_ref()], bump = base_bump, has_one = authority)]
    pub base_account: Account<'info, TransferParameter>,
    #[account(mut)]
    pub authority: Signer<'info>,
}
#[derive(Accounts)]
#[instruction(base_bump: u8, general_bump: u8, project_bump: u8, pool_bump : u8, transfer_id: String, project_id: String)]
pub struct SignTransfer<'info> {
    #[account(mut, seeds = [TRANSFER_SEED, transfer_id.as_bytes()[..18].as_ref(), transfer_id.as_bytes()[18..].as_ref()], bump = base_bump)]
    pub base_account: Account<'info, TransferParameter>,
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = project_bump, seeds::program = project_program.key())]
    pub project_account: Box<Account<'info, ProjectParameter>>,
    #[account(mut, seeds = [GENERAL_SEED], bump = general_bump, seeds::program = general_program.key())]
    pub general_account: Account<'info, GeneralParameter>,
    #[account(
        mut,
        seeds = [POOL_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()],
        bump = pool_bump,
        token::mint=token_mint,
        token::authority=project_account,
        seeds::program = project_program.key()
    )]
    pub project_pool_account: Account<'info, TokenAccount>,
    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub wallet_to_withdraw_from: Account<'info, TokenAccount>,
    pub general_program: Program<'info, General>,
    pub project_program: Program<'info, Project>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct TransferParameter {
    pub authority: Pubkey,    // 32
    pub amount: u32,          // 4
    pub signers: Vec<Pubkey>, // 32 * 10
    pub receiver: Pubkey,     // 32
    pub state: bool,          // 1
    pub description: String,  // 50
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Signer")]
    InvalidSigner,
    #[msg("There is no proposal to sign")]
    NoProposalCreated,
    #[msg("You have already signed")]
    RepeatedSignature,
    #[msg("Wrong account of reciever is passed")]
    InvalidReciever,
}
