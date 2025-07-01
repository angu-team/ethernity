pub mod uniswap_v2;
pub mod uniswap_v3;
pub mod smart_router;

/// Agrupamento semântico das implementações de detectores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cluster {
    UniswapV2,
    UniswapV3,
    SmartRouter,
    Unknown,
}
use crate::dex::SwapFunction;

impl From<&SwapFunction> for Cluster {
    fn from(func: &SwapFunction) -> Self {
        match func {
            SwapFunction::SwapExactTokensForTokens
            | SwapFunction::SwapTokensForExactTokens
            | SwapFunction::SwapExactETHForTokens
            | SwapFunction::SwapTokensForExactETH
            | SwapFunction::SwapExactTokensForETH
            | SwapFunction::ETHForExactTokens
            | SwapFunction::SwapExactTokensForTokensSupportingFeeOnTransferTokens
            | SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokens
            | SwapFunction::SwapExactTokensForETHSupportingFeeOnTransferTokens
            | SwapFunction::SwapV2ExactIn => Cluster::UniswapV2,
            SwapFunction::ExactInputSingle
            | SwapFunction::ExactInput
            | SwapFunction::ExactOutputSingle
            | SwapFunction::ExactOutput => Cluster::UniswapV3,
            _ => Cluster::Unknown,
        }
    }
}
