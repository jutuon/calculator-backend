//! HTTP API types and request handlers for all servers.

// Routes
pub mod account;
pub mod calculator;
pub mod common;

pub mod model;
pub mod utils;

use utoipa::{Modify, OpenApi};

use crate::{
    config::Config,
    server::{
        app::sign_in_with::SignInWithManager,
        database::{
            commands::WriteCommandRunnerHandle,
            read::ReadCommands,
            utils::{AccountIdManager, ApiKeyManager},
        },
        internal::InternalApiManager,
    },
};

use utils::SecurityApiTokenDefault;

// API docs

#[derive(OpenApi)]
#[openapi(
    paths(
        common::get_connect_websocket,
        account::post_register,
        account::post_login,
        account::post_sign_in_with_login,
        account::post_account_setup,
        account::post_complete_setup,
        account::post_delete,
        account::get_account_state,
        account::internal::check_api_key,
        account::internal::internal_get_account_state,
        calculator::get_calculator_state,
        calculator::post_calculator_state,
    ),
    components(schemas(
        common::EventToClient,
        account::data::AccountIdLight,
        account::data::ApiKey,
        account::data::Account,
        account::data::AccountState,
        account::data::AccountSetup,
        account::data::SignInWithLoginInfo,
        account::data::LoginResult,
        account::data::RefreshToken,
        account::data::AuthPair,
        calculator::data::CalculatorState,
    )),
    modifiers(&SecurityApiTokenDefault),
    info(
        title = "calculator-backend",
        description = "Calculator backend API",
        version = "0.1.0"
    )
)]
pub struct ApiDoc;

// App state getters

pub trait GetApiKeys {
    /// Users which are logged in.
    fn api_keys(&self) -> ApiKeyManager<'_>;
}

pub trait GetUsers {
    /// All users registered in the service.
    fn users(&self) -> AccountIdManager<'_>;
}

pub trait WriteDatabase {
    fn write_database(&self) -> &WriteCommandRunnerHandle;
}

pub trait ReadDatabase {
    fn read_database(&self) -> ReadCommands<'_>;
}

pub trait SignInWith {
    fn sign_in_with_manager(&self) -> &SignInWithManager;
}

pub trait GetInternalApi {
    fn internal_api(&self) -> InternalApiManager;
}

pub trait GetConfig {
    fn config(&self) -> &Config;
}
