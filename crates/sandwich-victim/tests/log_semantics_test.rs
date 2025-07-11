use ethers::abi::{encode, AbiParser, EventExt, Token};
use ethers::types::{Address, Bytes, Log, H256};
use ethers::utils::keccak256;
use sandwich_victim::log_semantics::map_logs;

#[test]
fn decode_uniswap_v3_swap() {
    let mut parser = AbiParser::default();
    let event = parser.parse_event("event Swap(address indexed sender,address indexed recipient,int256 amount0,int256 amount1,uint160 sqrtPriceX96,uint128 liquidity,int24 tick,uint128 protocolFeesToken0,uint128 protocolFeesToken1)").unwrap();
    let topic0 = H256::from_slice(keccak256(event.abi_signature()).as_slice());
    let topics = vec![topic0, H256::from_low_u64_be(1), H256::from_low_u64_be(2)];
    let data = encode(&[
        Token::Int(1.into()),
        Token::Int(2.into()),
        Token::Uint(3u64.into()),
        Token::Uint(4u64.into()),
        Token::Int(5i32.into()),
        Token::Uint(6u64.into()),
        Token::Uint(7u64.into()),
    ]);
    let log = Log {
        address: Address::zero(),
        topics,
        data: Bytes::from(data),
        ..Default::default()
    };
    let mapped = map_logs(&[log]);
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0].event, "Swap");
    assert_eq!(mapped[0].params.len(), 9);
}
