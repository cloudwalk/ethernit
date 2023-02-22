use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct RpcSubscriptionResponse {
    pub params: RpcSubscriptionResponseParams,
}

impl RpcSubscriptionResponse {
    pub fn id(&self) -> &str {
        &self.params.subscription
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcSubscriptionResponseParams {
    pub subscription: String,
    pub result: serde_json::Value,
}
