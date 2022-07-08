use crate::prelude::*;

pub struct Configuration<'a> {
    pub wallet: &'a Wallet,
    pub client: &'a Client<'a>,

    pub send_trx: bool,
    pub verbose: bool,
    pub testing: bool,
    pub start_time: NaiveDateTime,

    pub startup_realm_config: RealmConfig,
    pub working_realm_config: RealmConfig,
    pub community_governance_config: GovernanceConfig,
    pub emergency_governance_config: GovernanceConfig,
    pub maintenance_governance_config: GovernanceConfig,

    pub multi_sigs: Vec<MultiSig>,
    pub token_distribution: Vec<ExtraTokenAccount>,
}

macro_rules! token_distribution {
    ([ $(($amount:expr, $lockup:expr, $owner:expr),)*]) => {
        vec![ $(ExtraTokenAccount::new($amount * TOKEN_MULT, $lockup, $owner),)*]
    };
}

impl<'a> Configuration<'a> {
    pub fn create_from_config(
        wallet: &'a Wallet,
        client: &'a Client,
        send_trx: bool,
        verbose: bool,
        config: &ConfigFile,
    ) -> Self {
        Self::create(
            wallet,
            client,
            send_trx,
            verbose, 
            config.testing,
            Some(config.start_time),
        )
    }

