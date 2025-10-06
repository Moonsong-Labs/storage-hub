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

#[derive(Debug, Serialize)]
pub struct PaymentStreamInfo {
    pub provider: String,
    #[serde(rename = "providerType")]
    pub provider_type: String,
    #[serde(rename = "totalAmountPaid")]
    pub total_amount_paid: String,
    #[serde(rename = "costPerTick")]
    pub cost_per_tick: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentStreamsResponse {
    pub streams: Vec<PaymentStreamInfo>,
}
