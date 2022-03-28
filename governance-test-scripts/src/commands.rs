use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::{ Pubkey },
    instruction::{ Instruction },
    transaction::{ Transaction },
    signer::{
        Signer,
        keypair::{ Keypair },
    },
    signature::Signature,
};

use solana_client::rpc_client::RpcClient;
use solana_client::client_error::Result as ClientResult;
use solana_client::client_error::ClientError;

use borsh::{BorshDeserialize};

use spl_governance::{
    state::{
        enums::{
            // VoteThresholdPercentage,
            // VoteWeightSource,
            // VoteTipping,
            MintMaxVoteWeightSource,
        },
        governance::{
            GovernanceConfig,
            GovernanceV2,
            get_governance_address,
        },
        realm::{
            RealmV2,
            get_realm_address,
        },
        proposal::{
            VoteType,
            ProposalV2,
            get_proposal_address,
        },
        token_owner_record::{
            TokenOwnerRecordV2,
            get_token_owner_record_address,
        },
        // signatory_record::{
        //     get_signatory_record_address,
        // },
        vote_record::{
            Vote,
            VoteChoice,
        },
    },
    instruction::{
        create_realm,
        create_token_owner_record,
        create_governance,
        // set_governance_config,
        create_proposal,
        sign_off_proposal,
        add_signatory,
        cast_vote,
    }
};

use spl_governance_addin_api::{
    max_voter_weight::{
        MaxVoterWeightRecord,
    },
    voter_weight::{
        VoterWeightRecord,
    },
};

use spl_governance_addin_mock::{
    instruction::{
        setup_voter_weight_record,
        setup_max_voter_weight_record,
    }
};

use spl_governance_addin_fixed_weights::{
    instruction::{
        get_max_voter_weight_address,
    }
};

const MIN_COMMUNITY_WEIGHT_TO_CREATE_GOVERNANCE: u64 = 1;

pub struct SplGovernanceInteractor {
    solana_client: RpcClient,
    spl_governance_program_address: Pubkey,
    spl_governance_voter_weight_addin_address: Pubkey,
}

impl SplGovernanceInteractor {