    pub fn create(
        wallet: &'a Wallet,
        client: &'a Client,
        send_trx: bool,
        verbose: bool,
        testing: bool,
        start_time: Option<NaiveDateTime>,
    ) -> Self {
        let account = |seed, program| wallet.account_by_seed(seed, program);
        Self {
            wallet,
            client,
            send_trx,
            verbose,
            testing,
            start_time: start_time.unwrap_or_else(|| {
                if testing {
                    Utc::now().naive_utc()
                } else {
                    Utc::today().naive_utc().and_hms(0, 0, 0)
                }
            }),
            startup_realm_config: RealmConfig {
                council_token_mint: None,
                community_voter_weight_addin: Some(wallet.fixed_weight_addin_id),
                max_community_voter_weight_addin: Some(wallet.fixed_weight_addin_id),
                min_community_weight_to_create_governance: 1_000_000 * TOKEN_MULT,
                community_mint_max_vote_weight_source:
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
            },
            working_realm_config: RealmConfig {
                council_token_mint: None,
                community_voter_weight_addin: Some(wallet.vesting_addin_id),
                max_community_voter_weight_addin: None,
                min_community_weight_to_create_governance: 1_000_000 * TOKEN_MULT,
                community_mint_max_vote_weight_source:
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
            },
            community_governance_config: GovernanceConfig {
                vote_threshold_percentage: VoteThresholdPercentage::YesVote(1),
                min_community_weight_to_create_proposal: 3_000 * TOKEN_MULT,
                min_transaction_hold_up_time: (if testing {
                    Duration::minutes(1)
                } else {
                    Duration::days(2)
                })
                .num_seconds() as u32,
                max_voting_time: (if testing {
                    Duration::minutes(3)
                } else {
                    Duration::days(1)
                })
                .num_seconds() as u32,
                vote_tipping: VoteTipping::Disabled,
                proposal_cool_off_time: 0,
                min_council_weight_to_create_proposal: 0,
            },
            emergency_governance_config: GovernanceConfig {
                vote_threshold_percentage: VoteThresholdPercentage::YesVote(9),
                min_community_weight_to_create_proposal: 1_000_000 * TOKEN_MULT,
                min_transaction_hold_up_time: 0,
                max_voting_time: (if testing {
                    Duration::minutes(3)
                } else {
                    Duration::days(1)
                })
                .num_seconds() as u32,
                vote_tipping: VoteTipping::Disabled,
                proposal_cool_off_time: 0,
                min_council_weight_to_create_proposal: 0,
            },
            maintenance_governance_config: GovernanceConfig {
                vote_threshold_percentage: VoteThresholdPercentage::YesVote(1),
                min_community_weight_to_create_proposal: 200_000 * TOKEN_MULT,
                min_transaction_hold_up_time: 0,
                max_voting_time: (if testing {
                    Duration::minutes(3)
                } else {
                    Duration::days(1)
                })
                .num_seconds() as u32,
                vote_tipping: VoteTipping::Disabled,
                proposal_cool_off_time: 0,
                min_council_weight_to_create_proposal: 0,
            },
            multi_sigs: vec![
                MultiSig {
                    name: "1".to_string(),
                    threshold: 2,
                    governed_accounts: vec![],
                    signers: vec![
                        pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
                        pubkey!("6tAoNNAB6sXMbt8phMjr46noQ5T18GnnkBftWcw1HfCW"),
                        pubkey!("EsyJ9wzg2VTCCfHmnyi7ePE9LU368iVCrEd4LZeDYMzJ"),
                    ],
                },
                MultiSig {
                    name: "2".to_string(),
                    threshold: 2,
                    governed_accounts: vec![],
                    signers: vec![
                        pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
                        pubkey!("H3cAYot4UJuY1jQhn8FtpeP4fHia3SXtvuKYaov7KMA9"),
                        pubkey!("8ZjncH1eKhJMmwqymWwPEAEaPjTSt91R1gMwx2bMyZqC"),
                    ],
                },
                MultiSig {
                    name: "4".to_string(),
                    threshold: 2,
                    governed_accounts: vec![account("MSIG_4.1", &spl_token::id())],
                    signers: if testing {
                        vec![
                            pubkey!("tstUPDM1tDgRgC8KALbXQ3hJeKQQTxDywyDVvxv51Lu"),
                            pubkey!("tstTLYLzy9Q5meFUmhhiXfnaGai96hc7Ludu3gQz8nh"),
                            wallet.payer_keypair.pubkey(),
                        ]
                    } else {
                        vec![
                            pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
                            pubkey!("C16ojhtyjzqenxHcg9hNjhAwZhdLJrCBKavfc4gqa1v3"),
                            pubkey!("4vdhzpPYPABJe9WvZA8pFzdbzYaHrj7yNwDQmjBCtts5"),
                        ]
                    },
                },
                MultiSig {
                    name: "5".to_string(),
                    threshold: 2,
                    governed_accounts: vec![],
                    signers: vec![
                        pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
                        pubkey!("2Smf7Kyskf3VXUKUB16GVgCizW4qDhvRREGCLcHt7bJV"),
                        pubkey!("EwNeN5ixjqNmBNGbVKDHd1iipStGhMC9u5yGsq7zsw6L"),
                    ],
                },
            ],
            token_distribution: if testing {
                token_distribution!([
                    (187_762_400, Lockup::For1year1yearLinear, AccountOwner::MultiSig("1")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("1")),
    
                    ( 60_000_000, Lockup::For4Years,           AccountOwner::MultiSig("2")),
    
                    (210_000_000, Lockup::NoLockup,            AccountOwner::BothGovernance),
                    ( 80_000_000, Lockup::NoLockup,            AccountOwner::Key("tstzQJwDhrPNSmqtV5rmC26xbbeBf56xFz9wpyTV7tW")),
    
                    (149_000_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("4")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("4.1")),
    
                    (150_000_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("5")),
    
                    ( 40_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("keyBcYtD2h6PTWvx8Ewwrak2w72hoM5VdBNbwNqwmuX")),
                    ( 40_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("keyNBBcjcqbTGiEyihcS6FodYh68sPkWs6RG5yfLDCN")),
                    ( 20_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("HFTXn5oTGo9dgSJfgCAU59caaiwLWx1ZDy7VjE1qu4w")),
                    ( 20_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("tst18qx7Kd3ELAsM3Qxn4nKNRZeg26Zi7GKGHaeWFm6")),
                    (  4_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("tst6RG7t1J8XN3NYLNHkA3acfZcjurhurG7Kk3gAw9k")),
                    (  3_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("tst6YyNdi4nGhHAew2N9GKLfVE2gp99y4y4XNAo52qs")),
                    (  3_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("tstCUGzLUYcuuDVGgAzwi334fDhDS2asqHqcurDqhrS")),
                    (  2_400_000, Lockup::For1year1yearLinear, AccountOwner::Key("tstD4uLc8NE7JYXgKdamx8f3JpC3usDLcbiyDpdbrxJ")),
                    (  2_200_000, Lockup::For1year1yearLinear, AccountOwner::Key("tstKY6DqH9u7uwVw2qa3pgfJNoKWm12e82JRuccBwvV")),
                    (  2_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("tstnGPJyiQMUJqZxqvK4857xeWp7ZrczqZwsf4SB7R8")),
                    (  1_440_000, Lockup::For1year1yearLinear, AccountOwner::Key("tstPSu5sHGrZQraZ3Ef8MFmeSfKWxQSwQQviv7cYWwb")),
                    ( 23_197_600, Lockup::For1year1yearLinear, AccountOwner::Key("11111111111111111111111111111111")),
                ])
            } else {
                token_distribution!([
                    (187_762_400, Lockup::For1year1yearLinear, AccountOwner::MultiSig("1")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("1")),
    
                    ( 60_000_000, Lockup::For4Years,           AccountOwner::MultiSig("2")),
    
                    (290_000_000, Lockup::NoLockup,            AccountOwner::BothGovernance),
    
                    (145_250_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("4")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("4.1")),
                    (  1_250_000, Lockup::For1year1yearLinear, AccountOwner::Key("6tTYuzuZN31iHdFLQCjmoxqatoWMYpFM8qfXGo89AWK1")),
                    (  1_250_000, Lockup::For1year1yearLinear, AccountOwner::Key("27HjgEX8WxtmSMSogVLZJUKP3GrRN6A7zmgb7JZR3tMg")),
                    (  1_250_000, Lockup::For1year1yearLinear, AccountOwner::Key("7XYeZmjzjefApSCswonsr2NsNB81YmHskPwffzBtmqrH")),
    
                    ( 42_500_000, Lockup::For1year1yearLinear, AccountOwner::Key("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8")),
                    ( 42_500_000, Lockup::For1year1yearLinear, AccountOwner::Key("EaKk38a3S4XKum2YM8gEX6KSaW9CE9AbbUaW5xQpoTTC")),
                    ( 42_500_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("5")),
                    (  7_500_000, Lockup::For1year1yearLinear, AccountOwner::Key("DEskk1zj5w8hvfMf5rSkxUZLcZf7sGrf5G49C7wNQNce")),
                    (  7_500_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("5")),
                    (  3_750_000, Lockup::For1year1yearLinear, AccountOwner::Key("SMyuMjKsBJeHbqUerkpduW1TfwErdBLrXTLsx7BrgMm")),
                    (  3_750_000, Lockup::For1year1yearLinear, AccountOwner::MultiSig("5")),
    
                    ( 40_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("69GA1mJCEqyYxj57CCeamy2WGx7wM3ABEwuUFMmatu2d")),
                    ( 40_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("5CmWF9DMrcCtpuw3g1rnx9zYLX39bNwEX7dSEeaKFPPf")),
                    ( 20_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("HFTXn5oTGo9dgSJfgCAU59caaiwLWx1ZDy7VjE1qu4w")),//{{{
                    ( 20_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("6C3PmbTHi5xFZMW7c66xLvbQciVddbEFWJGpHVz1LGxX")),
                    (  4_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("FZXQwFXdHk4HaMhSKczdt3C4UseJpJiBn9hm8UHJWb8G")),
                    (  3_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("FYeKmwTpJGqZ2pzvSzzDAmwipT2J2AD3BiTdUdqTUbVv")),
                    (  3_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("GrjW2DtUd7WxVz1NYwguFpue5pHtVx6kqADjJqMNnwVD")),
                    (  2_400_000, Lockup::For1year1yearLinear, AccountOwner::Key("CthYJnfjz9YELmZPYVJn2A1yhpmDTLUdWKuhYwEyCYZz")),
                    (  2_200_000, Lockup::For1year1yearLinear, AccountOwner::Key("F5hTRH4Lu6fRkn6Scc5ogDdoFupz9oRM9fNHQfLRbehV")),
                    (  2_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("73dy4VtrmYoYwo2Q3q5soGwXhKngGrgnqvL5GEryC5Lk")),
                    (  1_440_000, Lockup::For1year1yearLinear, AccountOwner::Key("5cs6vpXKuKKNbzpDgzRSbdMdxej7qF3hQ5ccg7L7HV4n")),
                    (  1_400_000, Lockup::For1year1yearLinear, AccountOwner::Key("BWpZ4LwWg3ZV2fgQW6hxP1SmMMhwQkaKqtfq5xcx4zkd")),
                    (  1_200_000, Lockup::For1year1yearLinear, AccountOwner::Key("GtH2jmBppV8VAtbEKAngGnn6h9esv9MRtgqKARFDFrbf")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("ASLWzyVKsmYWHY8gYRVxJtBYd3UYkg19jeo8Wrhpb3rf")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("AXV1sKb86s1PfYSJ78YMKwq4ejhjKtvYZh9RhyrEyuB6")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("AUzMEoeKiLQWWGcZ38M6nTMKWr8SpeYViHSQtm9LfHue")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("EYYPcCewaYKhEtA7NymW83En8it7PmaxDiDqVEaDPMea")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("6Uqh9XMvx3L4g82W1qoduZUt3DeG19PPr6FM3gjgwYAg")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("9RKL5qesjTz6YNRRoehvu6qz9HCVp3ToV5w4Dz5aq8Xv")),
                    (  1_000_000, Lockup::For1year1yearLinear, AccountOwner::Key("GCtjNA958Nb1w3noeGHm5EZNh3pp6XyLjLB7yrP1wWRH")),
                    (    857_600, Lockup::For1year1yearLinear, AccountOwner::Key("H9LTEpFCiM8jxaEYrFyqhLaMEtjkNTMbpMimctHoeQDo")),
                    (    840_000, Lockup::For1year1yearLinear, AccountOwner::Key("4dDgPddsnJHznoEoBxpukYT6YmF5JXZkt5tKV7FSxhfs")),
                    (    640_000, Lockup::For1year1yearLinear, AccountOwner::Key("9kbwhpRLdXrsJgMkyEmtqHEn12gNL4pfW3WmX66gAqaT")),
                    (    600_000, Lockup::For1year1yearLinear, AccountOwner::Key("AN7zkfE7MWVcSEPHqGFrMNKrKgUntxpdgVBAH6YuThCp")),
                    (    600_000, Lockup::For1year1yearLinear, AccountOwner::Key("GquqZs6x4gQgpqyRnengHHWCKowrKWwAR2RhMmvdwBPV")),
                    (    600_000, Lockup::For1year1yearLinear, AccountOwner::Key("QuLWFDsYsrnjvgkT4XKYn5p2tkr4h6UUBB2Q8QfZu9E")),
                    (    600_000, Lockup::For1year1yearLinear, AccountOwner::Key("GwF6shT6ahXhHrwRg9if3YTfLJfwXNX2ZLVFGfqe2jvH")),
                    (    500_000, Lockup::For1year1yearLinear, AccountOwner::Key("FrPa7KM25m5fqbxW7VVHBovUZdi7hp3HTemwJhHa44jg")),
                    (    500_000, Lockup::For1year1yearLinear, AccountOwner::Key("2AuvzJ8jK9RrYcanKxLoamUiffNP7Vm6JdYxWCq74WFj")),
                    (    480_000, Lockup::For1year1yearLinear, AccountOwner::Key("2jUVAfmhwN3znyKZ95RZLzGi2x7ghXgj3pZy4Aij163t")),
                    (    400_000, Lockup::For1year1yearLinear, AccountOwner::Key("BAHbicz9bMb2qEjPgHgU32M711QCRVRK4xSDKBcETs9d")),
                    (    400_000, Lockup::For1year1yearLinear, AccountOwner::Key("6cYPAViwm7XBDH6RKrReM8QkSiDrWbxUzDEa2sKmNGL1")),
                    (    400_000, Lockup::For1year1yearLinear, AccountOwner::Key("DXJgRvrkafSzRL7kVq8f23NbXLHcBEQKe9W9zZJvAUEe")),
                    (    400_000, Lockup::For1year1yearLinear, AccountOwner::Key("3ovnDC6Md3F2i8RT3MvZcS7QekmUq8jso1e3ke8MmY3a")),
                    (    400_000, Lockup::For1year1yearLinear, AccountOwner::Key("CcgxjNdLRx83Wrg2qhbgvCPCk8MhpBrevgcof15BZDmB")),
                    (    400_000, Lockup::For1year1yearLinear, AccountOwner::Key("6VzjkuiyMjipt4e3qw87mH6sHVeN8uxsY8qK8PxkZDYK")),
                    (    400_000, Lockup::For1year1yearLinear, AccountOwner::Key("2n9Rf5KJDVR4GKpW4JHGEaKLRw3z89uuWf27dLbQqPWZ")),
                    (    300_000, Lockup::For1year1yearLinear, AccountOwner::Key("8Fogg1kwSYzyZCRBbniVWeprPwg8s8yShtXKUwyjczpA")),
                    (    300_000, Lockup::For1year1yearLinear, AccountOwner::Key("EcBVA94VYUr6mAZqxGdB4QN787H5a3N3kd8tb4mCySs")),
                    (    300_000, Lockup::For1year1yearLinear, AccountOwner::Key("G8wh49cSsBQGioJKB5F6k9aXEkZMtY7Pjs1aZiSRhqMZ")),
                    (    240_000, Lockup::For1year1yearLinear, AccountOwner::Key("Fw9t5qZU7uCdQhRqcXayPbXCNNwEsR1ry4SWMBNDtVMB")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("2Yf2eKbaAHxFCNUGPCqaqsXzqxMRb14C9JSPTKD3wDMF")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("FK7Kw7WnJXjt2nBUwF5AH1omrBYsaxXWt9PXQSCDPfuT")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("Dnu8pp4ttSxrS4weQ3drG5c873P97NoGoSnkvZY8AAkB")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("6WYtgZjuHD1hPDiNtDDU7QQpWw576bxoSbQyd7WQoobq")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("3Kr99Jqaw1VRHecKHhvb7BxNL7ZgyvafqA5tTZqUAJgK")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("HaJfzhg3RdB9vueG2qmVW6ajjGSw3q3yVd3XF5MnELCz")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("D5ntoe2zA7b2GnjHXeLytkW1zoaaSxieSibH7NhQvBQ7")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("BgtCPrqwftgRy7yqAQSajd3woQK24E3RPfkfbtyB57km")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("D9bpPfFu2xPZJdKDKV8iJLyhhKZuaucCEcsR7cVNAYjP")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("8oGy9tu6KWcFa8SHoBEHSmMLmDdzEPw3PZxLEuybpKJ9")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("DSiaQPpLwY73tpcKHi65MxbTmBif56qH2mYkdtUEia1i")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("97vxiqEJQrpJZez7hGcCXNqfZ8k9qWQTMbvtDtgv8PL8")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("5jrkc3RvjE9NYwYts1TvNkqjNeSjpQLFB7Sohq6zxWwp")),
                    (    200_000, Lockup::For1year1yearLinear, AccountOwner::Key("Ee6wuzBuzBQ1ScLvASQuYygbRBEWUgq3oTQJyWjjGhJF")),
                    (    160_000, Lockup::For1year1yearLinear, AccountOwner::Key("5euPxonYkrwyJKmRQe1gjsADb8RqccosbThZ3yVSf2x8")),
                    (    160_000, Lockup::For1year1yearLinear, AccountOwner::Key("Be2Ec9REaFYRcuHhtLN9hfsniFWe19LEmy3xFnkrSmv5")),
                    (    100_000, Lockup::For1year1yearLinear, AccountOwner::Key("B5C7P4F6NySR7BJFWTBUdr9Z1QDivFEjd8oQdmydJiNu")),
                    (    100_000, Lockup::For1year1yearLinear, AccountOwner::Key("DY4v61XYV7Tmf9YVWkxvFLUn3V3rccukz6Jewq7CgGCN")),//}}}
                    (     80_000, Lockup::For1year1yearLinear, AccountOwner::Key("CgWTnErgdNAzKoeRSQ1NHcW2B5ij8yfZkquVNqV3AByW")),
                    (     40_000, Lockup::For1year1yearLinear, AccountOwner::Key("A7uc5dBwaz4BjFHDm7582MHviSQ2Rq58tcUV2PA5n2Xo")),
                ])
            }
        }
    }

