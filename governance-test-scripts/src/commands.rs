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

use std::fmt;

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
            ProposalState,
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
        //add_signatory,
        cast_vote,
    }
};

//use spl_governance_addin_api::{
//    max_voter_weight::{
//        MaxVoterWeightRecord,
//    },
//    voter_weight::{
//        VoterWeightRecord,
//    },
//};

//use spl_governance_addin_fixed_weights::{
//    instruction::{
//        get_max_voter_weight_address,
//    }
//};

const MIN_COMMUNITY_WEIGHT_TO_CREATE_GOVERNANCE: u64 = 1;

pub struct SplGovernanceInteractor<'a> {
    url: String,
    solana_client: RpcClient,
    payer: &'a Keypair,
    spl_governance_program_address: Pubkey,
    spl_governance_voter_weight_addin_address: Pubkey,
}

impl<'a> fmt::Debug for SplGovernanceInteractor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SplGovernanceInteractor")
            .field("url", &self.url)
            .field("program", &self.spl_governance_program_address)
            .field("addin", &self.spl_governance_voter_weight_addin_address)
            .finish()
    }
}

#[derive(Debug)]
pub struct Governance<'a> {
    realm: &'a Realm<'a>,
    address: Pubkey,
    data: GovernanceV2,
}

impl<'a> Governance<'a> {
    pub fn get_interactor(&self) -> &SplGovernanceInteractor<'a> {self.realm.interactor}

    pub fn get_proposal_count(&self) -> u32 {
        self.data.proposals_count
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

#[derive(Debug)]
pub struct Proposal<'a> {
    governance: &'a Governance<'a>,
    address: Pubkey,
    token_owner_record: Pubkey,
    pub data: ProposalV2,
}

#[derive(Debug)]
pub struct TokenOwner<'a> {
    pub realm: &'a Realm<'a>,
    token_owner_record_address: Pubkey,
    token_owner_record: TokenOwnerRecordV2,
    voter_weight_record_address: Option<Pubkey>,
}

impl<'a> TokenOwner<'a> {
    pub fn set_voter_weight_record_address(&mut self, voter_weight_record_address: Option<Pubkey>) {
        self.voter_weight_record_address = voter_weight_record_address;
    }
}

impl<'a> SplGovernanceInteractor<'a> {

    pub fn new(url: &str, program_address: Pubkey, addin_address: Pubkey, payer: &'a Keypair) -> Self {
        SplGovernanceInteractor {
            url: url.to_string(),
            solana_client: RpcClient::new_with_commitment(url.to_string(),CommitmentConfig::confirmed()),
            payer: payer,
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
    pub fn get_realm_v2(&self, realm_name: &str) -> Result<RealmV2,()> {
        let realm_pubkey: Pubkey = self.get_realm_address(realm_name);

        self.solana_client.get_account_data(&realm_pubkey)
            .map_err(|_|())
            .and_then(|data|{
                let mut data_slice: &[u8] = &data;
                RealmV2::deserialize(&mut data_slice).map_err(|_|())

            })
    }
//    pub fn get_voter_weight_record(&self, voter_weight_record_pubkey: &Pubkey) -> VoterWeightRecord {
//        let mut dt: &[u8] = &self.solana_client.get_account_data(voter_weight_record_pubkey).unwrap();
//        VoterWeightRecord::deserialize(&mut dt).unwrap()
//    }
//    pub fn get_max_voter_weight_record(&self, max_voter_weight_record_pubkey: &Pubkey) -> MaxVoterWeightRecord {
//        let mut dt: &[u8] = &self.solana_client.get_account_data(max_voter_weight_record_pubkey).unwrap();
//        MaxVoterWeightRecord::deserialize(&mut dt).unwrap()
//    }

    pub fn create_realm(&'a self, realm_authority: &'a Keypair, community_mint_pubkey: &Pubkey, addin_opt: Option<Pubkey>, realm_name: &str) -> Result<Realm<'a>,ClientError> {
        let realm_pubkey: Pubkey = self.get_realm_address(realm_name);

        if !self.account_exists(&realm_pubkey) {
            let realm_authority_pubkey: Pubkey = realm_authority.pubkey();

            let create_realm_instruction: Instruction =
                create_realm(
                    &self.spl_governance_program_address,
                    &realm_authority_pubkey,
                    community_mint_pubkey,
                    &self.payer.pubkey(),
                    None,
                    addin_opt,
                    addin_opt,
                    realm_name.to_string(),
                    MIN_COMMUNITY_WEIGHT_TO_CREATE_GOVERNANCE,
                    //MintMaxVoteWeightSource::SupplyFraction(10_000_000_000),
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_realm_instruction,
                    ],
                    Some(&self.payer.pubkey()),
                    &[
                        self.payer,
                    ],
                    self.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.solana_client.send_and_confirm_transaction(&transaction)?;
        }

        Ok(
            Realm {
                //authority: realm_authority,
                interactor: self,
                address: realm_pubkey,
                community_mint: *community_mint_pubkey,
                data: self.get_realm_v2(realm_name).unwrap(),
                //max_voter_weight_addin_address: addin_opt,
                // voter_weight_addin_address: addin_opt,
                max_voter_weight_record_address: None,
            }
        )
    }


/*    pub fn _add_signatory(&self, realm: &Realm, _governance: &Governance, proposal: &Proposal, token_owner: &TokenOwner) -> Result<Signature,ClientError> {
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
    }*/

}

#[derive(Debug)]
pub struct AddinFixedWeights<'a> {
    interactor: &'a SplGovernanceInteractor<'a>,
    pub program_id: Pubkey,
}

