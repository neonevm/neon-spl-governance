use {
    crate::{
        client::Client,
        governance::Governance,
        token_owner::TokenOwner,
    },
    borsh::BorshDeserialize,
    solana_sdk::{
        pubkey::Pubkey,
        instruction::{AccountMeta, Instruction},
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
        signature::Signature,
        program_error::ProgramError,
    },
    spl_governance::{
        state::{
            enums::{ProposalState, TransactionExecutionStatus},
            vote_record::{Vote, VoteChoice},
            proposal::{ProposalV2, VoteType, get_proposal_address},
            proposal_transaction::{ProposalTransactionV2, InstructionData, get_proposal_transaction_address},
        },
        instruction::{
            cast_vote,
            sign_off_proposal,
            insert_transaction,
            remove_transaction,
            execute_transaction,
            create_proposal,
        },
    },
    solana_client::{
        client_error::{ClientError, Result as ClientResult},
        rpc_config::RpcSendTransactionConfig,
    },
};

#[derive(Debug)]
pub struct Proposal<'a> {
    pub governance: &'a Governance<'a>,
    pub proposal_index: u32,
    pub address: Pubkey,
}

impl<'a> Proposal<'a> {
    fn get_client(&self) -> &Client<'a> {self.governance.get_client()}

    pub fn get_data(&self) -> ClientResult<Option<ProposalV2>> {
        self.governance.realm.client.get_account_data::<ProposalV2>(&self.governance.realm.program_id, &self.address)
    }

    pub fn get_state(&self) -> Result<ProposalState,ClientError> {
        self.get_data().map(|v| v.unwrap().state)
    }

    pub fn create_proposal(&self, create_authority: &Keypair, token_owner: &TokenOwner, proposal_name: &str, proposal_description: &str) -> ClientResult<Signature> {
        self.governance.realm.client.send_and_confirm_transaction(
                &[
                    create_proposal(
                        &self.governance.realm.program_id,
                        &self.governance.address,
                        &token_owner.token_owner_record_address,
                        &create_authority.pubkey(),
                        &self.governance.realm.client.payer.pubkey(),
                        token_owner.voter_weight_record_address,
    
                        &self.governance.realm.address,
                        proposal_name.to_string(),
                        proposal_description.to_string(),
                        &self.governance.realm.community_mint,
                        VoteType::SingleChoice,
                        vec!["Yes".to_string()],
                        true,
                        self.proposal_index,
                    )
                ],
                &[create_authority],
            )
    }

    pub fn get_proposal_transaction_address(&self, option_index: u8, index: u16) -> Pubkey {
        get_proposal_transaction_address(
                &self.governance.realm.program_id,
                &self.address,
                &option_index.to_le_bytes(),
                &index.to_le_bytes())
    }

    pub fn get_proposal_transaction_data(&self, option_index: u8, index: u16) -> ClientResult<Option<ProposalTransactionV2>> {
        let transaction_pubkey: Pubkey = self.get_proposal_transaction_address(option_index, index);
        self.governance.realm.client.get_account_data::<ProposalTransactionV2>(
                &self.governance.realm.program_id,
                &self.get_proposal_transaction_address(option_index, index))
    }

//    pub fn finalize_vote(&self, sign_authority: &Keypair, token_owner: &TokenOwner) -> ClientResult<Signature>;

    pub fn insert_transaction_instruction(&self, authority: &Pubkey, token_owner: &TokenOwner, option_index: u8, index: u16, hold_up_time: u32, instructions: Vec<InstructionData>) -> Instruction {
        insert_transaction(
            &self.governance.realm.program_id,
            &self.governance.address,
            &self.address,
            &token_owner.token_owner_record_address,
            authority,
            &self.governance.realm.client.payer.pubkey(),

            option_index,
            index,
            hold_up_time,
            instructions,
        )
    }

    pub fn insert_transaction(&self, authority: &Keypair, token_owner: &TokenOwner, option_index: u8, index: u16, hold_up_time: u32, instructions: Vec<InstructionData>) -> ClientResult<Signature> {
        self.governance.realm.client.send_and_confirm_transaction(
                &[
                    self.insert_transaction_instruction(
                        &authority.pubkey(),
                        token_owner,
                        option_index,
                        index,
                        hold_up_time,
                        instructions,
                    ),
                ],
                &[authority],
            )
    }

