use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::client::errors::Error;
use crate::client::request_manager::BlockchainRequest;
use crate::operation::{SubscribeNewHeadsEvent, SubscribeNewHeadsRequest};
use crate::types::BlockNumber;
use crate::{
    request::rpc_request::ToRpcRequest,
    response::rpc_subscription_success_response::RpcSubscriptionResponse,
};

use super::Subscription;

const BLOCK_NUMBER_INTERVAL: Duration = Duration::from_millis(10_000);

#[derive(Debug)]
pub struct BlockNumberInfo {
    pub block_number: BlockNumber,
    updated_at: Instant,
}

impl BlockNumberInfo {
    pub fn update(&mut self, block_number: BlockNumber) {
        if block_number > self.block_number {
            self.block_number = block_number;
            self.updated_at = Instant::now()
        }
    }

    pub fn is_cache_valid(&self) -> bool {
        self.updated_at + BLOCK_NUMBER_INTERVAL >= Instant::now()
    }

    pub fn new_placeholder() -> Result<Self, Error> {
        let updated_at = match Instant::now().checked_sub(BLOCK_NUMBER_INTERVAL) {
            Some(instant) => instant,
            None => return Err(Error::BlockNumber),
        };
        Ok(Self {
            block_number: 1,
            updated_at,
        })
    }
}

#[derive(Debug)]
pub struct NewHeadsListener {
    tx_request: mpsc::UnboundedSender<BlockchainRequest>,
    block_number_info: Arc<RwLock<BlockNumberInfo>>,
    rx_subscription: Arc<Mutex<mpsc::UnboundedReceiver<RpcSubscriptionResponse>>>,
}

impl NewHeadsListener {
    #[tracing::instrument(skip_all, name = "blockchain::init_new_heads_listener")]
    pub async fn new(
        tx_request: mpsc::UnboundedSender<BlockchainRequest>,
        block_number_info: Arc<RwLock<BlockNumberInfo>>,
    ) -> Result<Self, Error> {
        let resp =
            NewHeadsListener::subscribe(&tx_request, Box::new(SubscribeNewHeadsRequest)).await?;

        Ok(Self {
            tx_request,
            block_number_info,
            rx_subscription: Arc::new(Mutex::new(resp.rx_subscription)),
        })
    }
}

#[async_trait]
impl Subscription for NewHeadsListener {
    fn name(&self) -> &'static str {
        "newHeads"
    }
    fn get_response_receiver(
        &self,
    ) -> Arc<Mutex<mpsc::UnboundedReceiver<RpcSubscriptionResponse>>> {
        self.rx_subscription.clone()
    }

    fn get_request_sender(&self) -> &mpsc::UnboundedSender<BlockchainRequest> {
        &self.tx_request
    }

    fn get_subscribe_operation(&self) -> Box<dyn ToRpcRequest> {
        Box::new(SubscribeNewHeadsRequest)
    }

    /// Creates a new message receiver
    fn set_response_receiver(
        &mut self,
        rx_subscription: mpsc::UnboundedReceiver<RpcSubscriptionResponse>,
    ) {
        self.rx_subscription = Arc::new(Mutex::new(rx_subscription))
    }

    /// Handles message received from newHeads subscription
    ///
    /// If the message received is succesfully parsed, it updates the internal [`BlockNumberInfo`] with its
    /// `block number` and the current instant.
    async fn handle_message(&self, payload: RpcSubscriptionResponse) {
        tracing::debug!(payload = ?payload, "blockchain -> listener newHeads");

        match serde_json::from_value::<SubscribeNewHeadsEvent>(payload.params.result) {
            Ok(result) => self
                .block_number_info
                .write()
                .await
                .update(result.block_number),
            Err(e) => {
                let error_message = format!(
                    "Received unexpected subscription message from blockchain | payload={e:?}"
                );
                tracing::error!(reason = ?error_message, "blockchain -> listener newHeads")
            }
        }
    }
}