impl<'a> AddinFixedWeights<'a> {
    pub fn new(interactor: &'a SplGovernanceInteractor, program_id: Pubkey) -> Self {
        AddinFixedWeights {
            interactor,
            program_id,
        }
    }

    pub fn setup_max_voter_weight_record(&self, realm: &Realm) -> Result<Pubkey, ()> {
        use spl_governance_addin_fixed_weights::instruction;
        let (max_voter_weight_record_pubkey,_): (Pubkey,u8) = instruction::get_max_voter_weight_address(
                &self.program_id,
                &realm.address,
                &realm.community_mint,
            );

        if !self.interactor.account_exists(&max_voter_weight_record_pubkey) {
            let setup_max_voter_weight_record_instruction: Instruction =
                instruction::setup_max_voter_weight_record(
                    &self.program_id,
                    &realm.address,
                    &realm.community_mint,
                    &self.interactor.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_max_voter_weight_record_instruction,
                    ],
                    Some(&self.interactor.payer.pubkey()),
                    &[
                        self.interactor.payer,
                    ],
                    self.interactor.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.interactor.solana_client.send_and_confirm_transaction(&transaction)
                .map_err(|_|())?;
        }
        Ok(max_voter_weight_record_pubkey)
    }

    pub fn setup_voter_weight_record(&self, realm: &Realm, token_owner: &Pubkey) -> Result<Pubkey,()> {
        let (voter_weight_record_pubkey,_): (Pubkey,u8) = spl_governance_addin_fixed_weights::instruction::get_voter_weight_address(
                &self.program_id,
                &realm.address,
                &realm.community_mint,
                &token_owner);

        if !self.interactor.account_exists(&voter_weight_record_pubkey) {
            let setup_voter_weight_record_instruction: Instruction =
                spl_governance_addin_fixed_weights::instruction::setup_voter_weight_record(
                    &self.program_id,
                    &realm.address,
                    &realm.data.community_mint,
                    &token_owner,
                    &self.interactor.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_voter_weight_record_instruction,
                    ],
                    Some(&self.interactor.payer.pubkey()),
                    &[
                        self.interactor.payer,
                    ],
                    self.interactor.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.interactor.solana_client.send_and_confirm_transaction(&transaction).unwrap();
        }
        Ok(voter_weight_record_pubkey)
    }
}