    pub fn account_by_seed(&self, seed: &str, program: &Pubkey) -> Pubkey {
        self.wallet.account_by_seed(seed, program)
    }

    pub fn neon_multisig_address(&self) -> Pubkey {
        self.account_by_seed(&format!("{}_multisig", REALM_NAME), &spl_token::id())
    }

    pub fn get_schedule_size(&self, lockup: &Lockup) -> u32 {
        lockup.get_schedule_size()
    }

    pub fn get_schedule(&self, lockup: &Lockup, amount: u64) -> Vec<VestingSchedule> {
        if self.testing {
            lockup.get_testing_schedule(self.start_time, amount)
        } else {
            lockup.get_mainnet_schedule(self.start_time, amount)
        }
    }

    pub fn get_owner_address(&self, account_owner: &AccountOwner) -> Result<Pubkey, ScriptError> {
        use std::str::FromStr;
        match account_owner {
            AccountOwner::MainGovernance => {
                let realm = Realm::new(
                    self.client,
                    &self.wallet.governance_program_id,
                    REALM_NAME,
                    &self.wallet.community_pubkey,
                );
                let governance = realm.governance(&self.wallet.community_pubkey);
                Ok(governance.governance_address)
            }
            AccountOwner::EmergencyGovernance => {
                let realm = Realm::new(
                    self.client,
                    &self.wallet.governance_program_id,
                    REALM_NAME,
                    &self.wallet.governance_program_id,
                );
                let governance = realm.governance(&self.wallet.community_pubkey);
                Ok(governance.governance_address)
            }
            AccountOwner::BothGovernance => Ok(self.neon_multisig_address()),
            AccountOwner::MultiSig(name) => {
                let (msig_name, _governed_seed) = if name.contains('.') {
                    name.split_once('.').unwrap()
                } else {
                    (*name, "")
                };
                let msig = self.multi_sigs.iter().find(|v| v.name == *msig_name)
                    .ok_or_else(|| StateError::UnknownMultiSig(msig_name.to_string()))?;
                let seed: String = format!("MSIG_{}", msig_name);
                let msig_mint = self.account_by_seed(&seed, &spl_token::id());
                let msig_realm = Realm::new(
                    self.client,
                    &self.wallet.governance_program_id,
                    &seed,
                    &msig_mint,
                );
                let governed = if name.contains('.') {
                    let governed = self.account_by_seed(&format!("MSIG_{}", name), &spl_token::id());
                    if !msig.governed_accounts.iter().any(|v| *v == governed) {
                        return Err(StateError::UnknownMultiSigGoverned(
                            msig_name.to_string(),
                            governed
                        ).into())
                    };
                    governed
                } else {
                    msig_mint
                };
                let msig_governance = msig_realm.governance(&governed);
                Ok(msig_governance.governance_address)
            },
            AccountOwner::Key(pubkey) => Pubkey::from_str(pubkey).map_err(|e| e.into()),
        }
    }

