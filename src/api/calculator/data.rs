use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Calculator's database data
#[derive(Debug, Clone)]
pub struct CalculatorStateInternal {
    pub state: String,
}

/// CalculatorState for HTTP GET
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct CalculatorState {
    pub state: String,
}

impl CalculatorState {
    pub fn into_update(self) -> CalculatorState {
        CalculatorState { state: self.state }
    }
}

impl From<CalculatorStateInternal> for CalculatorState {
    fn from(value: CalculatorStateInternal) -> Self {
        Self { state: value.state }
    }
}
