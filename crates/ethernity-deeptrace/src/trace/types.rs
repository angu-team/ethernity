use serde::Deserialize;

/// Estrutura de trace de chamada
#[derive(Debug, Clone, Deserialize)]
pub struct CallTrace {
    pub from: String,
    pub gas: String,
    #[serde(rename = "gasUsed")]
    pub gas_used: String,
    pub to: String,
    pub input: String,
    pub output: String,
    pub value: String,
    pub error: Option<String>,
    pub calls: Option<Vec<CallTrace>>,
    #[serde(rename = "type")]
    pub call_type: Option<String>,
}

/// Tipo de chamada
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallType {
    Call,
    StaticCall,
    DelegateCall,
    CallCode,
    Create,
    Create2,
    SelfDestruct,
    Unknown,
}

impl From<&str> for CallType {
    fn from(s: &str) -> Self {
        match s {
            "CALL" => CallType::Call,
            "STATICCALL" => CallType::StaticCall,
            "DELEGATECALL" => CallType::DelegateCall,
            "CALLCODE" => CallType::CallCode,
            "CREATE" => CallType::Create,
            "CREATE2" => CallType::Create2,
            "SELFDESTRUCT" => CallType::SelfDestruct,
            _ => CallType::Unknown,
        }
    }
}
