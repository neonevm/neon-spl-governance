use {
    crate::{
        client::{Client, ClientResult},
        realm::Realm,
        errors::GovernanceLibError,
    },
    borsh::{BorshSchema,BorshDeserialize},
    goblin::elf::Elf,
    solana_sdk::{
        borsh::{get_packed_len, try_from_slice_unchecked},
        pubkey::Pubkey,
        signer::{
            Signer,
            keypair::Keypair,
        },
        instruction::Instruction,
        transaction::Transaction,
    },
    std::collections::HashMap,
};

#[derive(Debug)]
pub struct AddinFixedWeights<'a> {
    client: &'a Client<'a>,
    pub program_id: Pubkey,
}

#[derive(Clone,Debug,BorshSchema,BorshDeserialize)]
pub struct VoterWeight {
    pub voter: Pubkey,
    pub weight: u64,
}

impl<'a> AddinFixedWeights<'a> {
    pub fn new(client: &'a Client, program_id: Pubkey) -> Self {
        AddinFixedWeights {
            client,
            program_id,
        }
    }

    pub fn setup_max_voter_weight_record(&self, realm: &Realm) -> Result<Pubkey, GovernanceLibError> {
        use spl_governance_addin_fixed_weights::instruction;
        let (max_voter_weight_record_pubkey,_): (Pubkey,u8) = instruction::get_max_voter_weight_address(
                &self.program_id,
                &realm.realm_address,
                &realm.community_mint,
            );

        if !self.client.account_exists(&max_voter_weight_record_pubkey) {
            let setup_max_voter_weight_record_instruction: Instruction =
                instruction::setup_max_voter_weight_record(
                    &self.program_id,
                    &realm.realm_address,
                    &realm.community_mint,
                    &self.client.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_max_voter_weight_record_instruction,
                    ],
                    Some(&self.client.payer.pubkey()),
                    &[
                        self.client.payer,
                    ],
                    self.client.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.client.solana_client.send_and_confirm_transaction(&transaction)
                .map_err(GovernanceLibError::ClientError)?;
        }

        Ok(max_voter_weight_record_pubkey)
    }

    pub fn get_voter_weight_record_address(&self, realm: &Realm, token_owner: &Pubkey) -> Pubkey {
        spl_governance_addin_fixed_weights::instruction::get_voter_weight_address(
                &self.program_id,
                &realm.realm_address,
                &realm.community_mint,
                token_owner).0
    }

    pub fn setup_voter_weight_record_instruction(&self, realm: &Realm, token_owner: &Pubkey) -> Instruction {
        spl_governance_addin_fixed_weights::instruction::setup_voter_weight_record(
            &self.program_id,
            &realm.realm_address,
            &realm.community_mint,
            token_owner,
            &self.client.payer.pubkey(),
        )
    }

    pub fn setup_voter_weight_record(&self, realm: &Realm, token_owner: &Pubkey) -> Result<Pubkey,GovernanceLibError> {
        let (voter_weight_record_pubkey,_): (Pubkey,u8) = spl_governance_addin_fixed_weights::instruction::get_voter_weight_address(
                &self.program_id,
                &realm.realm_address,
                &realm.community_mint,
                token_owner);

        if !self.client.account_exists(&voter_weight_record_pubkey) {
            let setup_voter_weight_record_instruction: Instruction =
                spl_governance_addin_fixed_weights::instruction::setup_voter_weight_record(
                    &self.program_id,
                    &realm.realm_address,
                    &realm.community_mint,
                    token_owner,
                    &self.client.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_voter_weight_record_instruction,
                    ],
                    Some(&self.client.payer.pubkey()),
                    &[
                        self.client.payer,
                    ],
                    self.client.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.client.solana_client.send_and_confirm_transaction(&transaction)
                .map_err(GovernanceLibError::ClientError)?;
        }
        
        Ok(voter_weight_record_pubkey)
    }

