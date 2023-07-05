use base64::Engine;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Used with database
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, Eq, Hash, PartialEq, Copy)]
pub struct AccountIdInternal {
    pub account_id: uuid::Uuid,
    pub account_row_id: i64,
}

impl AccountIdInternal {
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.account_id
    }

    pub fn row_id(&self) -> i64 {
        self.account_row_id
    }

    pub fn as_light(&self) -> AccountIdLight {
        AccountIdLight {
            account_id: self.account_id,
        }
    }
}

impl From<AccountIdInternal> for uuid::Uuid {
    fn from(value: AccountIdInternal) -> Self {
        value.account_id
    }
}

/// AccountId which is internally Uuid object.
/// Consumes less memory.
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, Eq, Hash, PartialEq, IntoParams, Copy)]
pub struct AccountIdLight {
    pub account_id: uuid::Uuid,
}

impl std::fmt::Display for AccountIdLight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Test"))
    }
}

impl AccountIdLight {
    pub fn new(account_id: uuid::Uuid) -> Self {
        Self { account_id }
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.account_id
    }

    pub fn to_string(&self) -> String {
        self.account_id.hyphenated().to_string()
    }
}

impl From<AccountIdLight> for uuid::Uuid {
    fn from(value: AccountIdLight) -> Self {
        value.account_id
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone, Eq, Hash, PartialEq)]
pub struct LoginResult {
    pub account: AuthPair,

    /// If None calculator microservice is disabled.
    pub calculator: Option<AuthPair>,
}

/// This is just a random string.
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone, Eq, Hash, PartialEq)]
pub struct ApiKey {
    /// API token which server generates.
    api_key: String,
}

impl ApiKey {
    pub fn generate_new() -> Self {
        Self {
            api_key: uuid::Uuid::new_v4().simple().to_string(),
        }
    }

    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn into_string(self) -> String {
        self.api_key
    }

    pub fn as_str(&self) -> &str {
        &self.api_key
    }
}

/// This is just a really long random number which is Base64 encoded.
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone, Eq, Hash, PartialEq)]
pub struct RefreshToken {
    token: String,
}

impl RefreshToken {
    pub fn generate_new_with_bytes() -> (Self, Vec<u8>) {
        let mut token = Vec::new();

        // TODO: use longer refresh token
        for _ in 1..=2 {
            token.extend(uuid::Uuid::new_v4().to_bytes_le())
        }

        (Self::from_bytes(&token), token)
    }

    pub fn generate_new() -> Self {
        let (token, _bytes) = Self::generate_new_with_bytes();
        token
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            token: base64::engine::general_purpose::STANDARD.encode(data),
        }
    }

    /// String must be base64 encoded
    /// TODO: add checks?
    pub fn from_string(token: String) -> Self {
        Self { token }
    }

    /// Base64 string
    pub fn into_string(self) -> String {
        self.token
    }

    /// Base64 string
    pub fn as_str(&self) -> &str {
        &self.token
    }

    pub fn bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
        base64::engine::general_purpose::STANDARD.decode(&self.token)
    }
}

/// AccessToken and RefreshToken
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone, Eq, Hash, PartialEq)]
pub struct AuthPair {
    pub refresh: RefreshToken,
    pub access: ApiKey,
}

impl AuthPair {
    pub fn new(refresh: RefreshToken, access: ApiKey) -> Self {
        Self { refresh, access }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct Account {
    state: AccountState,
}

impl Account {
    pub fn new() -> Self {
        Self {
            state: AccountState::InitialSetup,
        }
    }

    pub fn new_from(state: AccountState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> AccountState {
        self.state
    }

    pub fn complete_setup(&mut self) {
        if self.state == AccountState::InitialSetup {
            self.state = AccountState::Normal;
        }
    }
}

impl Default for Account {
    fn default() -> Self {
        Self {
            state: AccountState::InitialSetup,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub enum AccountState {
    InitialSetup,
    Normal,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default, PartialEq, Eq)]
pub struct AccountSetup {
    email: String,
}

impl AccountSetup {
    pub fn email(&self) -> &str {
        &self.email
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct SignInWithLoginInfo {
    pub apple_token: Option<String>,
    pub google_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignInWithInfo {
    pub google_account_id: Option<GoogleAccountId>,
}

#[derive(Debug, Clone, sqlx::Type, PartialEq)]
#[sqlx(transparent)]
pub struct GoogleAccountId(pub String);
