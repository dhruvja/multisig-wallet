use std::vec;

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};
use general::program::General;
use general::{self, GeneralParameter};

declare_id!("45GpJwQe42EXn8EQoyBnp5dU51h2BWrkv8ASmGKERpKD");

const PROJECT_SEED: &'static [u8] = b"project";
const POOL_SEED: &'static [u8] = b"pool";
const GENERAL_SEED: &'static [u8] = b"general";

#[program]
pub mod project {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        _project_id: String,
        percent_transfer: u8,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;
        parameters.authority = ctx.accounts.admin.key();
        parameters.last_tx = Clock::get().unwrap().unix_timestamp as i32;
        parameters.percent_transfer = percent_transfer;
        parameters.threshold = 1;

        let sig = Signature {
            key: ctx.accounts.authority.key(),
            add: false,
            delete: false,
            change_threshold: false,
            change_time_limit: false,
            transfer_amount: false,
        };
        parameters.signatories.push(sig);

        Ok(())
    }

    pub fn add_initial_signatories(
        ctx: Context<AddInitialSignatories>,
        _base_bump: u8,
        _project_id: String,
        signatures: Vec<Pubkey>,
        threshold: u32,
        time_limit: u32,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;

        msg!(&signatures[0].to_string());
        msg!(&signatures[1].to_string());
        // msg!(&signatures.to_string());

        parameters.threshold = threshold;
        parameters.time_limit = time_limit;

        for i in 0..signatures.len() {
            let sig = Signature {
                key: signatures[i],
                add: false,
                delete: false,
                change_threshold: false,
                change_time_limit: false,
                transfer_amount: false,
            };
            parameters.signatories.push(sig);
        }
        parameters.last_tx = Clock::get().unwrap().unix_timestamp as i32;
        Ok(())
    }

    pub fn add_new_signatory_proposal(
        ctx: Context<Proposal>,
        _base_bump: u8,
        _project_id: String,
        signatory: Vec<Pubkey>,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;

        if parameters.add.status == true {
            let current_timestamp = Clock::get().unwrap().unix_timestamp;
            if (current_timestamp - parameters.add.timestamp) > parameters.time_limit.into() {
                parameters.create_add(signatory);
            } else {
                return Err(error!(ErrorCode::ProposalInProgress));
            }
        } else {
            parameters.create_add(signatory);
        }
        Ok(())
    }

    pub fn remove_signatory_proposal(
        ctx: Context<Proposal>,
        _base_bump: u8,
        _project_id: String,
        signatory: Pubkey,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;

        let index = parameters.get_index(signatory);

        if parameters.delete.status == true {
            let current_timestamp = Clock::get().unwrap().unix_timestamp;
            if (current_timestamp - parameters.delete.timestamp) > parameters.time_limit.into() {
                if index == usize::MAX {
                    return Err(error!(ErrorCode::SignatoryNotFound));
                }
                parameters.create_delete(signatory);
            } else {
                return Err(error!(ErrorCode::ProposalInProgress));
            }
        } else {
            if index == usize::MAX {
                return Err(error!(ErrorCode::SignatoryNotFound));
            }
            parameters.create_delete(signatory);
        }
        Ok(())
    }

    pub fn change_threshold_proposal(
        ctx: Context<Proposal>,
        _base_bump: u8,
        _project_id: String,
        threshold: u32,
        current_timestamp: u32,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;

        let day = 60 * 60 * 24;
        // let current_timestamp = Clock::get().unwrap().unix_timestamp;

        if (current_timestamp as i32 - parameters.last_tx) / day >= 90 {
            msg!("reduce the approvals");

            if parameters.approval < parameters.threshold {
                let mut months =
                    (current_timestamp as i32 - parameters.last_reduced_threshold) / day;
                months = months / 30;
                if months < 1 {
                    return Err(error!(ErrorCode::MinimumTimeNotPassed));
                } else {
                    if parameters.approval - (months as u32) > 1 {
                        parameters.approval -= months as u32;
                        parameters.last_reduced_threshold = current_timestamp as i32;
                        parameters.reduce_approval(threshold);
                    } else {
                        parameters.approval = 1;
                        parameters.last_reduced_threshold = current_timestamp as i32;
                        parameters.reduce_approval(threshold);
                    }
                }
            } else {
                let mut months = (current_timestamp as i32 - parameters.last_tx) / day;
                months = ((months - 90) / 30) + 1;
                if parameters.approval - (months as u32) > 1 {
                    parameters.approval -= months as u32;
                    parameters.last_reduced_threshold = current_timestamp as i32;
                } else {
                    parameters.approval = 1;
                    parameters.last_reduced_threshold = current_timestamp as i32;
                }
            }
        } else {
            if parameters.change_threshold.status == true {
                if (current_timestamp as i64 - parameters.change_threshold.timestamp)
                    > parameters.time_limit.into()
                {
                    parameters.create_change(threshold);
                } else {
                    return Err(error!(ErrorCode::ProposalInProgress));
                }
            } else {
                parameters.create_change(threshold);
            }
        }

        Ok(())
    }

    pub fn change_time_limit_proposal(
        ctx: Context<Proposal>,
        _base_bump: u8,
        _project_id: String,
        time_limit: u32,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;

        if parameters.change_time_limit.status == true {
            let current_timestamp = Clock::get().unwrap().unix_timestamp;
            if (current_timestamp - parameters.change_time_limit.timestamp)
                > parameters.time_limit.into()
            {
                parameters.create_time_limit(time_limit);
            } else {
                return Err(error!(ErrorCode::ProposalInProgress));
            }
        } else {
            parameters.create_time_limit(time_limit);
        }
        Ok(())
    }

    pub fn transfer_amount_proposal(
        ctx: Context<Proposal>,
        _base_bump: u8,
        _project_id: String,
        amount: u32,
        reciever: Pubkey,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;

        if parameters.transfer_amount.status == true {
            let current_timestamp = Clock::get().unwrap().unix_timestamp;
            if (current_timestamp - parameters.transfer_amount.timestamp)
                > parameters.time_limit.into()
            {
                parameters.create_transfer_amount(amount, reciever);
            } else {
                return Err(error!(ErrorCode::ProposalInProgress));
            }
        } else {
            parameters.create_transfer_amount(amount, reciever);
        }
        Ok(())
    }

    pub fn sign_proposal(
        ctx: Context<SignProposal>,
        _base_bump: u8,
        _project_id: String,
        key: String,
    ) -> Result<()> {
        let matching_key = &key[..];
        let parameters = &mut ctx.accounts.base_account;
        let final_index = parameters.get_index(ctx.accounts.authority.key());

        if final_index == usize::MAX {
            return Err(error!(ErrorCode::InvalidSigner));
        }

        match matching_key {
            "add" => {
                if parameters.add.status == true {
                    if parameters.signatories[final_index].add == false {
                        parameters.signatories[final_index].add = true;
                        parameters.add.votes += 1;

                        if parameters.add.votes >= parameters.threshold {
                            for i in 0..parameters.add.new_signatory.len() {
                                let sig = Signature {
                                    key: parameters.add.new_signatory[i],
                                    add: false,
                                    delete: false,
                                    change_threshold: false,
                                    change_time_limit: false,
                                    transfer_amount: false,
                                };
                                parameters.signatories.push(sig);
                            }
                            parameters.last_tx = Clock::get().unwrap().unix_timestamp as i32;
                            parameters.reset_add();
                        }
                    } else {
                        return Err(error!(ErrorCode::RepeatedSignature));
                    }
                } else {
                    return Err(error!(ErrorCode::NoProposalCreated));
                }
            }
            "delete" => {
                if parameters.delete.status == true {
                    if parameters.signatories[final_index].delete == false {
                        parameters.signatories[final_index].delete = true;
                        parameters.delete.votes += 1;

                        let mut index = usize::MAX;

                        if parameters.delete.votes >= parameters.threshold {
                            for i in 0..parameters.signatories.len() {
                                if parameters.signatories[i].key == parameters.delete.old_signatory
                                {
                                    index = i;
                                    break;
                                }
                            }
                            if index == usize::MAX {
                                return Err(error!(ErrorCode::SignatoryNotFound));
                            }
                            parameters.signatories.remove(index);
                            if parameters.threshold
                                > parameters.signatories.len().try_into().unwrap()
                            {
                                parameters.threshold =
                                    parameters.signatories.len().try_into().unwrap();
                            }
                            parameters.reset_delete();
                            parameters.last_tx = Clock::get().unwrap().unix_timestamp as i32;
                        }
                    } else {
                        return Err(error!(ErrorCode::RepeatedSignature));
                    }
                } else {
                    return Err(error!(ErrorCode::NoProposalCreated));
                }
            }
            "change threshold" => {
                if parameters.change_threshold.status == true {
                    if parameters.signatories[final_index].change_threshold == false {
                        parameters.signatories[final_index].change_threshold = true;
                        parameters.change_threshold.votes += 1;

                        if parameters.change_threshold.votes >= parameters.approval {
                            parameters.threshold = parameters.change_threshold.new_threshold;
                            parameters.approval = parameters.change_threshold.new_threshold;
                            parameters.last_tx = Clock::get().unwrap().unix_timestamp as i32;
                            parameters.reset_change();
                        }
                    } else {
                        return Err(error!(ErrorCode::RepeatedSignature));
                    }
                } else {
                    return Err(error!(ErrorCode::NoProposalCreated));
                }
            }
            "change time limit" => {
                if parameters.change_time_limit.status == true {
                    if parameters.signatories[final_index].change_time_limit == false {
                        parameters.signatories[final_index].change_time_limit = true;
                        parameters.change_time_limit.votes += 1;

                        if parameters.change_time_limit.votes >= parameters.threshold {
                            parameters.time_limit = parameters.change_time_limit.new_time_limit;
                            parameters.last_tx = Clock::get().unwrap().unix_timestamp as i32;
                            parameters.reset_time_limit();
                        }
                    } else {
                        return Err(error!(ErrorCode::RepeatedSignature));
                    }
                } else {
                    return Err(error!(ErrorCode::NoProposalCreated));
                }
            }
            _ => msg!("Wrong proposal"),
        }
        Ok(())
    }

    pub fn deposit_funds(
        ctx: Context<Deposit>,
        project_id: String,
        project_bump: u8,
        _pool_bump: u8,
        _general_bump: u8,
        amount: u32,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;
        let general_parameters = &mut ctx.accounts.general_account;

        if general_parameters.token_mint == ctx.accounts.token_mint.key() {
            msg!("Mint is matching");

            let bump_vector = project_bump.to_le_bytes();
            let inner = vec![
                PROJECT_SEED,
                project_id.as_bytes()[..18].as_ref(),
                project_id.as_bytes()[18..].as_ref(),
                bump_vector.as_ref(),
            ];
            let outer = vec![inner.as_slice()];

            let amount_to_transfer =
                (amount * (100 - (parameters.percent_transfer as u32)) / 100) as u64;
            let amount_to_deduct = (amount - (amount_to_transfer as u32)) as u64;

            // Below is the actual instruction that we are going to send to the Token program.
            let transfer_instruction = Transfer {
                from: ctx.accounts.wallet_to_withdraw_from.to_account_info(),
                to: ctx.accounts.project_pool_account.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_instruction,
                outer.as_slice(), //signer PDA
            );

            let another_transfer_instruction = Transfer {
                from: ctx.accounts.wallet_to_withdraw_from.to_account_info(),
                to: ctx.accounts.admin_token_wallet.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            };

            let another_cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                another_transfer_instruction,
                outer.as_slice(), //signer PDA
            );

            msg!("{} {}", amount_to_transfer, amount_to_deduct);

            // The `?` at the end will cause the function to return early in case of an error.
            // This pattern is common in Rust.
            anchor_spl::token::transfer(cpi_ctx, amount_to_transfer)?;
            anchor_spl::token::transfer(another_cpi_ctx, amount_to_deduct)?;

            parameters.staked_amount += amount_to_transfer as u32;
        }

        Ok(())
    }

    pub fn sign_transfer(
        ctx: Context<SignTransfer>,
        _general_bump: u8,
        project_bump: u8,
        _pool_bump: u8,
        project_id: String,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;

        let final_index = parameters.get_index(ctx.accounts.authority.key());

        if final_index == usize::MAX {
            return Err(error!(ErrorCode::InvalidSigner));
        }

        if parameters.transfer_amount.status == true {
            if parameters.signatories[final_index].transfer_amount == false {
                parameters.signatories[final_index].transfer_amount = true;
                parameters.transfer_amount.votes += 1;

                if parameters.transfer_amount.votes >= parameters.threshold {
                    if parameters.transfer_amount.reciever
                        != ctx.accounts.wallet_to_withdraw_from.key()
                    {
                        return Err(error!(ErrorCode::InvalidReciever));
                    } else {
                        if !parameters.shutdown && parameters.threshold == 1 {
                            return Err(error!(ErrorCode::CannotTransferDueToLowThreshold));
                        } else {
                            msg!("transfering the amount to the reciever");

                            let bump_vector = project_bump.to_le_bytes();
                            let inner = vec![
                                PROJECT_SEED,
                                project_id.as_bytes()[..18].as_ref(),
                                project_id.as_bytes()[18..].as_ref(),
                                bump_vector.as_ref(),
                            ];
                            let outer = vec![inner.as_slice()];

                            // Below is the actual instruction that we are going to send to the Token program.
                            let transfer_instruction = Transfer {
                                from: ctx.accounts.project_pool_account.to_account_info(),
                                to: ctx.accounts.wallet_to_withdraw_from.to_account_info(),
                                authority: parameters.to_account_info(),
                            };
                            let cpi_ctx = CpiContext::new_with_signer(
                                ctx.accounts.token_program.to_account_info(),
                                transfer_instruction,
                                outer.as_slice(), //signer PDA
                            );

                            let amount_in_64 = parameters.transfer_amount.amount as u64;

                            // The `?` at the end will cause the function to return early in case of an error.
                            // This pattern is common in Rust.
                            anchor_spl::token::transfer(cpi_ctx, amount_in_64)?;
                        }
                    }

                    parameters.reset_transfer_amount();
                }
            } else {
                return Err(error!(ErrorCode::RepeatedSignature));
            }
        } else {
            return Err(error!(ErrorCode::NoProposalCreated));
        }

        Ok(())
    }

    pub fn fall_back(
        ctx: Context<FallBack>,
        _base_bump: u8,
        _project_id: String,
        current_time: i32,
    ) -> Result<()> {
        let parameters = &mut ctx.accounts.base_account;

        let day: i32 = 60 * 60 * 24; // 1 day

        if !parameters.shutdown {
            if (current_time - parameters.last_tx) / day >= 90 {
                parameters.shutdown = true;
                if parameters.threshold > 1 {
                    parameters.threshold -= 1;
                    parameters.last_reduced_threshold = current_time;
                } else {
                    return Err(error!(ErrorCode::MinimumThresholdReached));
                }
            } else {
                return Err(error!(ErrorCode::ShutDownCannotBeActivated));
            }
        } else {
            if (current_time - parameters.last_reduced_threshold) / day >= 30 {
                if parameters.threshold > 1 {
                    parameters.threshold -= 1;
                    parameters.last_reduced_threshold = current_time;
                } else {
                    return Err(error!(ErrorCode::MinimumThresholdReached));
                }
            } else {
                return Err(error!(ErrorCode::MinimumTimeNotPassed));
            }
        }

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(project_id: String)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump, space = 1800)]
    pub base_account: Account<'info, ProjectParameter>,
    #[account(
        init, payer = authority,
        seeds = [POOL_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()],
        bump,
        token::mint=token_mint,
        token::authority=base_account,
    )]
    pub project_pool_account: Account<'info, TokenAccount>,
    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:
    pub admin: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(base_bump: u8, project_id: String)]
