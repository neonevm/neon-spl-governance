use {
    crate::{
        client::Client,
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
        signature::Signature,
    },
    spl_governance::{
        state::{
            governance::{GovernanceConfig, GovernanceV2},
            proposal::{VoteType, ProposalV2, get_proposal_address},
        },
        instruction::{
            create_governance,
            create_mint_governance,
            create_proposal,
            set_governance_config
        },
    },
    solana_client::{
        client_error::{ClientError, Result as ClientResult},
    },
};

#[derive(Debug)]
pub struct Governance<'a> {
    pub realm: &'a Realm<'a>,
    pub address: Pubkey,
    pub governed_account: Pubkey,
}

impl<'a> Governance<'a> {
    pub fn get_client(&self) -> &Client<'a> {self.realm.client}

    pub fn get_data(&self) -> ClientResult<Option<GovernanceV2>> {
        self.realm.client.get_account_data::<GovernanceV2>(&self.realm.program_id, &self.address)
    }

    pub fn get_proposals_count(&self) -> u32 {
        self.get_data().unwrap().unwrap().proposals_count
    }

    pub fn create_governance(&self, create_authority: &Keypair, token_owner: &TokenOwner,
            gov_config: GovernanceConfig) -> ClientResult<Signature> {
        self.realm.client.send_and_confirm_transaction(
                &[
                    create_governance(
                        &self.realm.program_id,
                        &self.realm.address,
                        Some(&self.governed_account),
                        &token_owner.token_owner_record_address,
                        &self.realm.client.payer.pubkey(),
                        &create_authority.pubkey(),       // realm_authority OR token_owner authority
                        token_owner.voter_weight_record_address,
                        gov_config,
                    ),
                ],
                &[create_authority]
            )
    }

    pub fn create_mint_governance(&self, create_authority: &Keypair, token_owner: &TokenOwner,
            governed_mint_authority: &Keypair, gov_config: GovernanceConfig, transfer_mint_authorities: bool) -> ClientResult<Signature> {
        self.realm.client.send_and_confirm_transaction(
                &[
                    create_mint_governance(
                        &self.realm.program_id,
                        &self.realm.address,
                        &self.governed_account,
                        &governed_mint_authority.pubkey(),
                        &token_owner.token_owner_record_address,
                        &self.realm.client.payer.pubkey(),
                        &create_authority.pubkey(),       // realm_authority OR token_owner authority
                        token_owner.voter_weight_record_address,
                        gov_config,
                        transfer_mint_authorities,
                    ),
                ],
                &[create_authority]
            )
    }

    // Note: Only governance PDA via a proposal can authorize change to its own config
    pub fn set_governance_config_instruction(&self, config: GovernanceConfig) -> Instruction {
        set_governance_config(
                &self.realm.program_id,
                &self.address,
                config)
    }

    pub fn proposal<'b:'a>(&'b self, proposal_index: u32) -> Proposal {
        let proposal_address: Pubkey = get_proposal_address(
                &self.realm.program_id,
                &self.address,
                &self.realm.community_mint,
                &proposal_index.to_le_bytes()
            );
        Proposal {
            governance: self,
            proposal_index,
            address: proposal_address,
        }
    }
}

