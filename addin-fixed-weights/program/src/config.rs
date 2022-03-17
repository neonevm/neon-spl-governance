//! CONFIG MODULE
// #![allow(clippy::use_self,clippy::nursery)]
// #![allow(clippy::cast_precision_loss)]
// #![allow(clippy::cast_sign_loss)]
// #![allow(clippy::cast_possible_truncation)]

use cfg_if::cfg_if;

macro_rules! voter_weight_array {
    ($identifier:ident, [ $(($value_pubkey:expr,$value_weight:expr),)* ]) => {
        /// Voter Weight List
        pub static $identifier: [(::solana_program::pubkey::Pubkey,u64); [$(($value_pubkey,$value_weight),)*].len()] = [
            $((::solana_program::pubkey!($value_pubkey),$value_weight),)*
        ];
    };
}

cfg_if! {
    if #[cfg(feature = "mainnet")] {

        /// Governed Realm ID
        pub const REALM_ID: &'static str = "4iAx57Vh5Y3Fu1pZET4g7AnQH1yLMGsnJDUht7AExYTY";
        /// Governing Community Mint
        pub const GOVERNING_MINT: &'static str = "3vxj94fSd3jrhaGAwaEKGDPEwn5Yqs81Ay5j1BcdMqSZ";

        // <<< Why we need declare this id here?
        solana_program::declare_id!("56cFVhzLFKuvRXQW68ACLpcbJonZeUBNDdLdZoo5fGnB");

        voter_weight_array!(
            VOTER_LIST,
            [
                ("FcTRG2o9uiJQPSZhJufb6cDkYXpmdEz54m952TmireW",  5000),
                ("xaEuzqGFrUvjiEokdzpC2HAKotJFumGYo2YmNTS4eFZ",  2000),
                ("EnCMdFj7eMxMfK2KhXEfbMURWRQ3AYdwxvTNKH91GD7p", 3000),
            ]
        );

    } else if #[cfg(feature = "alpha")] {

        /// Governed Realm ID
        pub const REALM_ID: &'static str = "4iAx57Vh5Y3Fu1pZET4g7AnQH1yLMGsnJDUht7AExYTY";
        /// Governing Community Mint
        pub const GOVERNING_MINT: &'static str = "3vxj94fSd3jrhaGAwaEKGDPEwn5Yqs81Ay5j1BcdMqSZ";

        solana_program::declare_id!("56cFVhzLFKuvRXQW68ACLpcbJonZeUBNDdLdZoo5fGnB");

        voter_weight_array!(
            VOTER_LIST,
            [
                ("FcTRG2o9uiJQPSZhJufb6cDkYXpmdEz54m952TmireW",  5000),
                ("xaEuzqGFrUvjiEokdzpC2HAKotJFumGYo2YmNTS4eFZ",  2000),
                ("EnCMdFj7eMxMfK2KhXEfbMURWRQ3AYdwxvTNKH91GD7p", 3000),
            ]
        );

    } else if #[cfg(feature = "testnet")] {

        /// Governed Realm ID
        pub const REALM_ID: &'static str = "4iAx57Vh5Y3Fu1pZET4g7AnQH1yLMGsnJDUht7AExYTY";
        /// Governing Community Mint
        pub const GOVERNING_MINT: &'static str = "3vxj94fSd3jrhaGAwaEKGDPEwn5Yqs81Ay5j1BcdMqSZ";
        
        solana_program::declare_id!("56cFVhzLFKuvRXQW68ACLpcbJonZeUBNDdLdZoo5fGnB");

        voter_weight_array!(
            VOTER_LIST,
            [
                ("FcTRG2o9uiJQPSZhJufb6cDkYXpmdEz54m952TmireW",  5000),
                ("xaEuzqGFrUvjiEokdzpC2HAKotJFumGYo2YmNTS4eFZ",  2000),
                ("EnCMdFj7eMxMfK2KhXEfbMURWRQ3AYdwxvTNKH91GD7p", 3000),
            ]
        );

    } else if #[cfg(feature = "devnet")] {

        /// Governed Realm ID
        pub const REALM_ID: &'static str = "4iAx57Vh5Y3Fu1pZET4g7AnQH1yLMGsnJDUht7AExYTY";
        /// Governing Community Mint
        pub const GOVERNING_MINT: &'static str = "3vxj94fSd3jrhaGAwaEKGDPEwn5Yqs81Ay5j1BcdMqSZ";
        
        solana_program::declare_id!("56cFVhzLFKuvRXQW68ACLpcbJonZeUBNDdLdZoo5fGnB");

        voter_weight_array!(
            VOTER_LIST,
            [
                ("FcTRG2o9uiJQPSZhJufb6cDkYXpmdEz54m952TmireW",  5000),
                ("xaEuzqGFrUvjiEokdzpC2HAKotJFumGYo2YmNTS4eFZ",  2000),
                ("EnCMdFj7eMxMfK2KhXEfbMURWRQ3AYdwxvTNKH91GD7p", 3000),
            ]
        );

    } else {

        /// Governed Realm ID
        pub const REALM_ID: &'static str = "4iAx57Vh5Y3Fu1pZET4g7AnQH1yLMGsnJDUht7AExYTY";
        /// Governing Community Mint
        pub const GOVERNING_MINT: &'static str = "3vxj94fSd3jrhaGAwaEKGDPEwn5Yqs81Ay5j1BcdMqSZ";
        
        solana_program::declare_id!("56cFVhzLFKuvRXQW68ACLpcbJonZeUBNDdLdZoo5fGnB");

        voter_weight_array!(
            VOTER_LIST,
            [
                ("FcTRG2o9uiJQPSZhJufb6cDkYXpmdEz54m952TmireW",  9000000),
                ("xaEuzqGFrUvjiEokdzpC2HAKotJFumGYo2YmNTS4eFZ",  2000000),
                ("EnCMdFj7eMxMfK2KhXEfbMURWRQ3AYdwxvTNKH91GD7p", 3000000),
                ("ACVPBh4FmfYGZu5jK6MYiAquaid2Vr3yjYFJ5RWe597v", 1000000),
                ("A3ujb32N9vnxsMp3stRGVpxACHNmxUedgpT8knShfnhs", 5000000),
            ]
        );
    
    }
}

/// Voter Weight Addin Fixed Version
pub const VOTER_WEIGHT_SEED_VERSION: u8 = 1;    // <<< It is unneeded
