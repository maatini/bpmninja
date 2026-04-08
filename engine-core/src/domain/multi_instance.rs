use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiInstanceDef {
    pub is_sequential: bool,
    pub loop_cardinality: Option<String>,
    pub collection: Option<String>,
    pub element_variable: Option<String>,
}

// ---------------------------------------------------------------------------