    pub fn new(url: &str, program_address: Pubkey, addin_address: Pubkey) -> Self {
        SplGovernanceInteractor {
            solana_client: RpcClient::new_with_commitment(url.to_string(),CommitmentConfig::confirmed()),
            spl_governance_program_address: program_address,
            spl_governance_voter_weight_addin_address: addin_address,
        }
    }
    pub fn account_exists(&self, address: &Pubkey) -> bool {
        self.solana_client.get_account(&address).is_ok()
    }
    pub fn get_realm_address(&self, name: &str) -> Pubkey {
        get_realm_address(&self.spl_governance_program_address, name)
    }
    pub fn get_token_owner_record_address(&self, goverinig_token_owner: &Pubkey, community_mint_pubkey: &Pubkey, realm_name: &str) -> Pubkey {
        let realm_pubkey: Pubkey = self.get_realm_address(realm_name);
        get_token_owner_record_address(&self.spl_governance_program_address, &realm_pubkey, community_mint_pubkey, goverinig_token_owner)
    }
    pub fn get_governance_address(&self, realm_name: &str, governed_account_pubkey: &Pubkey) -> Pubkey {
        let realm_pubkey: Pubkey = self.get_realm_address(realm_name);
        get_governance_address(&self.spl_governance_program_address, &realm_pubkey, governed_account_pubkey)
    }
    pub fn get_proposal_address(&self, community_mint_pubkey: &Pubkey, realm_name: &str, governed_account_pubkey: &Pubkey, proposal_index: u8) -> Pubkey {
        let governance_pubkey: Pubkey = self.get_governance_address(realm_name, governed_account_pubkey);

        let proposal_index_arr: [u8; 4] = [proposal_index,0,0,0];
        get_proposal_address(&self.spl_governance_program_address, &governance_pubkey, community_mint_pubkey, &proposal_index_arr)
    }
    pub fn get_realm_v2(&self, realm_name: &str) -> Result<RealmV2,()> {
        let realm_pubkey: Pubkey = self.get_realm_address(realm_name);

        self.solana_client.get_account_data(&realm_pubkey)
            .map_err(|_|())
            .and_then(|data|{
                let mut data_slice: &[u8] = &data;
                RealmV2::deserialize(&mut data_slice).map_err(|_|())

            })
    }
    pub fn get_token_owner_record_v2(&self, goverinig_token_owner: &Pubkey, community_mint_pubkey: &Pubkey, realm_name: &str) -> TokenOwnerRecordV2 {
        let token_owner_record_pubkey: Pubkey = self.get_token_owner_record_address(goverinig_token_owner, community_mint_pubkey, realm_name);

        let mut dt: &[u8] = &self.solana_client.get_account_data(&token_owner_record_pubkey).unwrap();
        TokenOwnerRecordV2::deserialize(&mut dt).unwrap()
    }
    pub fn get_governance_v2(&self, realm_name: &str, governed_account_pubkey: &Pubkey) -> GovernanceV2 {
        let governance_pubkey: Pubkey = self.get_governance_address(realm_name, governed_account_pubkey);

        let mut dt: &[u8] = &self.solana_client.get_account_data(&governance_pubkey).unwrap();
        GovernanceV2::deserialize(&mut dt).unwrap()
    }
    pub fn get_proposal_v2(&self, community_mint_pubkey: &Pubkey, realm_name: &str, governed_account_pubkey: &Pubkey, proposal_index: u8) -> ProposalV2 {
        let proposal_pubkey: Pubkey = self.get_proposal_address(community_mint_pubkey, realm_name, governed_account_pubkey, proposal_index);

        let mut dt: &[u8] = &self.solana_client.get_account_data(&proposal_pubkey).unwrap();
        ProposalV2::deserialize(&mut dt).unwrap()
    }
    pub fn get_voter_weight_record(&self, voter_weight_record_pubkey: &Pubkey) -> VoterWeightRecord {
        let mut dt: &[u8] = &self.solana_client.get_account_data(voter_weight_record_pubkey).unwrap();
        VoterWeightRecord::deserialize(&mut dt).unwrap()
    }
    pub fn get_max_voter_weight_record(&self, max_voter_weight_record_pubkey: &Pubkey) -> MaxVoterWeightRecord {
        let mut dt: &[u8] = &self.solana_client.get_account_data(max_voter_weight_record_pubkey).unwrap();
        MaxVoterWeightRecord::deserialize(&mut dt).unwrap()
    }