pub struct AddInitialSignatories<'info> {
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = base_bump)]
    pub base_account: Account<'info, ProjectParameter>,
    #[account(mut, constraint = authority.key() == base_account.signatories[0].key @ErrorCode::InvalidSigner)]
    pub authority: Signer<'info>,
    // pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(base_bump: u8, project_id: String)]
pub struct Proposal<'info> {
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = base_bump, has_one = authority)]
    pub base_account: Account<'info, ProjectParameter>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(base_bump: u8, project_id: String)]
pub struct SignProposal<'info> {
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = base_bump)]
    pub base_account: Account<'info, ProjectParameter>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(base_bump: u8, project_id: String)]
pub struct FallBack<'info> {
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = base_bump, has_one = authority)]
    pub base_account: Account<'info, ProjectParameter>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(project_id: String, project_bump: u8, pool_bump: u8, general_bump: u8)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = project_bump)]
    pub base_account: Account<'info, ProjectParameter>,
    #[account(mut, seeds = [GENERAL_SEED], bump = general_bump, seeds::program = general_program.key())]
    pub general_account: Account<'info, GeneralParameter>,
    #[account(
        mut,
        seeds = [POOL_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()],
        bump = pool_bump,
        token::mint=token_mint,
        token::authority=base_account,
    )]
    pub project_pool_account: Account<'info, TokenAccount>,
    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub wallet_to_withdraw_from: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub admin_token_wallet: Box<Account<'info, TokenAccount>>,
    pub general_program: Program<'info, General>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(general_bump: u8, project_bump: u8, pool_bump : u8, project_id: String)]
