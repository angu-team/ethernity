use crate::trace::{CallTrace, CallType};
use crate::utils;
use crate::ExecutionStep;
use crate::TraceAnalysisConfig;
use ethereum_types::{Address, U256};

pub fn build_execution_path(trace: &CallTrace, config: &TraceAnalysisConfig) -> Result<Vec<ExecutionStep>, ()> {
    let mut path = Vec::new();
    build_execution_path_recursive(trace, 0, &mut path, config)?;
    Ok(path)
}

fn build_execution_path_recursive(trace: &CallTrace, depth: usize, path: &mut Vec<ExecutionStep>, config: &TraceAnalysisConfig) -> Result<(), ()> {
    if depth > config.max_depth { return Ok(()); }
    let step = ExecutionStep {
        depth,
        call_type: trace.call_type.as_deref().map(CallType::from).unwrap_or(CallType::Call),
        from: utils::parse_address(&trace.from),
        to: if trace.to.is_empty() { Address::zero() } else { utils::parse_address(&trace.to) },
        value: U256::from_dec_str(&trace.value).unwrap_or(U256::zero()),
        input: utils::decode_hex(&trace.input),
        output: utils::decode_hex(&trace.output),
        gas_used: U256::from_dec_str(&trace.gas_used).unwrap_or(U256::zero()),
        error: trace.error.clone(),
    };
    path.push(step);
    if let Some(calls) = &trace.calls {
        for child_call in calls {
            build_execution_path_recursive(child_call, depth + 1, path, config)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trace() -> CallTrace {
        CallTrace {
            from: "0x0000000000000000000000000000000000000001".into(), gas: "0".into(), gas_used: "0".into(),
            to: "0x0000000000000000000000000000000000000002".into(), input: "0x".into(), output: "0x".into(), value: "0".into(), error: None,
            calls: Some(vec![CallTrace {
                from: "0x0000000000000000000000000000000000000002".into(), gas: "0".into(), gas_used: "0".into(),
                to: "".into(), input: "0x".into(), output: "0x".into(), value: "0".into(), error: None,
                calls: None, call_type: Some("CALL".into())
            }]),
            call_type: Some("CALL".into()),
        }
    }

    #[test]
    fn test_build_execution_path_depth() {
        let trace = sample_trace();
        let mut cfg = TraceAnalysisConfig::default();
        cfg.max_depth = 0;
        let steps = build_execution_path(&trace, &cfg).unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].to, Address::from_low_u64_be(2));
    }

    #[test]
    fn test_build_execution_path_full() {
        let trace = sample_trace();
        let steps = build_execution_path(&trace, &TraceAnalysisConfig::default()).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[1].to, Address::zero());
    }
}
