use crate::{
    errors::{StateError, ScriptError},
};
use solana_sdk::{
    signer::{
        Signer,
        keypair::Keypair,
    },
    instruction::Instruction,
    transaction::Transaction,
    signature::Signature,
};

use spl_governance::{
    state::{
        proposal_transaction::InstructionData,
    },
};

use governance_lib::{
    client::Client,
    proposal::Proposal,
    token_owner::TokenOwner,
};


macro_rules! println_bold {
    ($format:literal, $($item:expr),*) => {
        println!(concat!("\x1b[1m", $format, "\x1b[0m"), $($item),*)
    };
    ($format:literal) => {
        println!(concat!("\x1b[1m", $format, "\x1b[0m"))
    };
}

macro_rules! println_item {
    ($format:literal, $($item:expr),*) => {
        println!(concat!("\x1b[34m", $format, "\x1b[0m"), $($item),*)
    }
}

macro_rules! println_correct {
    ($format:literal, $($item:expr),*) => {
        println!(concat!("\x1b[32m", $format, "\x1b[0m"), $($item),*)
    }
}

macro_rules! println_update {
    ($format:literal, $($item:expr),*) => {
        println!(concat!("\x1b[33m", $format, "\x1b[0m"), $($item),*)
    }
}

macro_rules! println_error {
    ($format:literal, $($item:expr),*) => {
        println!(concat!("\x1b[31m", $format, "\x1b[0m"), $($item),*)
    }
}

pub struct TransactionExecutor<'a> {
    pub client: &'a Client<'a>,
    pub setup: bool,
    pub verbose: bool,
}

impl<'a> TransactionExecutor<'a> {
    pub fn check_and_create_object<T,V,C>(&self, name: &str,
            object: Option<T>, verify: V, create: C) -> Result<Option<Signature>,ScriptError>
    where
        V: FnOnce(&T) -> Result<Option<Transaction>,ScriptError>,
        C: FnOnce() -> Result<Option<Transaction>,ScriptError>,
        T: std::fmt::Debug,
    {
        if let Some(data) = object {
            if self.verbose {println_item!("{}: {:?}", name, data);};
            match verify(&data) {
                Ok(None) => {
                    println_correct!("{}: correct", name);
                },
                Ok(Some(transaction)) => {
                    if self.setup {
                        let result = self.client.send_transaction(&transaction);
                        match result {
                            Ok(signature) => {
                                println_update!("{}: updated in trx {}", name, signature);
                                return Ok(Some(signature));
                            },
                            Err(error) => {
                                println_error!("{}: failed update with {}", name, error);
                                return Err(error.into());
                            }
                        };
                    } else {
                        if self.verbose {println_item!("{}: {:?}", name, transaction);};
                        println_update!("{}: will be updated", name);
                    }
                },
                Err(error) => {
                    println_error!("{}: wrong object {:?}", name, error);
                    if self.setup {return Err(error);}
                }
            }
        } else {
            match create() {
                Ok(None) => {
                    println_correct!("{}: missed ok", name);
                },
                Ok(Some(transaction)) => {
                    if self.setup {
                        let result = self.client.send_transaction(&transaction);
                        match result {
                            Ok(signature) => {
                                println_update!("{}: created in trx {}", name, signature);
                                return Ok(Some(signature));
                            },
                            Err(error) => {
                                println_error!("{}: failed create with {}", name, error);
                                return Err(error.into());
                            }
                        };
                    } else {
                        if self.verbose {println_item!("{}: {:?}", name, transaction);};
                        println_update!("{}: will be created", name);
                    }
                },
                Err(error) => {
                    println_error!("{}: can't be created: {:?}", name, error);
                    if self.setup {return Err(error);}
                }
            }
        }
        Ok(None)
    }
}

pub struct TransactionCollector<'a> {
    pub client: &'a Client<'a>,
    pub setup: bool,
    pub verbose: bool,
    pub name: String,
    instructions: Vec<Instruction>,
    signers: Vec<&'a dyn Signer>,
}

impl<'a> TransactionCollector<'a> {
    pub fn new(client: &'a Client<'a>, setup: bool, verbose: bool, name: &str) -> Self {
        println!("{}: collect instructions...", name);
        Self {
            client,
            setup,
            verbose,
            name: name.to_string(),
            instructions: Vec::new(),
            signers: Vec::new(),
        }
    }

    fn add_signers(&mut self, keypairs: Vec<&'a Keypair>) {
        for keypair in keypairs {
            self.signers.push(keypair as &dyn Signer);
        }
    }

