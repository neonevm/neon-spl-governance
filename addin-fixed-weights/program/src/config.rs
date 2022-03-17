//! CONFIG MODULE

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

        voter_weight_array!(
            VOTER_LIST,
            [
                ("FcTRG2o9uiJQPSZhJufb6cDkYXpmdEz54m952TmireW",  9000000),
                ("xaEuzqGFrUvjiEokdzpC2HAKotJFumGYo2YmNTS4eFZ",  2000000),
                ("EnCMdFj7eMxMfK2KhXEfbMURWRQ3AYdwxvTNKH91GD7p", 3000000),
            ]
        );

    } else {

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
