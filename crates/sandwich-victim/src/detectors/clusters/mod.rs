pub mod oneinch_aggregation_router_v6;
pub mod oneinch_generic_router;
pub mod smart_router;
pub mod uniswap_universal_router;
pub mod uniswap_v2;
pub mod uniswap_v3;
pub mod uniswap_v4;

/// Agrupamento semântico das implementações de detectores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cluster {
    UniswapV2,
    UniswapV3,
    UniswapV4,
    SmartRouter,
    UniswapUniversalRouter,
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
            | SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokensWithReferrer
            | SwapFunction::SwapExactTokensForETHSupportingFeeOnTransferTokens
            | SwapFunction::SwapV2ExactIn => Cluster::UniswapV2,
            SwapFunction::ExactInputSingle
            | SwapFunction::ExactInput
            | SwapFunction::ExactOutputSingle
            | SwapFunction::ExactOutput
            | SwapFunction::SwapV3ExactIn => Cluster::UniswapV3,
            SwapFunction::UniversalRouterSwap | SwapFunction::UniversalRouterSwapDeadline => {
                Cluster::UniswapUniversalRouter
            }
            SwapFunction::AggregationRouterV6Swap => Cluster::Unknown,
        }
    }
}
