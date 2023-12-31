/*
 * calculator-backend
 *
 * Calculator backend API
 *
 * The version of the OpenAPI document: 0.1.0
 *
 * Generated by: https://openapi-generator.tech
 */

/// CalculatorState : CalculatorState for HTTP GET

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct CalculatorState {
    #[serde(rename = "state")]
    pub state: String,
}

impl CalculatorState {
    /// CalculatorState for HTTP GET
    pub fn new(state: String) -> CalculatorState {
        CalculatorState { state }
    }
}
