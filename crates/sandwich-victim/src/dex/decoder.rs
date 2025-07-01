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
    SwapExactTokensForETHSupportingFeeOnTransferTokens,
    ExactInputSingle,
    ExactInput,
    ExactOutputSingle,
    ExactOutput,
    SwapV2ExactIn,
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
            SwapFunction::SwapV2ExactIn => {
                "swapV2ExactIn(address,address,uint256,uint256,address)"
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
    let mappings = [
        (SwapFunction::SwapExactTokensForTokens, "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::SwapTokensForExactTokens, "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactETHForTokens, "swapExactETHForTokens(uint256,address[],address,uint256)"),
        (SwapFunction::SwapTokensForExactETH, "swapTokensForExactETH(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactTokensForETH, "swapExactTokensForETH(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::ETHForExactTokens, "swapETHForExactTokens(uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactTokensForTokensSupportingFeeOnTransferTokens, "swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokens, "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)"),
        (SwapFunction::SwapExactTokensForETHSupportingFeeOnTransferTokens, "swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"),
        (SwapFunction::ExactInputSingle, "exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))"),
        (SwapFunction::ExactInput, "exactInput((bytes,address,uint256,uint256,uint256))"),
        (SwapFunction::ExactOutputSingle, "exactOutputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))"),
        (SwapFunction::ExactOutput, "exactOutput((bytes,address,uint256,uint256,uint256))"),
        (SwapFunction::SwapV2ExactIn, "swapV2ExactIn(address,address,uint256,uint256,address)"),
    ];
    for (func, sig) in mappings {
        if selector == &ethers::utils::id(sig)[..4] {
            let mut parser = AbiParser::default();
            let f = parser.parse_function(sig).expect("abi parse");
            return Some((func, f));
        }
    }

    const ALT_FOT_SELECTOR: [u8; 4] = [0x35, 0xd2, 0x94, 0x75];
    if selector == ALT_FOT_SELECTOR {
        let mut parser = AbiParser::default();
        let sig = "swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)";
        let f = parser.parse_function(sig).expect("abi parse");
        return Some((
            SwapFunction::SwapExactTokensForTokensSupportingFeeOnTransferTokens,
            f,
        ));
    }
    None
}
