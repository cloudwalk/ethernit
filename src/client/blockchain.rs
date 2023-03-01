use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tracing::{info_span, Instrument};

use crate::{
    operation::{
        BlockNumberRequest, BlockNumberResponse, BlockchainReceipt, BlockchainTransaction,
        ChainIdRequest, ChainIdResponse, EthCallRequest, NetListeningRequest, NetListeningResponse,
        NonceRequest, NonceResponse, ReceiptRequest, ReceiptResponse, SendTransactionRequest,
        SendTransactionResponse, TransactionRequest, TransactionResponse,
    },
    request::{rpc_request::ToRpcRequest, rpc_request_contract_call::RpcRequestContractCall},
    response::{
        rpc_response::RpcSyncResponse, rpc_subscription_success_response::RpcSubscriptionResponse,
        rpc_sync_success_response::FromRpcSyncSuccessResponse,
    },
    types::{
        BlockNumber, ChainId, ContractAddress, EthDecFormat, EthHexFormat, Nonce, SignedData,
        TransactionHash, WalletAddress,
    },
};

use super::{
    errors::Error,
    request_manager::{BlockchainRequest, BlockchainRequestManager},
    subscription::{
        new_heads::{BlockNumberInfo, NewHeadsListener},
        transactions::{TransactionConfirmation, TransactionsListener},
        Subscription,
    },
};

