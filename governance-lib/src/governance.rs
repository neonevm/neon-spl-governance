use {
    crate::{
        client::SplGovernanceInteractor,
        realm::Realm,
        token_owner::TokenOwner,
        proposal::Proposal,
    },
    borsh::BorshDeserialize,
    solana_sdk::{
        pubkey::Pubkey,
        instruction::Instruction,
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
    },
    spl_governance::{
        state::{
            governance::{GovernanceConfig, GovernanceV2},
            proposal::{VoteType, ProposalV2, get_proposal_address},
        },
        instruction::{create_proposal, set_governance_config},
    },
    solana_client::{
        client_error::ClientError,
    },
};

#[derive(Debug)]
pub struct Governance<'a> {
    pub realm: &'a Realm<'a>,
    pub address: Pubkey,
    pub data: GovernanceV2,
}

impl<'a> Governance<'a> {
    pub fn get_interactor(&self) -> &SplGovernanceInteractor<'a> {self.realm.interactor}

    pub fn get_proposals_count(&self) -> u32 {
        let data: GovernanceV2 = self.get_interactor().get_account_data(
                &self.get_interactor().spl_governance_program_address,
                &self.address).unwrap().unwrap();
        data.proposals_count
    }

    pub fn get_proposal_address(&self, proposal_index: u32) -> Pubkey {
        get_proposal_address(
                &self.get_interactor().spl_governance_program_address,
                &self.address,
                &self.realm.community_mint,
                &proposal_index.to_le_bytes())
    }

    pub fn get_proposal_v2(&self, proposal_pubkey: Pubkey) -> ProposalV2 {
        let mut dt: &[u8] = &self.get_interactor().solana_client.get_account_data(&proposal_pubkey).unwrap();
        ProposalV2::deserialize(&mut dt).unwrap()
    }

    // Note: Only governance PDA via a proposal can authorize change to its own config
    pub fn set_governance_config_instruction(&self, config: GovernanceConfig) -> Instruction {
        set_governance_config(
                &self.get_interactor().spl_governance_program_address,
                &self.address,
                config)
    }

    pub fn create_proposal<'b:'a>(&'b self, create_authority: &Keypair, token_owner: &TokenOwner, proposal_name: &str, proposal_description: &str, proposal_index: u32) -> Result<Proposal<'a>,ClientError> {
        let proposal_address: Pubkey = self.get_proposal_address(proposal_index);
        let payer = &self.get_interactor().payer;

        if !self.get_interactor().account_exists(&proposal_address) {
            let create_proposal_instruction: Instruction =
                create_proposal(
                    &self.get_interactor().spl_governance_program_address,
                    &self.address,
                    &token_owner.token_owner_record_address,
                    &create_authority.pubkey(),
                    &payer.pubkey(),
                    token_owner.voter_weight_record_address,

                    &self.realm.address,
                    proposal_name.to_string(),
                    proposal_description.to_string(),
                    &self.realm.community_mint,
                    VoteType::SingleChoice,
                    vec!["Yes".to_string()],
                    true,
                    proposal_index,
                );

            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_proposal_instruction,
                    ],
                    Some(&payer.pubkey()),
                    &[
                        create_authority,
                        payer,
                    ],
                    self.get_interactor().solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.get_interactor().solana_client.send_and_confirm_transaction(&transaction)?;
        }
        Ok(
            Proposal {
                governance: self,
                address: proposal_address,
                token_owner_record: token_owner.token_owner_record_address,
                data: self.get_proposal_v2(proposal_address),
            }
        )
    }

}

