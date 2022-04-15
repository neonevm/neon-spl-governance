use {
    crate::realm::{Realm, RealmSettings},
    borsh::BorshDeserialize,
    solana_sdk::{
        borsh::try_from_slice_unchecked,
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        instruction::Instruction,
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
        signers::Signers,
        signature::Signature,
        program_pack::{Pack, IsInitialized},
    },
    std::{cell::RefCell, fmt},
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
            payer,
            spl_governance_program_address: program_address,
            spl_governance_voter_weight_addin_address: addin_address,
        }
    }

    pub fn send_and_confirm_transaction<T: Signers>(
        &self,
        instructions: &[Instruction],
        signing_keypairs: &T,
    ) -> Result<Signature, ClientError> {
        let mut transaction: Transaction =
            Transaction::new_with_payer(
                instructions,
                Some(&self.payer.pubkey()),
            );

        let blockhash = self.solana_client.get_latest_blockhash().unwrap();
        transaction.partial_sign(&[self.payer], blockhash);
        transaction.sign(signing_keypairs, blockhash);
        
        self.solana_client.send_and_confirm_transaction(&transaction)
    }

    pub fn get_account_data_pack<T: Pack + IsInitialized>(
        &self,
        owner_program_id: &Pubkey,
        account_key: &Pubkey,
    ) -> Result<Option<T>, ClientError> {
        let account_info = &self.solana_client.get_account_with_commitment(
                &account_key, self.solana_client.commitment())?.value;

        if let Some(account_info) = account_info {
            if account_info.data.is_empty() {
                panic!("Account {} is empty", account_key);
            }
            if account_info.owner != *owner_program_id {
                panic!("Invalid account owner for {}: expected {}, actual {}",
                        account_key, owner_program_id, account_info.owner);
            }
        
            let account: T = T::unpack(&account_info.data).unwrap(); //try_from_slice_unchecked(&account_info.data).unwrap();
            if !account.is_initialized() {
                panic!("Unitialized account {}", account_key);
            }
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    pub fn get_account_data<T: BorshDeserialize + IsInitialized>(
        &self,
        owner_program_id: &Pubkey,
        account_key: &Pubkey,
    ) -> Result<Option<T>, ClientError> {
        let account_info = &self.solana_client.get_account_with_commitment(
                &account_key, self.solana_client.commitment())?.value;

        if let Some(account_info) = account_info {
            if account_info.data.is_empty() {
                panic!("Account {} is empty", account_key);
            }
            if account_info.owner != *owner_program_id {
                panic!("Invalid account owner for {}: expected {}, actual {}",
                        account_key, owner_program_id, account_info.owner);
            }
        
            let account: T = try_from_slice_unchecked(&account_info.data).unwrap();
            if !account.is_initialized() {
                panic!("Unitialized account {}", account_key);
            }
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    pub fn account_exists(&self, address: &Pubkey) -> bool {
        self.solana_client.get_account(address).is_ok()
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
                //max_voter_weight_record_address: RefCell::new(None),
                _settings: RefCell::new(RealmSettings {
                        max_voter_weight_record_address: addin_opt,
                    }),
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
