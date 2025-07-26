use steamguard::ConfirmationType;

pub fn conf_to_u32(t: &ConfirmationType) -> u32 {
    match t {
        ConfirmationType::Test => 1,
        ConfirmationType::Trade => 2,
        ConfirmationType::MarketSell => 3,
        ConfirmationType::FeatureOptOut => 4,
        ConfirmationType::PhoneNumberChange => 5,
        ConfirmationType::AccountRecovery => 6,
        ConfirmationType::ApiKeyCreation => 9,
        ConfirmationType::JoinSteamFamily => 11,
        ConfirmationType::Unknown(val) => *val, // Return the associated u32 value
    }
}
