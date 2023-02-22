use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::client::errors::Error;
use crate::client::request_manager::BlockchainRequest;
use crate::operation::{SubscribeLogsEvent, SubscribeLogsRequest};
use crate::types::TransactionHash;
use crate::{
    request::rpc_request::ToRpcRequest,
    response::rpc_subscription_success_response::RpcSubscriptionResponse, types::ContractAddress,
};

use super::Subscription;

#[derive(Debug, Clone)]
pub struct TransactionConfirmation {
    pub hash: TransactionHash,
    pub log_index: u64,
}

/// Transactions listener
///
/// Built on top of `logs` subscription, is responsible for listening to blockchain transactions and enabling its confirmation.
/// The `topics` field/argument, along with the `contract`, determines which kind of transactions will be listened to.
/// This is later used at the `blockchain` client, by subscribing to this `tx_transaction` and checking for a specific [`TransactionHash`].
#[derive(Debug)]
pub struct TransactionsListener {
    contract: ContractAddress,
    topics: Vec<serde_json::Value>,
    tx_request: mpsc::UnboundedSender<BlockchainRequest>,
    tx_transaction: broadcast::Sender<TransactionConfirmation>,
    rx_subscription: Arc<Mutex<mpsc::UnboundedReceiver<RpcSubscriptionResponse>>>,
}

impl TransactionsListener {
    #[tracing::instrument(skip_all, name = "blockchain::init_transactions_listener")]
    pub async fn new(
        contract: &ContractAddress,
        tx_request: mpsc::UnboundedSender<BlockchainRequest>,
        tx_transaction: broadcast::Sender<TransactionConfirmation>,
        transactions_topics: Vec<serde_json::Value>,
    ) -> Result<Self, Error> {
        let resp = TransactionsListener::subscribe(
            &tx_request,
            TransactionsListener::build_operation(contract, transactions_topics.clone()),
        )
        .await?;

        Ok(Self {
            contract: *contract,
            tx_request,
            tx_transaction,
            rx_subscription: Arc::new(Mutex::new(resp.rx_subscription)),
            topics: transactions_topics,
        })
    }

    fn build_operation(
        contract: &ContractAddress,
        transactions_topics: Vec<serde_json::Value>,
    ) -> Box<dyn ToRpcRequest> {
        Box::new(SubscribeLogsRequest {
            contract: *contract,
            topics: transactions_topics,
        })
    }
}

#[async_trait]
impl Subscription for TransactionsListener {
    fn name(&self) -> &'static str {
        "transactions"
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
        TransactionsListener::build_operation(&self.contract, self.topics.clone())
    }

    /// Creates a new message receiver
    fn set_response_receiver(
        &mut self,
        rx_subscription: mpsc::UnboundedReceiver<RpcSubscriptionResponse>,
    ) {
        self.rx_subscription = Arc::new(Mutex::new(rx_subscription))
    }

    /// Handles message received from logs subscription, regarding blockchain transactions
    ///
    /// If the message received is succesfully parsed, it builds a [`TransactionConfirmation`] and sends
    /// it to the confirmation broadcast channel. Later, this confirmations are used to decide if a
    /// transaction was confirmed or not.
    async fn handle_message(&self, payload: RpcSubscriptionResponse) {
        tracing::debug!(payload = ?payload, "blockchain -> listener logs");

        match serde_json::from_value::<SubscribeLogsEvent>(payload.params.result) {
            Ok(logs) => {
                let confirmation = TransactionConfirmation {
                    hash: logs.transaction_hash,
                    log_index: logs.log_index,
                };
                let _ = self.tx_transaction.send(confirmation);
            }
            Err(e) => {
                let custom_error = format!(
                    "Received unexpected subscription message from blockchain | payload={e:?}"
                );
                tracing::error!(reason = ?custom_error, "blockchain -> listener logs")
            }
        }
    }
}
