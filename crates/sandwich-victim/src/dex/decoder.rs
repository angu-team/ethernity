use ethers::abi::{AbiParser, Function};
use serde::{Deserialize, Serialize};

/// Funções de swap suportadas em routers compatíveis com Uniswap V2
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapFunction {
    SwapExactTokensForTokens,
    SwapTokensForExactTokens,
    SwapExactETHForTokens,
    SwapTokensForExactETH,
    SwapExactTokensForETH,
    ETHForExactTokens,
    SwapExactTokensForTokensSupportingFeeOnTransferTokens,
    SwapExactETHForTokensSupportingFeeOnTransferTokens,
    SwapExactETHForTokensSupportingFeeOnTransferTokensWithReferrer,
    SwapExactTokensForETHSupportingFeeOnTransferTokens,
    ExactInputSingle,
    ExactInput,
    ExactOutputSingle,
    ExactOutput,
    SwapV2ExactIn,
    /// Any swap function of the 1inch Aggregation Router V6
    AggregationRouterV6Swap,
    /// `UniversalRouter.execute(bytes,bytes[])`
    UniversalRouterSwap,
    /// `UniversalRouter.execute(bytes,bytes[],uint256)`
    UniversalRouterSwapDeadline,
}

impl SwapFunction {
    fn signature(&self) -> &'static str {
        match self {
            SwapFunction::SwapExactTokensForTokens => {
                "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::SwapTokensForExactTokens => {
                "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactETHForTokens => {
                "swapExactETHForTokens(uint256,address[],address,uint256)"
            }
            SwapFunction::SwapTokensForExactETH => {
                "swapTokensForExactETH(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactTokensForETH => {
                "swapExactTokensForETH(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::ETHForExactTokens => {
                "swapETHForExactTokens(uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactTokensForTokensSupportingFeeOnTransferTokens => {
                "swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokens => {
                "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokensWithReferrer => {
                "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256,address)"
            }
            SwapFunction::SwapExactTokensForETHSupportingFeeOnTransferTokens => {
                "swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::ExactInputSingle => {
                "exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))"
            }
            SwapFunction::ExactInput => {
                "exactInput((bytes,address,uint256,uint256,uint256))"
            }
            SwapFunction::ExactOutputSingle => {
                "exactOutputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))"
            }
            SwapFunction::ExactOutput => {
                "exactOutput((bytes,address,uint256,uint256,uint256))"
            }
            SwapFunction::SwapV2ExactIn => {
                "swapV2ExactIn(address,address,uint256,uint256,address)"
            }
            SwapFunction::AggregationRouterV6Swap => "aggregationRouterV6Swap()",
            SwapFunction::UniversalRouterSwap => "execute(bytes,bytes[])",
            SwapFunction::UniversalRouterSwapDeadline => "execute(bytes,bytes[],uint256)",
        }
    }
}

/// Identifica qual função de swap foi invocada
pub fn detect_swap_function(data: &[u8]) -> Option<(SwapFunction, Function)> {
    if data.len() < 4 {
        return None;
    }
    let selector = &data[..4];
    let mappings = [
        (SwapFunction::SwapExactTokensForTokens, "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::SwapTokensForExactTokens, "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactETHForTokens, "swapExactETHForTokens(uint256,address[],address,uint256)"),
        (SwapFunction::SwapTokensForExactETH, "swapTokensForExactETH(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactTokensForETH, "swapExactTokensForETH(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::ETHForExactTokens, "swapETHForExactTokens(uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactTokensForTokensSupportingFeeOnTransferTokens, "swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokens, "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokensWithReferrer, "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256,address)"),
        (SwapFunction::SwapExactTokensForETHSupportingFeeOnTransferTokens, "swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::ExactInputSingle, "exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))"),
        (SwapFunction::ExactInput, "exactInput((bytes,address,uint256,uint256,uint256))"),
        (SwapFunction::ExactOutputSingle, "exactOutputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))"),
        (SwapFunction::ExactOutput, "exactOutput((bytes,address,uint256,uint256,uint256))"),
        (SwapFunction::SwapV2ExactIn, "swapV2ExactIn(address,address,uint256,uint256,address)"),
        // Uniswap Universal Router execute functions
        (
            SwapFunction::UniversalRouterSwap,
            "execute(bytes,bytes[])",
        ),
        (
            SwapFunction::UniversalRouterSwapDeadline,
            "execute(bytes,bytes[],uint256)",
        ),
        // 1inch Aggregation Router V6
        (
            SwapFunction::AggregationRouterV6Swap,
            "swap(address,(address,address,address,address,uint256,uint256,uint256,uint256),bytes)",
        ),
        (
            SwapFunction::AggregationRouterV6Swap,
            "unoswap(address,uint256,uint256,bytes32[])",
        ),
        (
            SwapFunction::AggregationRouterV6Swap,
            "unoswapTo(address,address,uint256,uint256,bytes32[])",
        ),
        (
            SwapFunction::AggregationRouterV6Swap,
            "unoswapWithPermit(address,uint256,uint256,bytes32[],uint256,uint256,uint8,bytes32,bytes32)",
        ),
        (
            SwapFunction::AggregationRouterV6Swap,
            "unoswapToWithPermit(address,address,uint256,uint256,bytes32[],uint256,uint256,uint8,bytes32,bytes32)",
        ),
        (
            SwapFunction::AggregationRouterV6Swap,
            "uniswapV3Swap(uint256,uint256,uint256[])",
        ),
        (
            SwapFunction::AggregationRouterV6Swap,
            "uniswapV3SwapTo(address,uint256,uint256,uint256[])",
        ),
        (
            SwapFunction::AggregationRouterV6Swap,
            "uniswapV3SwapToWithPermit(address,uint256,uint256,uint256[],uint256,uint256,uint8,bytes32,bytes32)",
        ),
        (
            SwapFunction::AggregationRouterV6Swap,
            "clipperSwap(address,address,uint256,uint256,uint256,uint256)",
        ),
    ];
    for (func, sig) in mappings {
        if selector == &ethers::utils::id(sig)[..4] {
            let mut parser = AbiParser::default();
            let f = parser.parse_function(sig).expect("abi parse");
            return Some((func, f));
        }
    }
    if selector == &[0x07, 0xed, 0x23, 0x79] {
        let f = AbiParser::default()
            .parse_function("aggregationSwap(bytes)")
            .expect("abi parse");
        return Some((SwapFunction::AggregationRouterV6Swap, f));
    }
    None
}
