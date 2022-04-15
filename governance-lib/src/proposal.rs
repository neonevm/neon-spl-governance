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
            proposal::{ProposalV2},
            proposal_transaction::{ProposalTransactionV2, InstructionData, get_proposal_transaction_address},
        },
        instruction::{
            cast_vote,
            sign_off_proposal,
            insert_transaction,
            remove_transaction,
            execute_transaction,
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
    pub address: Pubkey,
    pub token_owner_record: Pubkey,
    pub data: ProposalV2,
}

impl<'a> Proposal<'a> {
    fn get_client(&self) -> &Client<'a> {self.governance.get_client()}

    pub fn get_state(&self) -> Result<ProposalState,ClientError> {
        let data = self.governance.get_proposal_v2(self.address);
        Ok(data.state)
    }

    pub fn get_proposal_transaction_address(&self, option_index: u8, index: u16) -> Pubkey {
        get_proposal_transaction_address(
                &self.get_client().spl_governance_program_address,
                &self.address,
                &option_index.to_le_bytes(),
                &index.to_le_bytes())
    }

    pub fn get_proposal_transaction_data(&self, option_index: u8, index: u16) -> ClientResult<Option<ProposalTransactionV2>> {
        let transaction_pubkey: Pubkey = self.get_proposal_transaction_address(option_index, index);

        //let mut dt: &[u8] = &self.get_client().solana_client.get_account_data(&transaction_pubkey)?;
        //Ok(ProposalTransactionV2::deserialize(&mut dt).unwrap())
        self.get_client().get_account_data::<ProposalTransactionV2>(
                &self.get_client().spl_governance_program_address,
                &self.get_proposal_transaction_address(option_index, index))
    }

//    pub fn finalize_vote(&self, sign_authority: &Keypair, token_owner: &TokenOwner) -> ClientResult<Signature>;

    pub fn insert_transaction_instruction(&self, authority: &Pubkey, option_index: u8, index: u16, hold_up_time: u32, instructions: Vec<InstructionData>) -> Result<Instruction,ProgramError> {
        Ok(
            insert_transaction(
                &self.get_client().spl_governance_program_address,
                &self.governance.address,
                &self.address,
                &self.token_owner_record,
                authority,
                &self.get_client().payer.pubkey(),
    
                option_index,
                index,
                hold_up_time,
                instructions,
            )
        )
    }

    pub fn insert_transaction(&self, authority: &Keypair, option_index: u8, index: u16, hold_up_time: u32, instructions: Vec<InstructionData>) -> ClientResult<Signature> {
        let payer = self.get_client().payer;

        let transaction: Transaction = Transaction::new_signed_with_payer(
            &[
                insert_transaction(
                    &self.get_client().spl_governance_program_address,
                    &self.governance.address,
                    &self.address,
                    &self.token_owner_record,
                    &authority.pubkey(),
                    &payer.pubkey(),
    
                    option_index,
                    index,
                    hold_up_time,
                    instructions,
                ),
            ],
            Some(&payer.pubkey()),
            &[
                payer,
                authority,
            ],
            self.get_client().solana_client.get_latest_blockhash().unwrap(),
        );

        self.get_client().solana_client.send_and_confirm_transaction(&transaction)
    }

    pub fn remove_transaction(&self, authority: &Keypair, option_index: u8, index: u16, beneficiary: &Pubkey) -> ClientResult<Signature> {
        let payer = self.get_client().payer;

        let transaction: Transaction = Transaction::new_signed_with_payer(
            &[
                remove_transaction(
                    &self.get_client().spl_governance_program_address,
                    &self.address,
                    &self.token_owner_record,
                    &authority.pubkey(),
                    &self.get_proposal_transaction_address(option_index, index),
                    beneficiary,
                ),
            ],
            Some(&payer.pubkey()),
            &[
                payer,
                authority,
            ],
            self.get_client().solana_client.get_latest_blockhash().unwrap(),
        );

        self.get_client().solana_client.send_and_confirm_transaction(&transaction)
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

        let transaction: Transaction = Transaction::new_signed_with_payer(
            &[
                execute_transaction(
                    &self.get_client().spl_governance_program_address,
                    &self.governance.address,
                    &self.address,
                    &self.get_proposal_transaction_address(
                            proposal_transaction.option_index,
                            proposal_transaction.transaction_index),
                    &self.governance.address,   // Dummy account to call execute_transaction (bug in instruction.rs implementation)
                    &accounts,
                ),
            ],
            Some(&payer.pubkey()),
            &[
                payer,
            ],
            self.get_client().solana_client.get_latest_blockhash().unwrap(),
        );

        self.get_client().solana_client.send_and_confirm_transaction(&transaction)
    }

    pub fn sign_off_proposal(&self, sign_authority: &Keypair, token_owner: &TokenOwner) -> ClientResult<Signature> {
        let payer = self.get_client().payer;

        let sign_off_proposal_instruction: Instruction =
            sign_off_proposal(
                &self.get_client().spl_governance_program_address,
                &self.governance.realm.address,
                &self.governance.address,
                &self.address,
                &sign_authority.pubkey(),
                Some(&token_owner.token_owner_record_address),
            );
        
        let transaction: Transaction =
            Transaction::new_signed_with_payer(
                &[
                    sign_off_proposal_instruction,
                ],
                Some(&payer.pubkey()),
                &[
                    payer,
                    sign_authority,
                ],
                self.get_client().solana_client.get_latest_blockhash().unwrap(),
            );
        
        self.get_client().solana_client.send_and_confirm_transaction(&transaction)
    }

    pub fn cast_vote(&self, voter_authority: &Keypair, voter: &TokenOwner, vote_yes_no: bool) -> ClientResult<Signature> {
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
        
        let cast_vote_instruction: Instruction =
            cast_vote(
                &self.get_client().spl_governance_program_address,
                &self.governance.realm.address,
                &self.governance.address,
                &self.address,
                &self.token_owner_record,
                &voter.token_owner_record_address,
                &voter_authority.pubkey(),
                &self.governance.realm.community_mint,
                &payer.pubkey(),
                voter.voter_weight_record_address,
                self.governance.realm.settings().max_voter_weight_record_address,
                vote,
            );
        
        let transaction: Transaction =
            Transaction::new_signed_with_payer(
                &[
                    cast_vote_instruction,
                ],
                Some(&payer.pubkey()),
                &[
                    payer, voter_authority,
                ],
                self.get_client().solana_client.get_latest_blockhash().unwrap(),
            );
        
        self.get_client().solana_client.send_and_confirm_transaction(&transaction)
    }
}
