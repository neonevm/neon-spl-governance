use crate::{
    AccountOwner, AccountOwnerResolver,
    Lockup,
    ExtraTokenAccount,
    TOKEN_MULT,
    tokens::{
        get_mint_data,
        get_account_data,
        create_mint_instructions,
        assert_is_valid_account_data,
    },
    errors::{StateError, ScriptError},
    wallet::Wallet,
    msig::MultiSig,
    helpers::{
        TransactionExecutor,
        TransactionCollector,
        ProposalTransactionInserter,
    },
};
use solana_sdk::{
    pubkey::Pubkey,
    signer::{
        Signer,
        keypair::Keypair,
    },
    system_instruction,
    rent::Rent,
};

use spl_governance::{
    state::{
        enums::{
            MintMaxVoteWeightSource,
            VoteThresholdPercentage,
            VoteTipping,
            ProposalState,
        },
        governance::GovernanceConfig,
        realm::SetRealmAuthorityAction,
    },
};

use governance_lib::{
    client::Client,
    realm::{RealmConfig, Realm},
    governance::Governance,
    proposal::Proposal,
    addin_fixed_weights::{VoterWeight, AddinFixedWeights},
    addin_vesting::AddinVesting,
};

pub struct Info {
    pub vesting_amount: u64,
    pub unlocked_amount: u64,
    pub total_amount: u64,
}

pub struct TokenDistribution<'a> {
    pub resolver: &'a AccountOwnerResolver<'a>,
    pub extra_token_accounts: &'a[ExtraTokenAccount],
    pub voter_list: Vec<VoterWeight>,
    pub info: Info,
}

impl<'a> TokenDistribution<'a> {
    pub fn new(fixed_weight_addin: &AddinFixedWeights, resolver: &'a AccountOwnerResolver, extra_token_accounts: &'a [ExtraTokenAccount]) -> Result<Self,ScriptError> {

        let params = fixed_weight_addin.get_params()?;
        let unlocked_amount = params.get("PARAM_EXTRA_TOKENS").ok_or(StateError::InvalidVoterList)?.parse::<u64>().unwrap();

        let voter_list = fixed_weight_addin.get_voter_list()?;
        let vesting_amount = voter_list.iter().map(|v| v.weight).sum::<u64>();

        let total_amount = unlocked_amount + vesting_amount;
        println!("Vesting: {}.{:09}, unlocked: {}.{:09}, total: {}.{:09}",
                vesting_amount/TOKEN_MULT, vesting_amount%TOKEN_MULT,
                unlocked_amount/TOKEN_MULT, unlocked_amount%TOKEN_MULT,
                total_amount/TOKEN_MULT, total_amount%TOKEN_MULT);


        Ok(Self {
            resolver,
            extra_token_accounts,
            voter_list,
            info: Info {
                vesting_amount,
                unlocked_amount,
                total_amount,
            },
        })
    }

    pub fn get_unique_owners(&self) -> Vec<AccountOwner> {
        let mut unique_owners: Vec<AccountOwner> = Vec::new();
        for extra_account in self.extra_token_accounts.iter() {
            if unique_owners.iter().find(|&u| *u == extra_account.owner).is_none() {
                unique_owners.push(extra_account.owner);
            }
        }
        unique_owners
    }

    pub fn get_special_accounts(&self) -> Vec<Pubkey> {
        let unique_owners = self.get_unique_owners();
        unique_owners.iter().map(|v| self.resolver.get_owner_pubkey(&v).unwrap()).collect()
    }

    pub fn validate(&self) -> Result<(),ScriptError> {
        let mut result = true;

        // 1. check sum extra_token_account with NoLockup equals extra_tokens
        let unlocked_amount = self.extra_token_accounts.iter()
                .filter_map(|v| if !v.lockup.is_locked() {Some(v.amount)} else {None})
                .sum::<u64>();
        if self.info.unlocked_amount != unlocked_amount {
            println!(" unlocked_amount {} doesn't equal sum of amounts in extra_token_accounts {}",
                self.info.unlocked_amount, unlocked_amount);
            result = false;
        }

        // 2. for each MultiSig exists corresponded record in voter_list with appropriate amount
        for owner in self.get_unique_owners().iter() {
            let locked_amount = self.extra_token_accounts.iter()
                .filter_map(|v| if v.lockup.is_locked() && v.owner == *owner {Some(v.amount)} else {None})
                .sum::<u64>();
            let owner_address = self.resolver.get_owner_pubkey(&owner).unwrap();
            print!(" {:45} {:10}.{:09} {:?}:  ", owner_address.to_string(), locked_amount/TOKEN_MULT, locked_amount%TOKEN_MULT, owner);
            let voter_item = self.voter_list.iter().find(|v| v.voter == owner_address);
            if locked_amount > 0 {
                if let Some(voter_item) = voter_item {
                    if voter_item.weight == locked_amount {
                        println!("correct");
                    } else {
                        println!("invalid amount {}", voter_item.weight);
                        result = false;
                    }
                } else {
                    println!("missed in voter_list");
                    result = false;
                };
            } else {
                if let Some(_) = voter_item {
                    println!("voter exists");
                    result = false;
                } else {
                    println!("no locked tokens");
                }
            }
        }

        if result == false {
            return Err(StateError::InvalidVoterList.into());
        }
        Ok(())
    }
}