    pub fn create_realm<'a>(&self, realm_authority: &'a Keypair, community_mint_pubkey: &Pubkey, addin_opt: Option<Pubkey>, realm_name: &str) -> Result<Realm<'a>,ClientError> {
        let realm_pubkey: Pubkey = self.get_realm_address(realm_name);

        if self.account_exists(&realm_pubkey) {
            Ok(
                Realm {
                    authority: realm_authority,
                    address: realm_pubkey,
                    data: self.get_realm_v2(realm_name).unwrap(),
                    max_voter_weight_addin_address: addin_opt,
                    // voter_weight_addin_address: addin_opt,
                }
            )
        } else {
            let realm_authority_pubkey: Pubkey = realm_authority.pubkey();

            let create_realm_instruction: Instruction =
                create_realm(
                    &self.spl_governance_program_address,
                    &realm_authority_pubkey,
                    community_mint_pubkey,
                    &realm_authority_pubkey,
                    None,
                    addin_opt,
                    addin_opt,
                    realm_name.to_string(),
                    MIN_COMMUNITY_WEIGHT_TO_CREATE_GOVERNANCE,
                    MintMaxVoteWeightSource::SupplyFraction(10_000_000_000),
                    // MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_realm_instruction,
                    ],
                    Some(&realm_authority_pubkey),
                    &[
                        realm_authority,
                    ],
                    self.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.solana_client.send_and_confirm_transaction(&transaction)
                .map(|_|
                    Realm {
                        authority: realm_authority,
                        address: realm_pubkey,
                        data: self.get_realm_v2(realm_name).unwrap(),
                        max_voter_weight_addin_address: addin_opt,
                        // voter_weight_addin_address: addin_opt,
                    }
                )
                // .map_err(|_|())
        }
    }

    pub fn create_token_owner_record<'a>(&self, realm: &Realm, token_owner_keypair: &'a Keypair) -> Result<TokenOwner<'a>,()> {
        let token_owner_pubkey: Pubkey = token_owner_keypair.pubkey();
        let token_owner_record_pubkey: Pubkey = self.get_token_owner_record_address(&token_owner_pubkey, &realm.data.community_mint, &realm.data.name);

        if self.account_exists(&token_owner_record_pubkey) {
            Ok(
                TokenOwner {
                    authority: token_owner_keypair,
                    token_owner_record_address: token_owner_record_pubkey,
                    token_owner_record: self.get_token_owner_record_v2(&token_owner_pubkey, &realm.data.community_mint, &realm.data.name),
                    // voter_weight_record_authority: None,
                    voter_weight_record_address: None,
                    // voter_weight_record: None,
                }
            )
        } else {
            let realm_authority_pubkey: Pubkey = realm.authority.pubkey();
        
            let create_token_owner_record_instruction: Instruction =
                create_token_owner_record(
                    &self.spl_governance_program_address,
                    &realm.address,
                    &token_owner_pubkey,
                    &realm.data.community_mint,
                    &realm_authority_pubkey,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_token_owner_record_instruction,
                    ],
                    Some(&realm_authority_pubkey),
                    &[
                        realm.authority,
                    ],
                    self.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.solana_client.send_and_confirm_transaction(&transaction)
                .map(|_|
                    TokenOwner {
                        authority: token_owner_keypair,
                        token_owner_record_address: token_owner_record_pubkey,
                        token_owner_record: self.get_token_owner_record_v2(&token_owner_pubkey, &realm.data.community_mint, &realm.data.name),
                        // voter_weight_record_authority: None,
                        voter_weight_record_address: None,
                        // voter_weight_record: None,
                    }
                )
                .map_err(|_|())
        }
    }

    pub fn _setup_max_voter_weight_record_mock(&self, realm: &Realm, max_voter_weight_record_keypair: Keypair, max_voter_weight: u64) -> Result<Signature,()> {
        let max_voter_weight_record_pubkey: Pubkey = max_voter_weight_record_keypair.pubkey();

        if self.account_exists(&max_voter_weight_record_pubkey) {
            Err(())
        } else {
            let realm_authority_pubkey: Pubkey = realm.authority.pubkey();

            let setup_max_voter_weight_record_instruction: Instruction =
                setup_max_voter_weight_record(
                    &self.spl_governance_voter_weight_addin_address,
                    &realm.address,
                    &realm.data.community_mint,
                    &max_voter_weight_record_pubkey,
                    &realm_authority_pubkey,
                    max_voter_weight,
                    None,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_max_voter_weight_record_instruction,
                    ],
                    Some(&realm_authority_pubkey),
                    &[
                        &realm.authority,
                        &max_voter_weight_record_keypair,
                    ],
                    self.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.solana_client.send_and_confirm_transaction(&transaction)
                .map_err(|_|())
        }
    }

    // pub fn setup_max_voter_weight_record_fixed(&self, voter_weight_addin_autority: &Keypair, realm: &Realm, max_voter_weight: u64) -> ClientResult<Signature> {
    pub fn setup_max_voter_weight_record_fixed(&self, realm: &Realm) -> Result<Signature,()> {
        // let max_voter_weight_record_pubkey: Pubkey = max_voter_weight_record_keypair.pubkey();
        let (max_voter_weight_record_pubkey,_): (Pubkey,u8) = spl_governance_addin_fixed_weights::instruction::get_max_voter_weight_address(&self.spl_governance_voter_weight_addin_address, &realm.address, &realm.data.community_mint);

        if self.account_exists(&max_voter_weight_record_pubkey) {
            Err(())
        } else {
            let realm_authority_pubkey: Pubkey = realm.authority.pubkey();

            let setup_max_voter_weight_record_instruction: Instruction =
                spl_governance_addin_fixed_weights::instruction::setup_max_voter_weight_record(
                    &self.spl_governance_voter_weight_addin_address,
                    &realm.address,
                    &realm.data.community_mint,
                    &realm_authority_pubkey,
                    // max_voter_weight,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_max_voter_weight_record_instruction,
                    ],
                    Some(&realm_authority_pubkey),
                    &[
                        realm.authority,
                        // voter_weight_addin_autority,
                        // &max_voter_weight_record_keypair,
                    ],
                    self.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.solana_client.send_and_confirm_transaction(&transaction)
                .map_err(|_|())
        }
    }