pub struct SignTransfer<'info> {
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = project_bump)]
    pub base_account: Account<'info, ProjectParameter>,
    #[account(mut, seeds = [GENERAL_SEED], bump = general_bump, seeds::program = general_program.key())]
    pub general_account: Account<'info, GeneralParameter>,
    #[account(
        mut,
        seeds = [POOL_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()],
        bump = pool_bump,
        token::mint=token_mint,
        token::authority=base_account,
    )]
    pub project_pool_account: Box<Account<'info, TokenAccount>>,
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

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct Signature {
    pub key: Pubkey,             // 32
    pub add: bool,               // 1
    pub delete: bool,            // 1
    pub change_threshold: bool,  // 1
    pub change_time_limit: bool, // 1
    pub transfer_amount: bool,   // 1
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct AddSignatory {
    pub status: bool,               // 1
    pub new_signatory: Vec<Pubkey>, // 32*10
    pub timestamp: i64,             // 8
    pub votes: u32,                 // 4
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct DeleteSignatory {
    pub status: bool,          // 1
    pub old_signatory: Pubkey, // 32
    pub timestamp: i64,        // 8
    pub votes: u32,            // 4
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct ChangeThreshold {
    pub status: bool,       // 1
    pub new_threshold: u32, // 4
    pub timestamp: i64,     // 8
    pub votes: u32,         // 4
}
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct ChangeTimeLimit {
    pub status: bool,        // 1
    pub new_time_limit: u32, // 4
    pub timestamp: i64,      // 8
    pub votes: u32,          // 4
}
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct TransferAmount {
    pub status: bool,     // 1
    pub amount: u32,      // 4
    pub reciever: Pubkey, // 32
    pub timestamp: i64,   // 8
    pub votes: u32,       // 4
}

#[account]
pub struct ProjectParameter {
    pub authority: Pubkey,                  // 32
    pub signatories: Vec<Signature>,        // 38 * n
    pub add: AddSignatory,                  // 41
    pub delete: DeleteSignatory,            // 41
    pub change_threshold: ChangeThreshold,  // 13
    pub change_time_limit: ChangeTimeLimit, // 13
    pub transfer_amount: TransferAmount,    // 45
    pub threshold: u32,                     // 4
    pub time_limit: u32,                    // 4
    pub last_tx: i32,                       // 4
    pub staked_amount: u32,                 // 4
    pub percent_transfer: u8,               // 1
    pub shutdown: bool,                     // 1
    pub last_reduced_threshold: i32,        //4
    pub approval: u32,                      //4
}

impl ProjectParameter {
    pub fn get_index(&self, key: Pubkey) -> usize {
        let mut index: usize = usize::MAX;

        for i in 0..self.signatories.len() {
            if self.signatories[i].key == key {
                index = i;
            }
        }

        index
    }
    pub fn reset_add(&mut self) {
        self.add.votes = 0;
        self.add.status = false;
        self.add.timestamp = 0;
        self.add.new_signatory = Vec::new();

        for i in 0..self.signatories.len() {
            self.signatories[i].add = false;
        }
    }
    pub fn create_add(&mut self, signatories: Vec<Pubkey>) {
        self.add.status = true;
        self.add.timestamp = Clock::get().unwrap().unix_timestamp;
        self.add.votes = 0;
        for i in 0..signatories.len() {
            self.add.new_signatory.push(signatories[i]);
        }
    }

    pub fn create_delete(&mut self, signatory: Pubkey) {
        self.delete.status = true;
        self.delete.old_signatory = signatory;
        self.delete.timestamp = Clock::get().unwrap().unix_timestamp;
        self.delete.votes = 0;
    }

    pub fn reset_delete(&mut self) {
        self.delete.votes = 0;
        self.delete.status = false;
        self.delete.timestamp = 0;

        for i in 0..self.signatories.len() {
            self.signatories[i].delete = false;
        }
    }

    pub fn create_change(&mut self, threshold: u32) {
        self.change_threshold.status = true;
        self.change_threshold.new_threshold = threshold;
        self.change_threshold.timestamp = Clock::get().unwrap().unix_timestamp;
        self.change_threshold.votes = 0;
        self.approval = self.threshold;
    }

    pub fn reset_change(&mut self) {
        self.change_threshold.status = false;
        self.change_threshold.new_threshold = 0;
        self.change_threshold.timestamp = 0;
        self.change_threshold.votes = 0;
        self.last_reduced_threshold = 0;
        self.shutdown = false;

        for i in 0..self.signatories.len() {
            self.signatories[i].change_threshold = false;
        }
    }

    pub fn reduce_approval(&mut self, threshold: u32) {
        self.change_threshold.status = true;
        self.change_threshold.new_threshold = threshold;
        self.change_threshold.timestamp = Clock::get().unwrap().unix_timestamp;
        self.change_threshold.votes = 0;
    }

    pub fn create_time_limit(&mut self, time_limit: u32) {
        self.change_time_limit.status = true;
        self.change_time_limit.new_time_limit = time_limit;
        self.change_time_limit.timestamp = Clock::get().unwrap().unix_timestamp;
        self.change_time_limit.votes = 0;
    }

    pub fn reset_time_limit(&mut self) {
        self.change_time_limit.status = false;
        self.change_time_limit.new_time_limit = 0;
        self.change_time_limit.timestamp = 0;
        self.change_time_limit.votes = 0;

        for i in 0..self.signatories.len() {
            self.signatories[i].change_threshold = false;
        }
    }

    pub fn create_transfer_amount(&mut self, amount: u32, reciever: Pubkey) {
        self.transfer_amount.status = true;
        self.transfer_amount.amount = amount;
        self.transfer_amount.reciever = reciever;
        self.transfer_amount.timestamp = Clock::get().unwrap().unix_timestamp;
        self.transfer_amount.votes = 0;
    }

    pub fn reset_transfer_amount(&mut self) {
        self.transfer_amount.status = false;
        self.transfer_amount.amount = 0;
        self.transfer_amount.timestamp = 0;
        self.transfer_amount.votes = 0;
        for i in 0..self.signatories.len() {
            self.signatories[i].transfer_amount = false;
        }
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Signer")]
    InvalidSigner,
    #[msg("You have already signed")]
    RepeatedSignature,
    #[msg("There is no proposal to sign")]
    NoProposalCreated,
    #[msg("There is already a proposal in progress")]
    ProposalInProgress,
    #[msg("This signatory does not exist")]
    SignatoryNotFound,
    #[msg("The project is still alive")]
    InvalidTimePeriod,
    #[msg("The threshold is 1 which cannot be reduced further")]
    MinimumThresholdReached,
    #[msg("A minimum of 90 days from the last transaction should pass until the shut down can be activated")]
    ShutDownCannotBeActivated,
    #[msg("A minimum of 30 days has to be passed to further reduce the threshold")]
    MinimumTimeNotPassed,
    #[msg("The reciever does not match")]
    InvalidReciever,
    #[msg("The transfer cannot be completed if there is only 1 signatory, add more signatories and you can complete the transfer")]
    CannotTransferDueToLowThreshold,
}
