/*
 * calculator-backend
 *
 * Calculator backend API
 *
 * The version of the OpenAPI document: 0.1.0
 *
 * Generated by: https://openapi-generator.tech
 */

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct LoginResult {
    #[serde(rename = "account")]
    pub account: Box<crate::models::AuthPair>,
    #[serde(
        rename = "calculator",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub calculator: Option<Option<Box<crate::models::AuthPair>>>,
}

impl LoginResult {
    pub fn new(account: crate::models::AuthPair) -> LoginResult {
        LoginResult {
            account: Box::new(account),
            calculator: None,
        }
    }
}
