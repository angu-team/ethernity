use ethernity_detector_mev::{TxNatureTagger};
use ethernity_core::{traits::RpcProvider, error::Result, types::TransactionHash};
use ethereum_types::{Address, H256};
use async_trait::async_trait;

struct DummyProvider;

#[async_trait]
impl RpcProvider for DummyProvider {
    async fn get_transaction_trace(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    async fn get_transaction_receipt(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    async fn get_code(&self, _address: Address) -> Result<Vec<u8>> {
        // bytecode simples sem delegatecall
        Ok(vec![0x60, 0x00, 0x60, 0x00, 0x56])
    }

    async fn call(&self, _to: Address, _data: Vec<u8>) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    async fn get_block_number(&self) -> Result<u64> {
        Ok(0)
    }
}

#[tokio::test]
async fn detect_swap_v2() {
    let provider = DummyProvider;
    let tagger = TxNatureTagger::new(provider);
    // calldata para swapExactTokensForTokens (selector 0x38ed1739)
    let data = hex::decode("38ed173900000000000000000000000000000000000000000000000000000000000001").unwrap();
    let to = Address::repeat_byte(0x11);
    let tx_hash = H256::repeat_byte(0x22);

    let res = tagger.analyze(to, &data, tx_hash).await.unwrap();
    assert!(res.tags.contains(&"swap-v2".to_string()));
}