/*    pub fn _setup_voter_weight_record_mock<'a>(&self, realm: &Realm, token_owner: &'a mut TokenOwner, voter_weight_record_keypair: Keypair, voter_weight: u64) -> Result<TokenOwner<'a>,()> {
        let voter_weight_record_pubkey: Pubkey = voter_weight_record_keypair.pubkey();

        if self.account_exists(&voter_weight_record_pubkey) {
            Ok(
                TokenOwner {
                    authority: token_owner.authority,
                    token_owner_record_address: token_owner.token_owner_record_address,
                    token_owner_record: token_owner.token_owner_record,
                    // voter_weight_record_authority: Some(voter_weight_record_keypair),
                    voter_weight_record_address: Some(voter_weight_record_pubkey),
                    // voter_weight_record: Some(self.get_voter_weight_record(&voter_weight_record_pubkey)),
                }
            )
        } else {
            let realm_authority_pubkey: Pubkey = realm.authority.pubkey();

            let setup_voter_weight_record_instruction: Instruction =
                setup_voter_weight_record(
                    &self.spl_governance_voter_weight_addin_address,
                    &realm.address,
                    &realm.data.community_mint,
                    &token_owner.authority.pubkey(),
                    &voter_weight_record_pubkey,
                    &realm_authority_pubkey,
                    voter_weight,
                    None,
                    None,
                    None,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_voter_weight_record_instruction,
                    ],
                    Some(&realm_authority_pubkey),
                    &[
                        &realm.authority,
                        &voter_weight_record_keypair,
                    ],
                    self.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.solana_client.send_and_confirm_transaction(&transaction)
                .map(|_|
                    TokenOwner {
                        authority: token_owner.authority,
                        token_owner_record_address: token_owner.token_owner_record_address,
                        token_owner_record: token_owner.token_owner_record,
                        // voter_weight_record_authority: Some(voter_weight_record_keypair),
                        voter_weight_record_address: Some(voter_weight_record_pubkey),
                        // voter_weight_record: Some(self.get_voter_weight_record(&voter_weight_record_pubkey)),
                    }
                )
                .map_err(|_|())
        }
    }*/

    pub fn setup_voter_weight_record_fixed<'a>(&self, realm: &Realm, token_owner: TokenOwner<'a>) -> Result<TokenOwner<'a>,()> {
        let token_owner_pubkey: Pubkey = token_owner.authority.pubkey();
        let (voter_weight_record_pubkey,_): (Pubkey,u8) = spl_governance_addin_fixed_weights::instruction::get_voter_weight_address(&self.spl_governance_voter_weight_addin_address, &realm.address, &realm.data.community_mint, &token_owner_pubkey);

        if self.account_exists(&voter_weight_record_pubkey) {
            Ok(
                TokenOwner {
                    authority: token_owner.authority,
                    token_owner_record_address: token_owner.token_owner_record_address,
                    token_owner_record: token_owner.token_owner_record,
                    // voter_weight_record_authority: None,
                    voter_weight_record_address: Some(voter_weight_record_pubkey),
                    // voter_weight_record: Some(self.get_voter_weight_record(&voter_weight_record_pubkey)),
                }
            )
        } else {
            let realm_authority_pubkey: Pubkey = realm.authority.pubkey();

            let setup_voter_weight_record_instruction: Instruction =
                spl_governance_addin_fixed_weights::instruction::setup_voter_weight_record(
                    &self.spl_governance_voter_weight_addin_address,
                    &realm.address,
                    &realm.data.community_mint,
                    &token_owner_pubkey,
                    &realm_authority_pubkey,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_voter_weight_record_instruction,
                    ],
                    Some(&realm_authority_pubkey),
                    &[
                        realm.authority,
                        // &voter_weight_record_keypair,
                    ],
                    self.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.solana_client.send_and_confirm_transaction(&transaction)
                .map(|_|
                    TokenOwner {
                        authority: token_owner.authority,
                        token_owner_record_address: token_owner.token_owner_record_address,
                        token_owner_record: token_owner.token_owner_record,
                        // voter_weight_record_authority: None,
                        voter_weight_record_address: Some(voter_weight_record_pubkey),
                        // voter_weight_record: Some(self.get_voter_weight_record(&voter_weight_record_pubkey)),
                    }
                )
                .map_err(|_|())
        }
    }

    pub fn create_governance(&self, realm: &Realm, token_owner: &TokenOwner, governed_account_pubkey: &Pubkey, gov_config: GovernanceConfig) -> Result<Governance,ClientError> {
        let governance_pubkey: Pubkey = self.get_governance_address(&realm.data.name, governed_account_pubkey);

        if self.account_exists(&governance_pubkey) {
            Ok(
                Governance {
                    address: governance_pubkey,
                    data: self.get_governance_v2(&realm.data.name, governed_account_pubkey)
                }
            )
        } else {
            let realm_authority_pubkey: Pubkey = realm.authority.pubkey();
            // let token_owner_record_pubkey: Pubkey = self.get_token_owner_record_address(&realm_authority_pubkey, &community_mint_pubkey, realm_name);

            let create_governance_instruction: Instruction =
                create_governance(
                    &self.spl_governance_program_address,
                    &realm.address,
                    Some(governed_account_pubkey),
                    &token_owner.token_owner_record_address,
                    &realm_authority_pubkey,
                    &realm_authority_pubkey,
                    token_owner.voter_weight_record_address,
                    gov_config,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_governance_instruction,
                    ],
                    Some(&realm_authority_pubkey),
                    &[
                        realm.authority,
                    ],
                    self.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.solana_client.send_and_confirm_transaction(&transaction)
                .map(|_|
                    Governance {
                        address: governance_pubkey,
                        data: self.get_governance_v2(&realm.data.name, governed_account_pubkey)
                    }
                )
                // .map_err(|_|())
        }
    }

    pub fn create_proposal(&self, realm: &Realm, token_owner: &TokenOwner, governance: &Governance, proposal_name: &str, proposal_description: &str, proposal_index: u32) -> Result<Proposal,ClientError> {
        let proposal_address: Pubkey = self.get_proposal_address(&realm.data.community_mint, &realm.data.name, &governance.data.governed_account, proposal_index as u8);

        if self.account_exists(&proposal_address) {
            let proposal_v2: ProposalV2 = self.get_proposal_v2(&realm.data.community_mint, &realm.data.name, &governance.data.governed_account, proposal_index as u8);
            Ok(
                Proposal {
                    address: proposal_address,
                    data: proposal_v2,
                }
            )
        } else {
            let realm_authority_pubkey: Pubkey = realm.authority.pubkey();

            let create_proposal_instruction: Instruction =
                create_proposal(
                    &self.spl_governance_program_address,
                    &governance.address,
                    &token_owner.token_owner_record_address,
                    &realm_authority_pubkey,
                    &realm_authority_pubkey,
                    token_owner.voter_weight_record_address,
                    &realm.address,
                    proposal_name.to_string(),
                    proposal_description.to_string(),
                    &realm.data.community_mint,
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
                    Some(&realm_authority_pubkey),
                    &[
                        realm.authority,
                    ],
                    self.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.solana_client.send_and_confirm_transaction(&transaction)
                .map(|_| {
                    let proposal_v2: ProposalV2 = self.get_proposal_v2(&realm.data.community_mint, &realm.data.name, &governance.data.governed_account, proposal_index as u8);
                    Proposal {
                        address: proposal_address,
                        data: proposal_v2,
                    }
                })
        }
    }

    pub fn sign_off_proposal(&self, realm: &Realm, governance: &Governance, proposal: Proposal, token_owner: &TokenOwner) -> Result<Proposal,ClientError> {
        let realm_authority_pubkey: Pubkey = realm.authority.pubkey();

        let sign_off_proposal_instruction: Instruction =
            sign_off_proposal(
                &self.spl_governance_program_address,
                &realm.address,
                &governance.address,
                &proposal.address,
                &realm_authority_pubkey,
                Some(&token_owner.token_owner_record_address),
            );
        
        let transaction: Transaction =
            Transaction::new_signed_with_payer(
                &[
                    sign_off_proposal_instruction,
                ],
                Some(&realm_authority_pubkey),
                &[
                    realm.authority,
                ],
                self.solana_client.get_latest_blockhash().unwrap(),
            );
        
        self.solana_client.send_and_confirm_transaction(&transaction)
            .map(|_|
                  Proposal {
                      address: proposal.address,
                      data: self.get_proposal_v2(&realm.data.community_mint, &realm.data.name, &governance.data.governed_account, governance.data.proposals_count as u8),
                  }
            )
    }

    pub fn _add_signatory(&self, realm: &Realm, _governance: &Governance, proposal: &Proposal, token_owner: &TokenOwner) -> Result<Signature,ClientError> {
        let realm_authority_pubkey: Pubkey = realm.authority.pubkey();
        // let signatory_record_address = get_signatory_record_address(&self.spl_governance_program_address, &proposal.address, &token_owner.authority.pubkey());

        let add_signatory_instruction: Instruction =
            add_signatory(
                &self.spl_governance_program_address,
                &proposal.address,
                &token_owner.token_owner_record_address,
                &realm_authority_pubkey,
                &realm_authority_pubkey,
                &token_owner.authority.pubkey(),
            );
        
        let transaction: Transaction =
            Transaction::new_signed_with_payer(
                &[
                    add_signatory_instruction,
                ],
                Some(&realm_authority_pubkey),
                &[
                    realm.authority,
                ],
                self.solana_client.get_latest_blockhash().unwrap(),
            );
        
        self.solana_client.send_and_confirm_transaction(&transaction)
            // .map(|_|
            //       Proposal {
            //           address: proposal.address,
            //           data: self.get_proposal_v2(&realm.data.community_mint, &realm.data.name, &governance.data.governed_account, governance.data.proposals_count as u8),
            //       }
            // )
    }

    pub fn cast_vote(&self, realm: &Realm, governance: &Governance, proposal: &Proposal, voter: &TokenOwner, vote_yes_no: bool) -> ClientResult<Signature> {
        let voter_authority_pubkey: Pubkey = voter.authority.pubkey();
        let max_voter_weight_addin_address: &Pubkey = &realm.max_voter_weight_addin_address.unwrap();
        let (max_voter_weight_record_address,_) = get_max_voter_weight_address(max_voter_weight_addin_address, &realm.address, &realm.data.community_mint);

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
                &self.spl_governance_program_address,
                &realm.address,
                &governance.address,
                &proposal.address,
                &proposal.data.token_owner_record,
                &voter.token_owner_record_address,
                &voter_authority_pubkey,
                &realm.data.community_mint,
                &voter_authority_pubkey,
                voter.voter_weight_record_address,
                Some(max_voter_weight_record_address),
                vote,
            );
        
        let transaction: Transaction =
            Transaction::new_signed_with_payer(
                &[
                    cast_vote_instruction,
                ],
                Some(&voter_authority_pubkey),
                &[
                    voter.authority,
                ],
                self.solana_client.get_latest_blockhash().unwrap(),
            );
        
        self.solana_client.send_and_confirm_transaction(&transaction)
    }
}

#[derive(Debug)]
pub struct Realm<'a> {
    authority: &'a Keypair,
    pub address: Pubkey,
    data: RealmV2,
    max_voter_weight_addin_address: Option<Pubkey>,
    // voter_weight_addin_address: Option<Pubkey>,
}

#[derive(Debug)]
pub struct Governance {
    address: Pubkey,
    data: GovernanceV2,
}

impl Governance {
    pub fn get_proposal_count(&self) -> u32 {
        self.data.proposals_count
    }
}

#[derive(Debug)]
pub struct Proposal {
    address: Pubkey,
    pub data: ProposalV2,
}

#[derive(Debug)]
pub struct TokenOwner<'a> {
    pub authority: &'a Keypair,
    token_owner_record_address: Pubkey,
    token_owner_record: TokenOwnerRecordV2,
    // voter_weight_record_authority: Option<Keypair>,
    voter_weight_record_address: Option<Pubkey>,
    // voter_weight_record: Option<VoterWeightRecord>,
}