    pub fn check_and_create_object<T,V,C>(&mut self, name: &str,
            object: Option<T>, verify: V, create: C) -> Result<(),ScriptError>
    where
        V: FnOnce(&T) -> Result<Option<(Vec<Instruction>,Vec<&'a Keypair>)>,ScriptError>,
        C: FnOnce() -> Result<Option<(Vec<Instruction>,Vec<&'a Keypair>)>,ScriptError>,
        T: std::fmt::Debug,
    {
        if let Some(data) = object {
            if self.verbose {println_item!("{}: {:?}", name, data);};
            match verify(&data) {
                Ok(None) => {
                    println_correct!("{}: correct", name);
                },
                Ok(Some((instructions,signers,))) => {
                    if self.verbose {println_item!("{}: {:?}", name, instructions);};
                    println_update!("{}: update instructions was added", name);
                    self.instructions.extend(instructions);
                    self.add_signers(signers);
                },
                Err(error) => {
                    println_error!("{}: wrong object {:?}", name, error);
                    if self.setup {return Err(error);}
                }
            }
        } else {
            match create() {
                Ok(None) => {
                    println_correct!("{}: missed ok", name);
                },
                Ok(Some((instructions,signers,))) => {
                    if self.verbose {println_item!("{}: {:?}", name, instructions);};
                    println_update!("{}: create instructions was added", name);
                    self.instructions.extend(instructions);
                    self.add_signers(signers);
                },
                Err(error) => {
                    println_error!("{}: can't be created: {:?}", name, error);
                    if self.setup {return Err(error);}
                }
            }
        }
        Ok(())
    }

    pub fn execute_transaction(&self) -> Result<Option<Signature>,ScriptError> {
        if self.setup && !self.instructions.is_empty() {
            let result = if self.signers.is_empty() {
                self.client.send_and_confirm_transaction_with_payer_only(&self.instructions)
            } else {
                self.client.send_and_confirm_transaction(&self.instructions, &self.signers)
            };
            match result {
                Ok(signature) => {
                    println_update!("{}: processed in trx {}", self.name, signature);
                    Ok(Some(signature))
                },
                Err(error) => {
                    println_error!("{}: failed process with {}", self.name, error);
                    Err(error.into())
                }
            }
        } else {
            println_correct!("{}: no instructions for execute", self.name);
            Ok(None)
        }
    }
}

pub struct ProposalTransactionInserter<'a> {
    pub proposal: &'a Proposal<'a>,
    pub creator_keypair: &'a Keypair,
    pub creator_token_owner: &'a TokenOwner<'a>,
    pub hold_up_time: u32,
    pub setup: bool,
    pub verbose: bool,

    pub proposal_transaction_index: u16,
}

impl<'a> ProposalTransactionInserter<'a> {
    pub fn insert_transaction_checked(&mut self, name: &str, instructions: Vec<InstructionData>) -> Result<(), ScriptError> {
        use borsh::BorshSerialize;
        let mut extra_signers = vec!();
        for (idx,instruction) in instructions.iter().enumerate() {
            println!("Instruction {}", idx);
            extra_signers.extend(
                instruction.accounts.iter().filter(|a| {
                    if a.is_signer { println!("Instruction signer {:?}", a.pubkey) };
                    a.is_signer && a.pubkey != self.proposal.governance.governance_address
                })
            );
        }
        if !extra_signers.is_empty() {
            let error = StateError::RequireAdditionalSigner(extra_signers[0].pubkey);
            println_error!("Proposal transaction '{}'/{}: {:?}", name, self.proposal_transaction_index, error);
            if self.setup {return Err(error.into());}
        }

        if let Some(transaction_data) = self.proposal.get_proposal_transaction_data(0, self.proposal_transaction_index)? {
            if self.verbose {println_item!("Proposal transaction '{}'/{}: {:?}", name, self.proposal_transaction_index, transaction_data);};
            if transaction_data.instructions != instructions {
                let error = StateError::InvalidProposalTransaction(self.proposal_transaction_index);
                println_error!("Proposal transaction '{}'/{}: {:?}", name, self.proposal_transaction_index, error);
                if self.setup {return Err(error.into())}
            } else {
                println_correct!("Proposal transaction '{}'/{} correct", name, self.proposal_transaction_index);
            }
        } else if self.setup {
            let signature = self.proposal.insert_transaction(
                    self.creator_keypair,
                    self.creator_token_owner,
                    0, self.proposal_transaction_index, self.hold_up_time,
                    instructions
                )?;
            println_update!("Proposal transaction '{}'/{} was inserted in trx: {}", name, self.proposal_transaction_index, signature);
        } else {
            println_update!("Proposal transaction '{}'/{} will be inserted", name, self.proposal_transaction_index);
            if self.verbose {
                for instruction in &instructions {
                    println_item!("{:?}", instruction);
                    println_bold!("BASE64: {}", base64::encode(instruction.try_to_vec()?));
                }
            }
        }
        self.proposal_transaction_index += 1;
        Ok(())
    }
}
