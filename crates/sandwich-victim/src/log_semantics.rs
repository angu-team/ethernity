use ethers::abi::{AbiParser, Event, EventExt, RawLog, Token};
use ethers::types::{Address, Log, H256};
use ethers::utils::keccak256;
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MappedLog {
    pub address: Address,
    pub event: String,
    pub params: Vec<(String, Token)>,
}

fn build_event_map() -> HashMap<H256, Event> {
    let mut map = HashMap::new();
    let mut parser = AbiParser::default();
    // Uniswap V3 Swap event
    if let Ok(ev) = parser.parse_event("event Swap(address indexed sender,address indexed recipient,int256 amount0,int256 amount1,uint160 sqrtPriceX96,uint128 liquidity,int24 tick,uint128 protocolFeesToken0,uint128 protocolFeesToken1)") {
        let topic = H256::from_slice(keccak256(ev.abi_signature()).as_slice());
        map.insert(topic, ev);
    }
    // Uniswap V2 Swap event
    if let Ok(ev) = parser.parse_event("event Swap(address indexed sender,uint256 amount0In,uint256 amount1In,uint256 amount0Out,uint256 amount1Out,address indexed to)") {
        let topic = H256::from_slice(keccak256(ev.abi_signature()).as_slice());
        map.insert(topic, ev);
    }
    map
}

static EVENT_MAP: Lazy<HashMap<H256, Event>> = Lazy::new(build_event_map);

/// Decodes logs using known event signatures
pub fn map_logs(logs: &[Log]) -> Vec<MappedLog> {
    logs.iter()
        .filter_map(|log| {
            let topic0 = log.topics.get(0)?;
            let event = EVENT_MAP.get(topic0)?;
            let raw = RawLog {
                topics: log.topics.clone(),
                data: log.data.to_vec(),
            };
            match event.parse_log(raw) {
                Ok(decoded) => Some(MappedLog {
                    address: log.address,
                    event: event.name.clone(),
                    params: decoded
                        .params
                        .into_iter()
                        .map(|p| (p.name, p.value))
                        .collect(),
                }),
                Err(_) => None,
            }
        })
        .collect()
}
