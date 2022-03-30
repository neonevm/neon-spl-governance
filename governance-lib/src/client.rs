use {
    crate::realm::Realm,
    borsh::BorshDeserialize,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        instruction::Instruction,
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
    },
    std::fmt,
    spl_governance::{
        state::{
            enums::MintMaxVoteWeightSource,
            realm::{RealmV2, get_realm_address},
        },
        instruction::create_realm,
    },
    solana_client::{
        rpc_client::RpcClient,
        client_error::ClientError,
    },
};

const MIN_COMMUNITY_WEIGHT_TO_CREATE_GOVERNANCE: u64 = 1;

pub struct SplGovernanceInteractor<'a> {
    pub url: String,
    pub solana_client: RpcClient,
    pub payer: &'a Keypair,
    pub spl_governance_program_address: Pubkey,
    pub spl_governance_voter_weight_addin_address: Pubkey,
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
