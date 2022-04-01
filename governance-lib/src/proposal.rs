use {
    crate::{
        client::SplGovernanceInteractor,
        governance::Governance,
        token_owner::TokenOwner,
    },
    solana_sdk::{
        pubkey::Pubkey,
        instruction::Instruction,
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
        signature::Signature,
    },
    spl_governance::{
        state::{
            enums::ProposalState,
            vote_record::{Vote, VoteChoice},
            proposal::{ProposalV2},
        },
        instruction::{
            cast_vote,
            sign_off_proposal,
        },
    },
    solana_client::{
        client_error::{ClientError, Result as ClientResult},
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
    fn get_interactor(&self) -> &SplGovernanceInteractor<'a> {self.governance.get_interactor()}

    pub fn get_state(&self) -> Result<ProposalState,ClientError> {
        let data = self.governance.get_proposal_v2(self.address);
        Ok(data.state)
    }

    pub fn sign_off_proposal(&self, sign_authority: &Keypair, token_owner: &TokenOwner) -> ClientResult<Signature> {
        let payer = self.get_interactor().payer;

        let sign_off_proposal_instruction: Instruction =
            sign_off_proposal(
                &self.get_interactor().spl_governance_program_address,
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
                self.get_interactor().solana_client.get_latest_blockhash().unwrap(),
            );
        
        self.get_interactor().solana_client.send_and_confirm_transaction(&transaction)
    }

    pub fn cast_vote(&self, voter_authority: &Keypair, voter: &TokenOwner, vote_yes_no: bool) -> ClientResult<Signature> {
        let payer = self.get_interactor().payer;

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
                &self.get_interactor().spl_governance_program_address,
                &self.governance.realm.address,
                &self.governance.address,
                &self.address,
                &self.token_owner_record,
                &voter.token_owner_record_address,
                &voter_authority.pubkey(),
                &self.governance.realm.community_mint,
                &payer.pubkey(),
                voter.voter_weight_record_address,
                self.governance.realm.max_voter_weight_record_address,
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
                self.get_interactor().solana_client.get_latest_blockhash().unwrap(),
            );
        
        self.get_interactor().solana_client.send_and_confirm_transaction(&transaction)
    }
}