#[derive(Debug)]
pub struct Realm<'a> {
//    authority: &'a Keypair,
//    payer_authority: &'a Keypair,
    interactor: &'a SplGovernanceInteractor<'a>,
    pub address: Pubkey,
    pub community_mint: Pubkey,
    pub data: RealmV2,
    //max_voter_weight_addin_address: Option<Pubkey>,
    pub max_voter_weight_record_address: Option<Pubkey>,
    // voter_weight_addin_address: Option<Pubkey>,
}

impl<'a> Realm<'a> {

    pub fn set_max_voter_weight_record_address(&mut self, max_voter_weight_record_address: Option<Pubkey>) {
        self.max_voter_weight_record_address = max_voter_weight_record_address;
    }

    pub fn get_token_owner_record_address(&self, token_owner: &Pubkey) -> Pubkey {
        get_token_owner_record_address(&self.interactor.spl_governance_program_address, &self.address, &self.community_mint, token_owner)
    }

    pub fn get_token_owner_record_v2(&self, token_owner: &Pubkey) -> TokenOwnerRecordV2 {
        let record_address = self.get_token_owner_record_address(token_owner);
        let mut dt: &[u8] = &self.interactor.solana_client.get_account_data(&record_address).unwrap();
        TokenOwnerRecordV2::deserialize(&mut dt).unwrap()
    }

    pub fn create_token_owner_record<'b:'a>(&'b self, token_owner: &Pubkey) -> Result<TokenOwner<'a>,ClientError> {
        let token_owner_record_address: Pubkey = self.get_token_owner_record_address(&token_owner);

        if !self.interactor.account_exists(&token_owner_record_address) {
            let create_token_owner_record_instruction: Instruction =
                create_token_owner_record(
                    &self.interactor.spl_governance_program_address,
                    &self.address,
                    token_owner,
                    &self.community_mint,
                    &self.interactor.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_token_owner_record_instruction,
                    ],
                    Some(&self.interactor.payer.pubkey()),
                    &[
                        self.interactor.payer,
                    ],
                    self.interactor.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.interactor.solana_client.send_and_confirm_transaction(&transaction)?;
        }
        Ok(
            TokenOwner {
                realm: self,
                //authority: token_owner_keypair,
                token_owner_record_address: token_owner_record_address,
                token_owner_record: self.get_token_owner_record_v2(&token_owner),
                // voter_weight_record_authority: None,
                voter_weight_record_address: None,
                // voter_weight_record: None,
            }
        )
    }

    pub fn get_governance_address(&self, governed_account_pubkey: &Pubkey) -> Pubkey {
        get_governance_address(&self.interactor.spl_governance_program_address, &self.address, governed_account_pubkey)
    }

    pub fn get_governance_v2(&self, governed_account_pubkey: &Pubkey) -> GovernanceV2 {
        let governance_pubkey: Pubkey = self.get_governance_address(governed_account_pubkey);

        let mut dt: &[u8] = &self.interactor.solana_client.get_account_data(&governance_pubkey).unwrap();
        GovernanceV2::deserialize(&mut dt).unwrap()
    }

    pub fn create_governance<'b:'a>(&'b self, create_authority: &Keypair, token_owner: &TokenOwner, governed_account_pubkey: &Pubkey, gov_config: GovernanceConfig) -> Result<Governance<'a>,ClientError> {
        let governance_pubkey: Pubkey = self.get_governance_address(governed_account_pubkey);

        if !self.interactor.account_exists(&governance_pubkey) {
            let create_governance_instruction: Instruction =
                create_governance(
                    &self.interactor.spl_governance_program_address,
                    &self.address,
                    Some(governed_account_pubkey),
                    &token_owner.token_owner_record_address,
                    &self.interactor.payer.pubkey(),
                    &create_authority.pubkey(),       // realm_authority OR token_owner authority
                    token_owner.voter_weight_record_address,
                    gov_config,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_governance_instruction,
                    ],
                    Some(&self.interactor.payer.pubkey()),
                    &[
                        create_authority,
                        self.interactor.payer,
                    ],
                    self.interactor.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.interactor.solana_client.send_and_confirm_transaction(&transaction).unwrap();
        }
        Ok(
            Governance {
                realm: self,
                address: governance_pubkey,
                data: self.get_governance_v2(governed_account_pubkey)
            }
        )
    }
}
