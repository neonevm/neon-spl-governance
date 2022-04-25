#![deny(missing_docs)]
#![cfg(all(target_arch = "bpf", not(feature = "no-entrypoint")))]
//! Governance VoterWeight Addin program

use spl_governance_addin_fixed_weights::entrypoint::process_instruction;
solana_program::entrypoint!(process_instruction);