/// Blockchain client
///
/// Provides an interface with `eth` blockchain methods.
/// The connection is made via `websocket` and handled by the [`BlockchainRequestManager`].
/// Two subscriptions are spawned at the start: `transactions`, from `logs` along with `transactions_topics` and `block number`, from `newHeads`.
/// The `transactions` subscription is used to confirm them, while the `block number` one is to keep track of new blocks.
#[derive(Debug)]
pub struct Blockchain {
    tx_request: mpsc::UnboundedSender<BlockchainRequest>,
    tx_transaction: broadcast::Sender<TransactionConfirmation>,
    blockchain_info: Arc<RwLock<BlockNumberInfo>>,
}
impl Blockchain {
    #[tracing::instrument(
        skip_all,
        name = "blockchain::init",
        fields(
            url,
            contract = %contract.formatted_hex()
        )
    )]
    pub async fn new(
        url: &str,
        contract: ContractAddress,
        transactions_topics: Vec<serde_json::Value>,
    ) -> Result<Self, Error> {
        let placeholder_blockchain_info =
            Arc::new(RwLock::new(BlockNumberInfo::new_placeholder()?));

        // create channels
        let (tx_request, rx_request) = mpsc::unbounded_channel::<BlockchainRequest>();
        let (tx_transaction, _) = broadcast::channel::<TransactionConfirmation>(10000);

        // spawn listeners
        BlockchainRequestManager::new(url, rx_request)
            .await?
            .spawn();
        TransactionsListener::new(
            &contract,
            tx_request.clone(),
            tx_transaction.clone(),
            transactions_topics,
        )
        .await?
        .spawn()
        .await;
        NewHeadsListener::new(tx_request.clone(), placeholder_blockchain_info.clone())
            .await?
            .spawn()
            .await;

        let blockchain = Blockchain {
            tx_request,
            tx_transaction,
            blockchain_info: placeholder_blockchain_info,
        };

        let block_number = blockchain.get_next_block_number().await?;
        blockchain
            .blockchain_info
            .write()
            .await
            .update(block_number);

        Ok(blockchain)
    }

    /// Sends a request to the blockchain request manager using a oneshot channel
    /// and awaits a response before returning.
    async fn request<T>(&self, operation: Box<dyn ToRpcRequest>) -> Result<T, Error>
    where
        T: FromRpcSyncSuccessResponse,
    {
        tracing::debug!(payload = ?operation, "blockchain command");

        // create communication channels for request and subscription
        let (tx_response, rx_response) = oneshot::channel::<RpcSyncResponse>();
        let (tx_subscription, rx_subscription) =
            mpsc::unbounded_channel::<RpcSubscriptionResponse>();

        // create request
        let tx_subscription = if operation.is_subscription() {
            Some(tx_subscription)
        } else {
            None
        };
        let req = BlockchainRequest {
            operation,
            tx_response,
            tx_subscription,
        };

        // send request
        tracing::debug!(payload = ?req, "client -> manager");
        self.tx_request
            .send(req)
            .map_err(|_| Error::RequestManagerSend)?;

        // await response
        let response = rx_response
            .await
            .map_err(|_| Error::RequestManagerReceive)?;
        match response {
            // request -> response
            RpcSyncResponse::Success(rpc_response) => {
                let parsed = T::from_rpc_response(rpc_response, rx_subscription);
                match parsed {
                    Ok(ref r) => tracing::debug!(payload = ?r, "blockchain response"),
                    Err(ref e) => tracing::error!(reason = ?e, "blockchain response"),
                };
                Ok(parsed?)
            }
            // request -> error
            RpcSyncResponse::Error(rpc) => {
                let report = format!(
                    "Received error response from blockchain after sending a request | payload={rpc:?}"
                );
                tracing::warn!(reason = ?report, "blockchain response");
                Err(Error::RpcResponse(
                    crate::response::errors::Error::SyncResponse(rpc.error),
                ))
            }
        }
    }

    /// Gets the next block number
    ///
    /// The "next" block number is actually the last plus one.
    /// First, it checks if the cache from the `newHeads` subscription is valid. If so, it returns its block number.
    /// In case of invalidation, an `eth_blockNumber` request is sent.
    #[tracing::instrument(skip_all, name = "blockchain::get_next_block_number")]
    pub async fn get_next_block_number(&self) -> Result<BlockNumber, Error> {
        tracing::info!("retrieving block number");

        if self.blockchain_info.read().await.is_cache_valid() {
            return Ok(self.blockchain_info.read().await.block_number + 1);
        }
        let resp: BlockNumberResponse = self.request(Box::new(BlockNumberRequest)).await?;
        Ok(resp.block_number + 1)
    }

    /// Gets chain id
    ///
    /// Uses `eth_chainId` to retrieve the chain id
    #[tracing::instrument(skip_all, name = "blockchain::get_chain_id")]
    pub async fn get_chain_id(&self) -> Result<ChainId, Error> {
        tracing::info!("retrieving chain id");
        let resp: ChainIdResponse = self.request(Box::new(ChainIdRequest)).await?;
        Ok(resp.chain_id)
    }

    /// Gets nonce from an wallet
    ///
    /// Uses `eth_getTransactionCount` along with the supplied `wallet` and "pending" as parameters.
    #[tracing::instrument(skip_all, name = "blockchain::get_nonce", fields(wallet, nonce))]
    pub async fn get_nonce(&self, wallet: WalletAddress) -> Result<Nonce, Error> {
        tracing::Span::current().record("wallet", wallet.formatted_hex().as_str());
        tracing::info!(wallet = %wallet.formatted_hex(), "retrieving nonce");

        let resp: NonceResponse = self.request(Box::new(NonceRequest { wallet })).await?;

        let nonce = resp.nonce;
        tracing::Span::current().record("nonce", nonce.formatted_dec().as_str());
        Ok(nonce)
    }

    /// Sends a signed transaction to blockchain
    ///
    /// The supplied `signed_data` is sent to blockchain via `eth_sendRawTransaction`.
    /// Nonce from signing must be provided along with the wallet, in order to verify its integrity and avoid errors.
    #[tracing::instrument(skip_all, name = "blockchain::send_transaction")]
    pub async fn send_transaction(
        &self,
        wallet: WalletAddress,
        nonce: u128,
        signed_data: SignedData,
    ) -> Result<SendTransactionResponse, Error> {
        tracing::info!(
            wallet = %wallet.formatted_hex(),
            nonce = %nonce,
            payload = %signed_data,
            "sending transaction"
        );

        // await nonce to submit transaction to chain
        loop {
            let chain_nonce = self.get_nonce(wallet).await?;
            if chain_nonce.as_u128() >= nonce {
                tracing::info!(name = "nonce", transaction_nonce = %nonce, chain_nonce = %chain_nonce.as_u128(), "transaction nonce matches blockchain nonce, proceeding with submission");
                break;
            } else {
                tracing::info!(name = "nonce", transaction_nonce = %nonce, chain_nonce = %chain_nonce.as_u128(), "transaction nonce does not match blockchain nonce, awaiting next nonce");
                tokio::task::yield_now().await
            }
        }

        // submit transaction to blockchain
        let submit_response: SendTransactionResponse = self
            .request(Box::new(SendTransactionRequest { signed_data }))
            .instrument(info_span!("blockchain::submit_transaction"))
            .await?;
        Ok(submit_response)
    }

    /// Awaits for a transaction to be confirmed
    ///
    /// It searches for the provided [`TransactionHash`] in the transactions confirmations (see [`TransactionsListener`]).
    ///
    /// # Warn
    /// This function can loop indefinitely if for some reason the `transaction` couldn't be confirmed.
    #[tracing::instrument(skip_all, name = "blockchain::await_transaction_confirmation")]
    pub async fn await_transaction_confirmation(
        &self,
        hash: TransactionHash,
    ) -> eyre::Result<TransactionConfirmation> {
        tracing::info!(hash = %hash.formatted_hex(), "awaiting transaction");

        let mut rx_transaction = self.tx_transaction.subscribe();
        let log_index = loop {
            let await_response = rx_transaction.recv().await?;

            if hash == await_response.hash {
                break await_response.log_index;
            }
        };

        Ok(TransactionConfirmation { hash, log_index })
    }

    /// Sends a transaction and awaits it to be confirmed
    ///
    /// Calls [`Self::send_transaction()`] and then [`Self::await_transaction_confirmation()`].
    ///
    /// # Warn
    /// This function can loop indefinitely if for some reason the `transaction` couldn't be confirmed.
    #[tracing::instrument(skip_all, name = "blockchain::send_transaction_and_await_confirmation")]
    pub async fn send_transaction_and_await_confirmation(
        &self,
        wallet: WalletAddress,
        nonce: u128,
        signed_data: SignedData,
        transaction_hash: &TransactionHash,
    ) -> eyre::Result<TransactionConfirmation> {
        tracing::info!(
            wallet = %wallet.formatted_hex(),
            nonce = %nonce,
            payload = %signed_data,
            transaction_hash = %transaction_hash.formatted_hex(),
            "sending transaction"
        );

        let _ = self.send_transaction(wallet, nonce, signed_data).await?;
        self.await_transaction_confirmation(*transaction_hash).await
    }

    /// Retrieves a blockchain receipt for a transaction hash.
    #[tracing::instrument(skip_all, name = "blockchain::get_receipt")]
    pub async fn get_receipt(
        &self,
        hash: TransactionHash,
    ) -> Result<Option<BlockchainReceipt>, Error> {
        tracing::info!(hash = %hash.formatted_hex(), "getting receipt");

        let request = Box::new(ReceiptRequest { hash });
        let resp: ReceiptResponse = self.request(request).await?;
        Ok(resp.receipt)
    }

    /// Retrieves a blockchain transaction for a transaction hash.
    #[tracing::instrument(skip_all, name = "blockchain::get_transaction")]
    pub async fn get_transaction(
        &self,
        hash: TransactionHash,
    ) -> Result<Option<BlockchainTransaction>, Error> {
        tracing::info!(hash = %hash.formatted_hex(), "getting transaction");

        let request = Box::new(TransactionRequest { hash });
        let resp: TransactionResponse = self.request(request).await?;
        Ok(resp.transaction)
    }

    /// Performs an `eth_call`
    #[tracing::instrument(skip_all, name = "blockchain::eth_call", fields(id = id_prefix))]
    pub async fn eth_call<T>(
        &self,
        id_prefix: String,
        payload: RpcRequestContractCall,
    ) -> Result<T, Error>
    where
        T: FromRpcSyncSuccessResponse,
    {
        tracing::info!("performing eth_call");
        let response = self
            .request(Box::new(EthCallRequest { id_prefix, payload }))
            .await?;
        Ok(response)
    }

    #[tracing::instrument(skip_all, name = "blockchain::net_listening")]
    pub async fn net_listening(&self) -> Result<bool, Error> {
        let response: NetListeningResponse = self.request(Box::new(NetListeningRequest)).await?;
        Ok(response.is_listening)
    }
}
