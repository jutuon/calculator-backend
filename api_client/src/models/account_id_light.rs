/*
 * calculator-backend
 *
 * Calculator backend API
 *
 * The version of the OpenAPI document: 0.1.0
 *
 * Generated by: https://openapi-generator.tech
 */

/// AccountIdLight : AccountId which is internally Uuid object. Consumes less memory.

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct AccountIdLight {
    #[serde(rename = "account_id")]
    pub account_id: uuid::Uuid,
}

impl AccountIdLight {
    /// AccountId which is internally Uuid object. Consumes less memory.
    pub fn new(account_id: uuid::Uuid) -> AccountIdLight {
        AccountIdLight { account_id }
    }
}
