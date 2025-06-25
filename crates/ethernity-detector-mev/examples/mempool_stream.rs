use std::env;
use std::sync::Arc;
use std::time::Duration;

use ethernity_detector_mev::{
    MempoolSupervisor, SupervisorEvent, BlockMetadata, TxNatureTagger, RawTx,
};
use ethernity_core::{traits::RpcProvider, error::Result};
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use ethereum_types::{Address, H256};
use tokio::sync::mpsc;
use futures::StreamExt;
use web3::transports::WebSocket;
use web3::types::TransactionId;

#[derive(Clone)]
struct SharedRpc(Arc<EthernityRpcClient>);

#[async_trait::async_trait]
impl RpcProvider for SharedRpc {
    async fn get_transaction_trace(&self, tx_hash: H256) -> Result<Vec<u8>> {
        self.0.get_transaction_trace(tx_hash).await
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> Result<Vec<u8>> {
        self.0.get_transaction_receipt(tx_hash).await
    }

    async fn get_code(&self, address: Address) -> Result<Vec<u8>> {
        self.0.get_code(address).await
    }

    async fn call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>> {
        self.0.call(to, data).await
    }

    async fn get_block_number(&self) -> Result<u64> {
        self.0.get_block_number().await
    }

    async fn get_block_hash(&self, block_number: u64) -> Result<H256> {
        self.0.get_block_hash(block_number).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <WS_RPC_ENDPOINT>", args[0]);
        eprintln!("Exemplo: {} wss://mainnet.infura.io/ws/v3/YOURKEY", args[0]);
        std::process::exit(1);
    }
    let endpoint = args[1].clone();

    let rpc_config = RpcConfig { endpoint: endpoint.clone(), ..Default::default() };
    let rpc_client = Arc::new(EthernityRpcClient::new(rpc_config).await?);
    let provider = SharedRpc(rpc_client.clone());

    let ws = WebSocket::new(&endpoint).await?;
    let web3 = web3::Web3::new(ws);
    let mut sub_tx = web3.eth_subscribe().subscribe_new_pending_transactions().await?;
    let mut sub_heads = web3.eth_subscribe().subscribe_new_heads().await?;

    let (tx_raw, rx_raw) = mpsc::channel(1024);
    let (tx_annotated, mut rx_annotated) = mpsc::channel(1024);
    let (tx_sup_in, rx_sup_in) = mpsc::channel(1024);
    let (tx_groups, mut rx_groups) = mpsc::channel(1024);

    let tagger = TxNatureTagger::new(provider.clone());
    let sup = MempoolSupervisor::new(provider.clone(), 1, Duration::from_secs(1), 10);

    tokio::spawn(async move { tagger.process_stream(rx_raw, tx_annotated).await; });
    tokio::spawn(async move { sup.process_stream(rx_sup_in, tx_groups).await; });

    let tx_sup = tx_sup_in.clone();
    tokio::spawn(async move {
        while let Some(ann) = rx_annotated.recv().await {
            let _ = tx_sup.send(SupervisorEvent::NewTxObserved(ann)).await;
        }
    });

    tokio::spawn(async move {
        while let Some(group) = rx_groups.recv().await {
            println!("\nNovo grupo formado: {} transações", group.group.txs.len());
            println!("Score de alinhamento de estado: {:.2}", group.metadata.state_alignment_score);
        }
    });

    loop {
        tokio::select! {
            Some(Ok(hash)) = sub_tx.next() => {
                if let Ok(Some(tx)) = web3.eth().transaction(TransactionId::Hash(hash)).await {
                    if let (Some(to), input) = (tx.to, tx.input.0.clone()) {
                        let raw = RawTx {
                            tx_hash: H256::from_slice(hash.as_bytes()),
                            to: Address::from_slice(to.as_bytes()),
                            input,
                            first_seen: chrono::Utc::now().timestamp() as u64,
                            gas_price: tx.gas_price.map(|g| g.as_u128() as f64).unwrap_or_default(),
                            max_priority_fee_per_gas: tx.max_priority_fee_per_gas.map(|v| v.as_u128() as f64),
                        };
                        let _ = tx_raw.send(raw).await;
                    }
                }
            }
            Some(Ok(head)) = sub_heads.next() => {
                if let Some(num) = head.number {
                    let _ = tx_sup_in.send(SupervisorEvent::BlockAdvanced(BlockMetadata { number: num.as_u64() })).await;
                }
            }
        }
    }
}
