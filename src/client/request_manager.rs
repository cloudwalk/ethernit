use std::collections::HashMap;
use std::time::Duration;

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
// use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
// use tokio::time::{timeout, Instant};
use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use crate::request::rpc_request::ToRpcRequest;
use crate::request::{RequestId, SubscriptionId};
use crate::response::rpc_response::{RpcResponse, RpcSyncResponse};
use crate::response::rpc_subscription_success_response::RpcSubscriptionResponse;
use crate::response::rpc_sync_success_response::RpcSyncSuccessResponse;

use super::errors::Error;

type BlockchainSender = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type BlockchainReceiver = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const RECONNECT_INTERVAL: Duration = Duration::from_millis(1000);
const PING_INTERVAL: Duration = Duration::from_millis(10000);

#[derive(Debug)]
pub struct BlockchainRequest {
    pub(crate) operation: Box<dyn ToRpcRequest>,
    pub(crate) tx_response: oneshot::Sender<RpcSyncResponse>,
    pub(crate) tx_subscription: Option<mpsc::UnboundedSender<RpcSubscriptionResponse>>,
}

pub(crate) struct BlockchainRequestManager {
    url: String,

    tx_blockchain: BlockchainSender,
    rx_blockchain: BlockchainReceiver,
    rx_request: UnboundedReceiver<BlockchainRequest>,

    pending_reqs: HashMap<RequestId, oneshot::Sender<RpcSyncResponse>>,
    pending_subs: HashMap<RequestId, mpsc::UnboundedSender<RpcSubscriptionResponse>>,
    active_subs: HashMap<SubscriptionId, mpsc::UnboundedSender<RpcSubscriptionResponse>>,
}

impl BlockchainRequestManager {
    // -------------------------------------------------------------------------
    // Factory
    // -------------------------------------------------------------------------
    #[tracing::instrument(skip_all, name = "blockchain::init_request_manager")]
    pub async fn new(
        url: &str,
        rx_request: UnboundedReceiver<BlockchainRequest>,
    ) -> Result<Self, Error> {
        let (tx_blockchain, rx_blockchain) = BlockchainRequestManager::connect(url).await?;

        Ok(Self {
            url: url.into(),
            tx_blockchain,
            rx_blockchain,
            rx_request,
            pending_reqs: HashMap::new(),
            pending_subs: HashMap::new(),
            active_subs: HashMap::new(),
        })
    }

    pub fn spawn(mut self) {
        tracing::info!("spawning blockchain request manager");
        tokio::spawn(async move {
            let next_ping = tokio::time::sleep_until(Instant::now() + PING_INTERVAL);
            tokio::pin!(next_ping);

            loop {
                tokio::select! {
                    _ = &mut next_ping => {
                        self.handle_client_ping().await;
                        next_ping.as_mut().reset(Instant::now() + PING_INTERVAL);
                    }
                    msg = self.rx_request.recv() => {
                        self.handle_blockchain_request(msg).await;
                    }
                    msg = self.rx_blockchain.next() => {
                        self.handle_blockchain_response(msg).await;
                    }
                }
            }
        });
    }

    // -------------------------------------------------------------------------
    // Connection / Reconnection
    // -------------------------------------------------------------------------
    async fn connect(url: &str) -> Result<(BlockchainSender, BlockchainReceiver), Error> {
        tracing::info!(url = %url, "blockchain connecting");
        let (socket, _) = connect_async(url)
            .await
            .map_err(|e| Error::Connection(e.to_string()))?;
        Ok(socket.split())
    }

