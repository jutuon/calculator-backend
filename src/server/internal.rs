//! Routes for server to server connections

use api_client::apis::{accountinternal_api, configuration::Configuration};
use axum::{routing::get, Router};

use error_stack::Result;

use hyper::StatusCode;

use tracing::{error, info};

use crate::{
    api::{self},
    config::InternalApiUrls,
    utils::IntoReportExt,
};

use crate::{api::model::ApiKey, config::Config};

use super::{
    app::AppState,
    database::{
        commands::WriteCommandRunnerHandle,
        read::ReadCommands,
        utils::{AccountIdManager, ApiKeyManager},
    },
};

// TODO: Use TLS for checking that all internal communication comes from trusted
//       sources.

#[derive(thiserror::Error, Debug)]
pub enum InternalApiError {
    #[error("API request failed")]
    ApiRequest,

    #[error("Database call failed")]
    DatabaseError,

    #[error("Account API URL not configured")]
    AccountApiUrlNotConfigured,

    #[error("Missing value")]
    MissingValue,

    #[error("Invalid value")]
    InvalidValue,
}

/// Internal route handlers for server to server communication.
pub struct InternalApp;

impl InternalApp {
    pub fn create_account_server_router(state: AppState) -> Router {
        Router::new()
            .route(
                api::account::internal::PATH_INTERNAL_CHECK_API_KEY,
                get({
                    let state = state.clone();
                    move |body| api::account::internal::check_api_key(body, state)
                }),
            )
            .route(
                api::account::internal::PATH_INTERNAL_GET_ACCOUNT_STATE,
                get({
                    let state = state.clone();
                    move |param1| api::account::internal::internal_get_account_state(param1, state)
                }),
            )
    }
}

// TOOD: PrintWarningsTriggersAtomics?
pub struct PrintWarningsTriggersAtomics {}

pub struct InternalApiClient {
    account: Option<Configuration>,
}

impl InternalApiClient {
    pub fn new(base_urls: InternalApiUrls) -> Self {
        let client = reqwest::Client::new();

        let account = base_urls.account_base_url.map(|url| {
            let url = url.as_str().trim_end_matches('/').to_string();

            info!("Account internal API base url: {}", url);

            Configuration {
                base_path: url,
                client: client.clone(),
                ..Configuration::default()
            }
        });

        Self { account }
    }

    pub fn account(&self) -> Result<&Configuration, InternalApiError> {
        self.account
            .as_ref()
            .ok_or(InternalApiError::AccountApiUrlNotConfigured.into())
    }
}

pub enum AuthResponse {
    Ok,
    Unauthorized,
}

/// Handle requests to internal API. If the required feature is located
/// on the current server, then request is not made.
pub struct InternalApiManager<'a> {
    config: &'a Config,
    api_client: &'a InternalApiClient,
    keys: ApiKeyManager<'a>,
    read_database: ReadCommands<'a>,
    write_database: &'a WriteCommandRunnerHandle,
    account_id_manager: AccountIdManager<'a>,
}

impl<'a> InternalApiManager<'a> {
    pub fn new(
        config: &'a Config,
        api_client: &'a InternalApiClient,
        keys: ApiKeyManager<'a>,
        read_database: ReadCommands<'a>,
        write_database: &'a WriteCommandRunnerHandle,
        account_id_manager: AccountIdManager<'a>,
    ) -> Self {
        Self {
            config,
            api_client,
            keys,
            read_database,
            write_database,
            account_id_manager,
        }
    }

    /// Check that API key is valid. Use this only from ApiKey checker handler.
    /// This function will cache the account ID, so it can be found using normal
    /// database calls after this runs.
    pub async fn check_api_key(&self, key: ApiKey) -> Result<AuthResponse, InternalApiError> {
        if self.keys.api_key_exists(&key).await.is_some() {
            Ok(AuthResponse::Ok)
        } else if !self.config.components().account {
            // Check ApiKey from external service

            let result = accountinternal_api::check_api_key(
                self.api_client.account()?,
                api_client::models::ApiKey {
                    api_key: key.into_string(),
                },
            )
            .await;

            match result {
                Ok(_res) => {
                    // TODO: Cache this API key. Also needed for initializing
                    // database tables.
                    Ok(AuthResponse::Ok)
                }
                Err(api_client::apis::Error::ResponseError(response))
                    if response.status == StatusCode::UNAUTHORIZED =>
                {
                    // TODO: NOTE: Logging every error is not good as it would spam
                    // the log, but maybe an error counter or logging just
                    // once for a while.
                    Ok(AuthResponse::Unauthorized)
                }
                Err(e) => Err(e).into_error(InternalApiError::ApiRequest),
            }
        } else {
            Ok(AuthResponse::Unauthorized)
        }
    }
}
