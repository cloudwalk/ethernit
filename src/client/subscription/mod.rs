pub mod new_heads;
pub mod transactions;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::timeout;

use crate::response::rpc_sync_success_response::FromRpcSyncSuccessResponse;
use crate::{
    operation::SubscribeResponse,
    request::rpc_request::ToRpcRequest,
    response::{
        rpc_response::RpcSyncResponse, rpc_subscription_success_response::RpcSubscriptionResponse,
    },
};

use super::{errors::Error, request_manager::BlockchainRequest};

const RECONNECT_INTERVAL: Duration = Duration::from_millis(1000);

/// Trait to handle blockchain subscriptions
///
/// The core functionality, regarding the whole messaging passing, are already implemented.
/// It relies into [`mpsc`] channels, sending [`BlockchainRequest`]s and receiving [`RpcSubscriptionResponse`]s.
/// Also, it already covers the resubscription, in case of it was dropped somehow.
#[async_trait]
pub trait Subscription: Send + Sync + Sized + 'static {
    /// Event name
    fn name(&self) -> &'static str;
    /// Blockchain subscription operation
    ///
    /// It's necessary to default [`Subscription::resubscribe()`] impl
    fn get_subscribe_operation(&self) -> Box<dyn ToRpcRequest>;
    /// Messages response receiver
    ///
    /// It's necessary to default [`Subscription::spawn()`] impl
    fn get_response_receiver(&self)
        -> Arc<Mutex<mpsc::UnboundedReceiver<RpcSubscriptionResponse>>>;
    /// Blockchain requests sender
    ///
    /// It's necessary to default [`Subscription::resubscribe()`] impl
    fn get_request_sender(&self) -> &mpsc::UnboundedSender<BlockchainRequest>;
    /// Set messages response receiver
    ///
    /// It's necessary to default [`Subscription::resubscribe()`] impl
    fn set_response_receiver(
        &mut self,
        rx_subscription: mpsc::UnboundedReceiver<RpcSubscriptionResponse>,
    );
    /// Spawns subscription listener
    ///
    /// The default impl spawns an asynchronous task that loops forever and do not return.
    /// It gets and locks the `response_receiver`, awaiting for messages.
    /// When a message is received, it's dispatched to [`Subscription::handle_message()`].
    /// In case of error, [`Subscription::resubscribe()`] is called.
    async fn spawn(mut self) {
        tracing::info!("spawning blockchain {} listener", self.name());
        tokio::spawn(async move {
            loop {
                let rx_subscription = self.get_response_receiver();
                let mut rx_subscription = rx_subscription.lock().await;
                match rx_subscription.recv().await {
                    Some(sub) => {
                        let sub = sub.clone();
                        self.handle_message(sub).await;
                    }
                    None => {
                        tracing::warn!("{} event subscription dropped", self.name());
                        self.resubscribe().await;
                    }
                }
            }
        });
    }
    /// Subscribes to a blockchain event
    ///
    /// Sends a blockchain request in order to subscribe to some event.
    /// In case of success, returns an object containing the blockchain message receiver.
    async fn subscribe(
        tx_request: &mpsc::UnboundedSender<BlockchainRequest>,
        operation: Box<dyn ToRpcRequest>,
    ) -> Result<SubscribeResponse, Error> {
        tracing::info!("subscribing to blockchain event");

        // send request to subscribe
        let (tx_response, rx_response) = oneshot::channel::<RpcSyncResponse>();
        let (tx_subscription, rx_subscription) = mpsc::unbounded_channel();
        let req = BlockchainRequest {
            operation,
            tx_response,
            tx_subscription: Some(tx_subscription),
        };
        tx_request
            .send(req)
            .map_err(|_| Error::RequestManagerSend)?;

        // await response of subscription
        match timeout(Duration::from_secs(5), rx_response)
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(|_| Error::RequestManagerReceive)?
        {
            RpcSyncResponse::Success(rpc) => {
                let parsed = SubscribeResponse::from_rpc_response(rpc, rx_subscription)?;
                Ok(parsed)
            }
            RpcSyncResponse::Error(rpc) => {
                tracing::error!(payload = ?rpc, "blockchain response");
                Err(Error::Subscription(format!("{rpc:?}")))
            }
        }
    }
    /// Resubscribe to blockchain event
    ///
    /// Called in case of failure from subscription handler, it tries indefinitely to override the response receiver.
    async fn resubscribe(&mut self) {
        loop {
            match Self::subscribe(self.get_request_sender(), self.get_subscribe_operation()).await {
                Ok(resp) => {
                    self.set_response_receiver(resp.rx_subscription);
                    tracing::info!("resubscribed to {} event", self.name());
                    break;
                }
                // failed to resubscribe
                Err(e) => {
                    tracing::error!(reason = ?e, "failed to resubscribe to {} event", self.name());
                    tokio::time::sleep(RECONNECT_INTERVAL).await;
                }
            }
        }
    }
    /// Handles message received from blockchain event subscription
    async fn handle_message(&self, payload: RpcSubscriptionResponse);
}