    pub fn validate_fixed_weight_addin(&self, verbose: bool) -> Result<(), ScriptError> {
        let unique_owners = self.get_unique_vesting_owners()?;
        if verbose {
            for item in unique_owners.iter() {
                let owner = self.get_owner_address(&item.0)?;
                println!("{}.{:09} {} {:?}", item.1/TOKEN_MULT, item.1%TOKEN_MULT, owner, item.0);
            }
        }

        let fixed_weight_addin = AddinFixedWeights::new(self.client, self.wallet.fixed_weight_addin_id);
        let params = fixed_weight_addin.get_params()?;
        let unlocked_amount = params.get("PARAM_EXTRA_TOKENS")
                .ok_or(StateError::InvalidVoterList("Missing parameres in addin".to_string()))?.parse::<u64>().unwrap();
        let expected_unlocked_amount = self.get_unlocked_amount();

        if unlocked_amount != expected_unlocked_amount {
            let err_str = format!("invalid unlocked amount: actual {}.{:09}, expected {}.{:09}",
                unlocked_amount/TOKEN_MULT, unlocked_amount%TOKEN_MULT,
                expected_unlocked_amount/TOKEN_MULT, expected_unlocked_amount%TOKEN_MULT);
            return Err(StateError::InvalidVoterList(err_str).into());
        }

        let voter_list = fixed_weight_addin.get_voter_list()?;
        if voter_list.len() != unique_owners.len() {
            return Err(StateError::InvalidVoterList("Invalid voter_list length".to_string()).into());
        }

        for (i,(owner,amount)) in unique_owners.iter().enumerate() {
            let owner_address = self.get_owner_address(owner)?;
            if voter_list[i].weight != *amount || voter_list[i].voter != owner_address {
                let err_str = format!("Invalid voter on position {}: {:?}", i, voter_list[i]);
                return Err(StateError::InvalidVoterList(err_str).into());
            }
        }
        Ok(())
    }

