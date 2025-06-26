use ethernity_detector_mev::{TxNatureTagger, TxAggregator, AnnotatedTx};
use ethernity_core::{traits::RpcProvider, error::Result, types::TransactionHash};
use ethereum_types::{Address, H256};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct CountingProvider {
    calls: Arc<Mutex<usize>>,
    code: Vec<u8>,
}

impl CountingProvider {
    fn new(code: Vec<u8>) -> Self { Self { calls: Arc::new(Mutex::new(0)), code } }
}

#[async_trait]
impl RpcProvider for CountingProvider {
    async fn get_transaction_trace(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_transaction_receipt(&self, _tx_hash: TransactionHash) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_code(&self, _address: Address) -> Result<Vec<u8>> {
        let mut c = self.calls.lock().unwrap();
        *c += 1;
        Ok(self.code.clone())
    }
    async fn call(&self, _to: Address, _data: Vec<u8>) -> Result<Vec<u8>> { Ok(vec![]) }
    async fn get_block_number(&self) -> Result<u64> { Ok(0) }
    async fn get_block_hash(&self, _block_number: u64) -> Result<H256> { Ok(H256::zero()) }
}

#[tokio::test]
async fn pancakeswap_exactinput_detection() {
    let provider = CountingProvider::new(vec![0x60, 0x00, 0x56]);
    let tagger = TxNatureTagger::new(provider);
    let to = Address::from_slice(&hex::decode("13f4ea83d0bd40e75c8222255bc855a974568dd4").unwrap());
    let mut aggr = TxAggregator::new();

    let front_data = hex::decode("b858183f00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000080000000000000000000000000b8c1155414756d5bcf92c488d1c34e26b0c0c13b00000000000000000000000000000000000000000000008600288d159613000000000000000000000000000000000000000000000001f6c08990e2fe0bbaa80000000000000000000000000000000000000000000000000000000000000002b8d0d000ee44948fc98c9b98a4fa4921476f08b0d0000643c8d20001fe883934a15c949a3355a65ca9844440000000000000000000000000000000000000000000").unwrap();
    let res_f = tagger.analyze(to, &front_data, H256::from_low_u64_be(1)).await.unwrap();
    let tx_f = AnnotatedTx {
        tx_hash: H256::from_low_u64_be(1),
        token_paths: res_f.token_paths,
        targets: res_f.targets,
        tags: res_f.tags,
        first_seen: 1,
        gas_price: 50.0,
        max_priority_fee_per_gas: Some(5.0),
        confidence: res_f.confidence,
    };
    aggr.add_tx(tx_f);

    let back_data = hex::decode("b858183f00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000080000000000000000000000000b8c1155414756d5bcf92c488d1c34e26b0c0c13b0000000000000000000000000000000000000000000022ed606ea8e1d0890000000000000000000000000000000000000000000000000078a60b99bdae03c5c0000000000000000000000000000000000000000000000000000000000000002b3c8d20001fe883934a15c949a3355a65ca9844440000648d0d000ee44948fc98c9b98a4fa4921476f08b0d000000000000000000000000000000000000000000").unwrap();
    let res_b = tagger.analyze(to, &back_data, H256::from_low_u64_be(2)).await.unwrap();
    let tx_b = AnnotatedTx {
        tx_hash: H256::from_low_u64_be(2),
        token_paths: res_b.token_paths,
        targets: res_b.targets,
        tags: res_b.tags,
        first_seen: 2,
        gas_price: 30.0,
        max_priority_fee_per_gas: Some(3.0),
        confidence: res_b.confidence,
    };
    let k1 = aggr.add_tx(tx_b.clone());

    assert!(k1.is_some());
    assert_eq!(aggr.groups().len(), 2); // distinct token paths produce separate groups
    for g in aggr.groups().values() {
        assert!(g.txs[0].tags.contains(&"swap-v3".to_string()));
    }
}
