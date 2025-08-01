use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PaymentStream {
    #[serde(rename = "tokensPerBlock")]
    pub tokens_per_block: u64,
    #[serde(rename = "lastChargedTick")]
    pub last_charged_tick: u64,
    #[serde(rename = "userDeposit")]
    pub user_deposit: u64,
    #[serde(rename = "outOfFundsTick")]
    pub out_of_funds_tick: Option<u64>,
}