    fn get_unlocked_amount(&self) -> u64 {
        let mut unlocked_amount = 0;
        for account in self.token_distribution.iter() {
            if !account.lockup.is_locked() {
                unlocked_amount += account.amount;
            }
        }
        unlocked_amount
    }

    pub fn get_total_amount(&self) -> u64 {
        let mut total_amount = 0;
        for account in self.token_distribution.iter() {
            total_amount += account.amount;
        }
        total_amount
    }

    pub fn get_unique_vesting_owners(&self) -> Result<Vec<(AccountOwner,u64)>, ScriptError> {
        let mut vesting_owners: Vec<(AccountOwner,u64)> = Vec::new();
        for account in self.token_distribution.iter() {
            if account.lockup.is_locked() {
                match vesting_owners.iter_mut().find(|v| v.0 == account.owner) {
                    Some(mut item) => item.1 += account.amount,
                    None => vesting_owners.push((account.owner, account.amount,)),
                }
            }
        }
        Ok(vesting_owners)
    }

    pub fn print_token_distribution(&self) -> Result<(), ScriptError> {
        let unlocked_amount = self.token_distribution.iter()
            .filter_map(|v| if v.lockup.is_locked() {None} else {Some(v.amount)})
            .sum::<u64>();
        let vesting_amount = self.token_distribution.iter()
            .filter_map(|v| if v.lockup.is_locked() {Some(v.amount)} else {None})
            .sum::<u64>();
        let total_amount = unlocked_amount + vesting_amount;

        println!("TOKEN DISTRIBUTION TABLE");
        println!("   VESTING: {:10}.{:09}", vesting_amount/TOKEN_MULT, vesting_amount%TOKEN_MULT);
        println!("  UNLOCKED: {:10}.{:09}", unlocked_amount/TOKEN_MULT, unlocked_amount%TOKEN_MULT);
        println!("     TOTAL: {:10}.{:09}", total_amount/TOKEN_MULT, total_amount%TOKEN_MULT);

        println!("  {:3} {:20} {:20} {:45} {:45} {}", "NUM", "AMOUNT", "LOCKUP", "ACCOUNT CreateWithSeed(creator, 'NEON_account_#')", "OWNER", "COMMENT");
        for (i, account) in self.token_distribution.iter().enumerate() {
            let seed: String = format!("{}_account_{}", REALM_NAME, i);
            let token_account = self.account_by_seed(&seed, &spl_token::id());
            let comment = match account.owner {
                AccountOwner::Key(_) => "Key".to_string(),
                _ => format!("{:?}", account.owner),
            };

            println!("  {:3} {:10}.{:09} {:20} {:45} {:45} {}", i,
                    account.amount/TOKEN_MULT, account.amount%TOKEN_MULT,
                    format!("{:?}", account.lockup),
                    format!("{:?}", token_account),
                    format!("{:?}", self.get_owner_address(&account.owner).unwrap()),
                    comment);
        }
        Ok(())
    }
}
