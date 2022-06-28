use anchor_lang::prelude::*;

declare_id!("45GpJwQe42EXn8EQoyBnp5dU51h2BWrkv8ASmGKERpKD");

const PROJECT_SEED: &'static [u8] = b"project";

#[program]
pub mod project {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, _project_id: String) -> Result<()> {

        let parameters = &mut ctx.accounts.base_account;
        parameters.authority = ctx.accounts.admin.key();
        Ok(())
    }

    pub fn add_initial_signatories(ctx: Context<AddInitialSignatories>, _base_bump: u8, _project_id: String, signatures: Vec<Pubkey>, threshold: u32, time_limit: u32) -> Result<()> {

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
            };
            parameters.signatories.push(sig);
        }
        Ok(())
    }

    pub fn add_new_signatory_proposal(ctx: Context<Proposal>, _base_bump: u8,_project_id: String, signatory: Pubkey) -> Result<()> {

        let parameters = &mut ctx.accounts.base_account;

        if parameters.add.status == true {
            let current_timestamp = Clock::get().unwrap().unix_timestamp;
            if (current_timestamp - parameters.add.timestamp) > parameters.time_limit.into() {
                parameters.create_add(signatory);
            }
            else{
                return Err(error!(ErrorCode::ProposalInProgress));
            }
        }
        else{
            parameters.create_add(signatory);
        }

        Ok(())
    }

    pub fn remove_signatory_proposal(ctx: Context<Proposal>, _base_bump: u8,_project_id: String, signatory: Pubkey) -> Result<()> {

        let parameters = &mut ctx.accounts.base_account;

        let index = parameters.get_index(signatory);

        if parameters.delete.status == true {
            let current_timestamp = Clock::get().unwrap().unix_timestamp;
            if (current_timestamp - parameters.delete.timestamp) > parameters.time_limit.into() {
                if index == usize::MAX{
                    return Err(error!(ErrorCode::SignatoryNotFound));
                }
                parameters.create_delete(signatory);
            }
            else{
                return Err(error!(ErrorCode::ProposalInProgress));
            }
        }
        else {
            if index == usize::MAX{
                return Err(error!(ErrorCode::SignatoryNotFound));
            }
            parameters.create_delete(signatory);
        }

        Ok(())
    }

    pub fn change_threshold_proposal(ctx: Context<Proposal>, _base_bump: u8, _project_id: String, threshold: u32) -> Result<()> {

        let parameters = &mut ctx.accounts.base_account;

        if parameters.change_threshold.status == true {
            let current_timestamp = Clock::get().unwrap().unix_timestamp;
            if (current_timestamp - parameters.delete.timestamp) > parameters.time_limit.into() {
                parameters.create_change(threshold);
            }
            else{
                return Err(error!(ErrorCode::ProposalInProgress)); 
            }
        }
        else {
            parameters.create_change(threshold);    
        }

        Ok(())
    }

    pub fn sign_proposal(ctx: Context<SignProposal>, _base_bump: u8, _project_id: String, key: String) -> Result<()> {

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
                            let sig = Signature {
                                key: parameters.add.new_signatory,
                                add: false,
                                delete: false,
                                change_threshold: false,
                            };
                            parameters.signatories.push(sig);
                            parameters.reset_add();
                        }
                    }
                    else {
                        return Err(error!(ErrorCode::RepeatedSignature));
                    }
                }
                else{
                    return Err(error!(ErrorCode::NoProposalCreated))
                }
            },
            "delete" => {
                if parameters.delete.status == true {
                    if parameters.signatories[final_index].delete == false {
                        parameters.signatories[final_index].delete = true;
                        parameters.delete.votes += 1;

                        let mut index = usize::MAX;

                        if parameters.delete.votes >= parameters.threshold {
                            for i in 0..parameters.signatories.len() {
                                if parameters.signatories[i].key == parameters.delete.old_signatory{
                                    index = i;
                                }
                            }
                            if index == usize::MAX {
                                return Err(error!(ErrorCode::SignatoryNotFound))
                            }
                            parameters.signatories.remove(index);
                            if parameters.threshold > parameters.signatories.len().try_into().unwrap() {
                                parameters.threshold = parameters.signatories.len().try_into().unwrap();
                            }
                            parameters.reset_delete();
                        }
                    }
                    else {
                        return Err(error!(ErrorCode::RepeatedSignature));
                    }
                }
                else{
                    return Err(error!(ErrorCode::NoProposalCreated))
                }
            }
            "change" => {
                if parameters.change_threshold.status == true {
                    if parameters.signatories[final_index].change_threshold == false {
                        parameters.signatories[final_index].change_threshold = true;
                        parameters.change_threshold.votes += 1;

                        if parameters.change_threshold.votes >= parameters.threshold {
                            parameters.threshold = parameters.change_threshold.new_threshold;
                            parameters.reset_change();
                        }
                    }
                    else {
                        return Err(error!(ErrorCode::RepeatedSignature));
                    }
                }
                else{
                    return Err(error!(ErrorCode::NoProposalCreated))
                } 
            },
            _ => msg!("Wrong proposal")
        }

        Ok(())
    }

}

#[derive(Accounts)]
#[instruction(project_id: String)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump, space = 1000)]
    pub base_account: Account<'info, ProjectParameter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: 
    pub admin: AccountInfo<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(base_bump: u8, project_id: String)]       
pub struct AddInitialSignatories<'info> {
    #[account(mut, seeds = [PROJECT_SEED, project_id.as_bytes()[..18].as_ref(), project_id.as_bytes()[18..].as_ref()], bump = base_bump)]
    pub base_account: Account<'info, ProjectParameter>,
    // #[account(mut)]
    // pub authority: Signer<'info>,
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

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct Signature {
    pub key: Pubkey, // 32
    pub add: bool, // 1
    pub delete: bool, // 1
    pub change_threshold: bool, // 1
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct AddSignatory {
    pub status: bool, // 1
    pub new_signatory: Pubkey, // 32
    pub timestamp: i64, // 8
    pub votes: u32 // 4
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct DeleteSignatory {
    pub status: bool, // 1
    pub old_signatory: Pubkey, // 32
    pub timestamp: i64, // 8
    pub votes: u32 // 4
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct ChangeThreshold {
    pub status: bool, // 1
    pub new_threshold: u32, // 4
    pub timestamp: i64, // 8
    pub votes: u32 // 4
}

#[account]
pub struct ProjectParameter {
    pub authority: Pubkey, // 32 
    pub signatories: Vec<Signature>, // 37 * n
    pub add: AddSignatory, // 41
    pub delete: DeleteSignatory, // 41
    pub change_threshold: ChangeThreshold, // 13
    pub threshold: u32, // 4
    pub time_limit: u32 // 4
}

impl ProjectParameter {
    pub fn get_index(&self, key: Pubkey) -> usize {

        let mut index: usize = usize::MAX;

        for i in 0..self.signatories.len(){
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

        for i in 0..self.signatories.len() {
            self.signatories[i].add = false;
        }
    }
    pub fn create_add(&mut self, signatory: Pubkey) {
        self.add.status = true;
        self.add.new_signatory = signatory;
        self.add.timestamp = Clock::get().unwrap().unix_timestamp;
        self.add.votes = 0;
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
    }
    
    pub fn reset_change(&mut self) {
        self.change_threshold.status = false;
        self.change_threshold.new_threshold = 0;
        self.change_threshold.timestamp = 0;
        self.change_threshold.votes = 0;

        for i in 0..self.signatories.len() {
            self.signatories[i].change_threshold = false;
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
}