    pub fn remove_transaction(&self, authority: &Keypair, token_owner: &TokenOwner, option_index: u8, index: u16, beneficiary: &Pubkey) -> ClientResult<Signature> {
        self.governance.realm.client.send_and_confirm_transaction(
                &[
                    remove_transaction(
                        &self.governance.realm.program_id,
                        &self.address,
                        &token_owner.token_owner_record_address,
                        &authority.pubkey(),
                        &self.get_proposal_transaction_address(option_index, index),
                        beneficiary,
                    ),
                ],
                &[authority],
            )
    }

    pub fn execute_transactions(&self, option_index: u8) -> ClientResult<Vec<Signature>> {
        let mut signatures = vec!();
        let mut index = 0;

        while let Some(proposal_transaction) = self.get_proposal_transaction_data(option_index, index)? {
            if proposal_transaction.execution_status == TransactionExecutionStatus::None {
                println!("Execute proposal transaction: {} {} =====================", option_index, index);
                signatures.push(self._execute_transaction(&proposal_transaction)?);
            }
            index += 1;
        }
        Ok(signatures)
    }

    pub fn execute_transaction(&self, option_index: u8, index: u16) -> ClientResult<Signature> {
        let proposal_transaction = self.get_proposal_transaction_data(option_index, index)?.unwrap();
        self._execute_transaction(&proposal_transaction)
    }

    fn _execute_transaction(&self, proposal_transaction: &ProposalTransactionV2) -> ClientResult<Signature> {
        let payer = self.get_client().payer;
        println!("Proposal transaction: {:?}", proposal_transaction);
        let mut accounts = vec!();
        for instruction in &proposal_transaction.instructions {
            accounts.push(AccountMeta::new_readonly(instruction.program_id, false));
            accounts.extend(instruction.accounts.iter()
                    .map(|a| if a.is_writable {
                         AccountMeta::new(a.pubkey, a.is_signer && a.pubkey != self.governance.address)
                     } else {
                         AccountMeta::new_readonly(a.pubkey, a.is_signer && a.pubkey != self.governance.address)
                     }));
        }

        println!("Governance: {}", self.governance.address);
        println!("Proposal: {}", self.address);
        println!("Execute transaction with accounts {:?}", accounts);

        self.governance.realm.client.send_and_confirm_transaction_with_payer_only(
                &[
                    execute_transaction(
                        &self.governance.realm.program_id,
                        &self.governance.address,
                        &self.address,
                        &self.get_proposal_transaction_address(
                                proposal_transaction.option_index,
                                proposal_transaction.transaction_index),
                        &self.governance.address,   // Dummy account to call execute_transaction (bug in instruction.rs implementation)
                        &accounts,
                    ),
                ]
            )
    }

    pub fn sign_off_proposal(&self, sign_authority: &Keypair, token_owner: &TokenOwner) -> ClientResult<Signature> {
        self.governance.realm.client.send_and_confirm_transaction(
                &[
                    sign_off_proposal(
                        &self.governance.realm.program_id,
                        &self.governance.realm.address,
                        &self.governance.address,
                        &self.address,
                        &sign_authority.pubkey(),
                        Some(&token_owner.token_owner_record_address),
                    ),
                ],
                &[sign_authority],
            )
    }

    pub fn cast_vote(&self, proposal_owner: &TokenOwner, voter_authority: &Keypair, voter: &TokenOwner, vote_yes_no: bool) -> ClientResult<Signature> {
        let payer = self.get_client().payer;

        let vote: Vote =
            if vote_yes_no {
                Vote::Approve(vec![
                    VoteChoice {
                        rank: 0,
                        weight_percentage: 100,
                    }
                ])
            } else {
                Vote::Deny
            };
        
        self.governance.realm.client.send_and_confirm_transaction(
                &[
                    cast_vote(
                        &self.governance.realm.program_id,
                        &self.governance.realm.address,
                        &self.governance.address,
                        &self.address,
                        &proposal_owner.token_owner_record_address,
                        &voter.token_owner_record_address,
                        &voter_authority.pubkey(),
                        &self.governance.realm.community_mint,
                        &payer.pubkey(),
                        voter.voter_weight_record_address,
                        self.governance.realm.settings().max_voter_weight_record_address,
                        vote,
                    ),
                ],
                &[voter_authority],
            )
    }
}
