macro_rules! voter_weight_array {
    ($identifier:ident, [ $(($value_pubkey:expr,$value_weight:expr),)* ]) => {
        /// Voter Weight List
        pub static $identifier: [(::solana_sdk::pubkey::Pubkey,u64); [$(($value_pubkey,$value_weight),)*].len()] = [
            $((::solana_sdk::pubkey!($value_pubkey),$value_weight),)*
        ];
    };
}

voter_weight_array!(
    DISTRIBUTION_LIST,
    [
        ("FcTRG2o9uiJQPSZhJufb6cDkYXpmdEz54m952TmireW",  9_000_000),
        ("xaEuzqGFrUvjiEokdzpC2HAKotJFumGYo2YmNTS4eFZ",  2_000_000),
        ("EnCMdFj7eMxMfK2KhXEfbMURWRQ3AYdwxvTNKH91GD7p", 3_000_000),
        ("ACVPBh4FmfYGZu5jK6MYiAquaid2Vr3yjYFJ5RWe597v", 1_000_000),
        ("A3ujb32N9vnxsMp3stRGVpxACHNmxUedgpT8knShfnhs", 5_000_000),
    ]
);