    async fn reconnect(&mut self) {
        loop {
            match BlockchainRequestManager::connect(&self.url).await {
                // reconnected
                Ok((tx, rx)) => {
                    // update tx and rx
                    self.tx_blockchain = tx;
                    self.rx_blockchain = rx;

                    // remove subs so subscriber are notified to recreate them
                    self.pending_subs.clear();
                    self.active_subs.clear();

                    // success
                    tracing::info!("blockchain reconnected");
                    break;
                }
                // failed to reconnect
                // wait some time and keep trying
                Err(e) => {
                    tracing::error!(reason = ?e, "blockchain failed to reconnect");
                    tokio::time::sleep(RECONNECT_INTERVAL).await;
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Handlers
    // -------------------------------------------------------------------------
    async fn handle_client_ping(&mut self) {
        let _ = self.tx_blockchain.send(Message::Ping(vec![0])).await;
    }

    async fn handle_blockchain_request(&mut self, message: Option<BlockchainRequest>) {
        if let Some(req) = message {
            // convert
            let rpc = req.operation.to_rpc_request();

            // keep track of pending responses and pending subscriptions
            self.pending_reqs.insert(rpc.id.clone(), req.tx_response);
            if let Some(tx_subscription) = req.tx_subscription {
                self.pending_subs.insert(rpc.id.clone(), tx_subscription);
            }

            // send request to blockchain
            match serde_json::to_string(&rpc) {
                Ok(payload) => {
                    tracing::debug!(payload = %payload, "req manager -> blockchain");
                    let _ = self.tx_blockchain.send(Message::text(payload)).await;
                }
                Err(e) => {
                    tracing::error!(reason = ?e, "req manager -> blockchain")
                }
            }
        }
    }

    async fn handle_blockchain_response(
        &mut self,
        message: Option<Result<Message, TungsteniteError>>,
    ) {
        match message {
            // text message
            Some(Ok(Message::Text(content))) => {
                self.handle_text_message(content);
            }

            // ping pong messages
            Some(Ok(Message::Ping(payload))) => {
                let _ = self.tx_blockchain.send(Message::Pong(payload)).await;
            }
            Some(Ok(Message::Pong(_))) => {}

            // connection closed, must reconnect
            None => {
                tracing::error!("blockchain lost connection");
                self.reconnect().await;
            }

            // unhandled messages
            msg => {
                tracing::warn!(reason = %"unhandled message type", payload = ?msg, "blockchain -> req manager")
            }
        }
    }

    fn handle_text_message(&mut self, text_message_payload: String) {
        tracing::debug!(payload = ?text_message_payload, "blockchain -> req manager");

        // parse response
        let chain_resp = match RpcResponse::parse(text_message_payload) {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(reason = ?e, "blockchain -> req manager");
                return;
            }
        };

        // send response to client if it is awaiting
        match chain_resp {
            RpcResponse::Sync(resp) => {
                self.activate_subscription_if_necessary(&resp);
                self.send_callback_for_response(resp);
            }
            RpcResponse::Subscription(sub) => {
                self.send_callback_for_subscription(sub);
            }
        }
    }

    /// When a response is a success for a request that has a pending subscription,
    /// it tries to activate the subscriptions by moving the TX from pending_subscriptions
    /// to active_subscriptions.
    ///
    /// This way, the callbacks from the blockchain can find the TX and send the callbacks to any
    /// active listeners.
    fn activate_subscription_if_necessary(&mut self, resp: &RpcSyncResponse) {
        if let Some(pending_sub) = self.pending_subs.remove(resp.id()) {
            if let RpcSyncResponse::Success(RpcSyncSuccessResponse {
                result: Value::String(result_str),
                ..
            }) = resp
            {
                tracing::info!(id = %result_str, "enabling subscription");
                self.active_subs.insert(result_str.to_owned(), pending_sub);
            }
        }
    }

    /// Sends a message from the request manager to a channel awaiting a response.
    /// The channel is identified by the request id and it is stored in pending_requests.
    fn send_callback_for_response(&mut self, resp: RpcSyncResponse) {
        match self.pending_reqs.remove(resp.id()) {
            Some(tx) => {
                tracing::debug!(id = %resp.id(), payload = ?resp, "req manager -> client");
                let _ = tx.send(resp);
            }
            None => {
                tracing::warn!(id = %resp.id(), reason = %"response tx not found", payload = ?resp, "req manager -> client")
            }
        }
    }

    /// Sends a message from the request manager to a channel subscribed to events.
    /// The channel is identified by the subscription id and it is stored in active_subscriptions.
    fn send_callback_for_subscription(&mut self, sub: RpcSubscriptionResponse) {
        match self.active_subs.get_mut(sub.id()) {
            Some(tx) => {
                tracing::debug!(id = %sub.id(), payload = ?sub, "req manager -> client");
                let _ = tx.send(sub);
            }
            None => {
                tracing::warn!(id = %sub.id(), reason = %"subscription tx not found", payload = ?sub, "req manager -> client")
            }
        }
    }
}
