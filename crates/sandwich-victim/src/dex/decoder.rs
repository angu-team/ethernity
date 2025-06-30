use ethers::abi::{AbiParser, Function};
use once_cell::sync::Lazy;
use serde_json::from_str;
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
    SwapExactTokensForETHSupportingFeeOnTransferTokens,
    ExactInputSingle,
    ExactInput,
    ExactOutputSingle,
    ExactOutput,
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
        }
    }
}

/// Identifica qual função de swap foi invocada
pub fn detect_swap_function(data: &[u8]) -> Option<(SwapFunction, Function)> {
    if data.len() < 4 {
        return None;
    }
    let selector = &data[..4];
    for (kind, func) in V2_FUNCTIONS.iter().chain(V3_FUNCTIONS.iter()) {
        if selector == &func.short_signature() {
            return Some((kind.clone(), func.clone()));
        }
    }
    None
}

static V2_FUNCTIONS: Lazy<Vec<(SwapFunction, Function)>> = Lazy::new(|| {
    let mut parser = AbiParser::default();
    [
        SwapFunction::SwapExactTokensForTokens,
        SwapFunction::SwapTokensForExactTokens,
        SwapFunction::SwapExactETHForTokens,
        SwapFunction::SwapTokensForExactETH,
        SwapFunction::SwapExactTokensForETH,
        SwapFunction::ETHForExactTokens,
        SwapFunction::SwapExactTokensForTokensSupportingFeeOnTransferTokens,
        SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokens,
        SwapFunction::SwapExactTokensForETHSupportingFeeOnTransferTokens,
    ]
    .into_iter()
    .map(|f| {
        let func = parser.parse_function(f.signature()).expect("abi parse");
        (f, func)
    })
    .collect()
});

static V3_FUNCTIONS: Lazy<Vec<(SwapFunction, Function)>> = Lazy::new(|| {
    vec![
        (
            SwapFunction::ExactInputSingle,
            from_str(
                r#"{
                    "type":"function",
                    "name":"exactInputSingle",
                    "inputs":[{"name":"params","type":"tuple","components":[{"type":"address","name":"tokenIn"},{"type":"address","name":"tokenOut"},{"type":"uint24","name":"fee"},{"type":"address","name":"recipient"},{"type":"uint256","name":"amountIn"},{"type":"uint256","name":"amountOutMinimum"},{"type":"uint160","name":"sqrtPriceLimitX96"}]}],
                    "outputs":[{"type":"uint256","name":"amountOut"}],
                    "stateMutability":"payable"
                }"#,
            )
            .expect("abi"),
        ),
        (
            SwapFunction::ExactInput,
            from_str(
                r#"{
                    "type":"function",
                    "name":"exactInput",
                    "inputs":[{"name":"params","type":"tuple","components":[{"type":"bytes","name":"path"},{"type":"address","name":"recipient"},{"type":"uint256","name":"amountIn"},{"type":"uint256","name":"amountOutMinimum"}]}],
                    "outputs":[{"type":"uint256","name":"amountOut"}],
                    "stateMutability":"payable"
                }"#,
            )
            .expect("abi"),
        ),
        (
            SwapFunction::ExactOutputSingle,
            from_str(
                r#"{
                    "type":"function",
                    "name":"exactOutputSingle",
                    "inputs":[{"name":"params","type":"tuple","components":[{"type":"address","name":"tokenIn"},{"type":"address","name":"tokenOut"},{"type":"uint24","name":"fee"},{"type":"address","name":"recipient"},{"type":"uint256","name":"amountOut"},{"type":"uint256","name":"amountInMaximum"},{"type":"uint160","name":"sqrtPriceLimitX96"}]}],
                    "outputs":[{"type":"uint256","name":"amountIn"}],
                    "stateMutability":"payable"
                }"#,
            )
            .expect("abi"),
        ),
        (
            SwapFunction::ExactOutput,
            from_str(
                r#"{
                    "type":"function",
                    "name":"exactOutput",
                    "inputs":[{"name":"params","type":"tuple","components":[{"type":"bytes","name":"path"},{"type":"address","name":"recipient"},{"type":"uint256","name":"amountOut"},{"type":"uint256","name":"amountInMaximum"}]}],
                    "outputs":[{"type":"uint256","name":"amountIn"}],
                    "stateMutability":"payable"
                }"#,
            )
            .expect("abi"),
        ),
    ]
});