    pub fn set_vote_percentage_fixed(&self, realm: &Realm, token_owner: &Keypair, percentage: u16) -> Result<Pubkey,GovernanceLibError> {
        let (voter_weight_record_pubkey,_): (Pubkey,u8) = spl_governance_addin_fixed_weights::instruction::get_voter_weight_address(
                &self.program_id,
                &realm.realm_address,
                &realm.community_mint,
                &token_owner.pubkey());

        if self.client.account_exists(&voter_weight_record_pubkey) {
            let set_partial_voting_instruction: Instruction =
                spl_governance_addin_fixed_weights::instruction::set_vote_percentage_with_realm(
                    &self.program_id,
                    &realm.community_mint,
                    &token_owner.pubkey(),
                    &token_owner.pubkey(),
                    &realm.program_id,
                    &realm.realm_address,
                    percentage,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        set_partial_voting_instruction,
                    ],
                    Some(&self.client.payer.pubkey()),
                    &[
                        self.client.payer,
                        token_owner,
                    ],
                    self.client.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.client.solana_client.send_and_confirm_transaction(&transaction)
                .map_err(GovernanceLibError::ClientError)?;
        }
        
        Ok(voter_weight_record_pubkey)
    }

    pub fn get_voter_list(&self) -> ClientResult<Vec<VoterWeight>> {
        let program_data = &self.client.get_program_data(&self.program_id)?;
        let elf = Elf::parse(program_data).expect("Can't parse Elf data");
        for sym in elf.dynsyms.iter() {
            let name = String::from(&elf.dynstrtab[sym.st_name]);
            if name == "VOTER_LIST" {
                let end = program_data.len();
                let from: usize = usize::try_from(sym.st_value).map_err(|_| GovernanceLibError::InvalidElfData("Unable to cast usize".to_string()))?;
                let to: usize = usize::try_from(sym.st_value + sym.st_size).map_err(|_| GovernanceLibError::InvalidElfData("Unable to cast usize".to_string()))?;
                if to < end && from < end {
                    let item_len:usize = get_packed_len::<VoterWeight>();
                    if (to-from) % item_len != 0 {
                        return Err(GovernanceLibError::InvalidElfData("Invalid length of VOTER_LIST".to_string()));
                    }
                    let buf = &program_data[from..to];
                    let items_count = (to-from) / item_len;
                    let mut result = Vec::new();
                    for i in 0..items_count {
                        let item = try_from_slice_unchecked::<VoterWeight>(&buf[i*item_len..(i+1)*item_len])?;
                        result.push(item);
                    }
                    return Ok(result);
                }
                else {
                    return Err(GovernanceLibError::InvalidElfData(format!("{} is out of bounds", name)));
                }
            }
        }
        panic!("Can't find VOTER_LIST symbol in Elf data");
    }

    pub fn get_params(&self) -> ClientResult<HashMap<String,String>> {
        let mut result = HashMap::new();
        let program_data = &self.client.get_program_data(&self.program_id)?;
        let elf = Elf::parse(program_data).expect("Can't parse Elf data");
        elf.dynsyms.iter().for_each(|sym| {
            let name = String::from(&elf.dynstrtab[sym.st_name]);
            if name.starts_with("PARAM_")
            {
                let end = program_data.len();
                let from: usize = usize::try_from(sym.st_value).unwrap_or_else(|_| panic!("Unable to cast usize from u64:{:?}", sym.st_value));
                let to: usize = usize::try_from(sym.st_value + sym.st_size).unwrap_or_else(|err| panic!("Unable to cast usize from u64:{:?}. Error: {}", sym.st_value + sym.st_size, err));
                if to < end && from < end {
                    let buf = &program_data[from..to];
                    let value = std::str::from_utf8(buf).unwrap();
                    result.insert(name, String::from(value));
                }
                else {
                    panic!("{} is out of bounds", name);
                }
            }
        });
        Ok(result)
    }
}
