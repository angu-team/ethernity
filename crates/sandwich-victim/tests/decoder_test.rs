use sandwich_victim::dex::{detect_swap_function, SwapFunction};
use hex::decode;

#[test]
fn detect_swap_v2_exact_in_function() {
    let data_hex = "5b9e900600000000000000000000000000000000000000000000000000000000000000000000000000000000000000006ef0c49650090d47f61cc934ba3774784fee4444000000000000000000000000000000000000000000000000016345785d8a0000000000000000000000000000000000000000000b72b653b4048f7774171500000000000000000000000037b35d3d8e2208ac0ec533f4f98508d0e1620f9b";
    let data = decode(data_hex).unwrap();
    let (func, _) = detect_swap_function(&data).expect("failed to detect");
    assert_eq!(func, SwapFunction::SwapV2ExactIn);
}
