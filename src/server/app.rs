pub mod connected_routes;
pub mod connection;
pub mod sign_in_with;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    api::{
        self, GetApiKeys, GetConfig, GetInternalApi, GetUsers, ReadDatabase, SignInWith,
        WriteDatabase,
    },
    config::Config,
};

use self::{
    connected_routes::ConnectedApp, connection::WebSocketManager, sign_in_with::SignInWithManager,
};

use super::{
    database::{
        commands::WriteCommandRunnerHandle,
        read::ReadCommands,
        utils::{AccountIdManager, ApiKeyManager},
        RouterDatabaseReadHandle,
    },
    internal::{InternalApiClient, InternalApiManager},
};

#[derive(Clone)]
pub struct AppState {
    database: Arc<RouterDatabaseReadHandle>,
    internal_api: Arc<InternalApiClient>,
    config: Arc<Config>,
    sign_in_with: Arc<SignInWithManager>,
}

impl GetApiKeys for AppState {
    fn api_keys(&self) -> ApiKeyManager<'_> {
        self.database.api_key_manager()
    }
}

impl GetUsers for AppState {
    fn users(&self) -> AccountIdManager<'_> {
        self.database.account_id_manager()
    }
}

impl ReadDatabase for AppState {
    fn read_database(&self) -> ReadCommands<'_> {
        self.database.read()
    }
}

impl WriteDatabase for AppState {
    fn write_database(&self) -> &WriteCommandRunnerHandle {
        self.database.write()
    }
}

impl SignInWith for AppState {
    fn sign_in_with_manager(&self) -> &SignInWithManager {
        &self.sign_in_with
    }
}

impl GetInternalApi for AppState {
    fn internal_api(&self) -> InternalApiManager {
        InternalApiManager::new(
            &self.config,
            &self.internal_api,
            self.api_keys(),
            self.read_database(),
            self.write_database(),
            self.database.account_id_manager(),
        )
    }
}

impl GetConfig for AppState {
    fn config(&self) -> &Config {
        &self.config
    }
}

pub struct App {
    state: AppState,
    ws_manager: Option<WebSocketManager>,
}

impl App {
    pub async fn new(
        database_handle: RouterDatabaseReadHandle,
        config: Arc<Config>,
        ws_manager: WebSocketManager,
    ) -> Self {
        let state = AppState {
            config: config.clone(),
            database: Arc::new(database_handle),
            internal_api: InternalApiClient::new(config.external_service_urls().clone()).into(),
            sign_in_with: SignInWithManager::new(config).into(),
        };

        Self {
            state,
            ws_manager: Some(ws_manager),
        }
    }

    pub fn state(&self) -> AppState {
        self.state.clone()
    }

    pub fn create_common_server_router(&mut self) -> Router {
        Router::new().route(
            api::common::PATH_CONNECT,
            get({
                let state = self.state.clone();
                let ws_manager = self.ws_manager.take().unwrap(); // Only one instance required.
                move |param1, param2, param3| {
                    api::common::get_connect_websocket(param1, param2, param3, state, ws_manager)
                }
            }),
        )
        // This route checks the access token by itself.
    }

    pub fn create_account_server_router(&self) -> Router {
        let public = Router::new()
            .route(
                api::account::PATH_REGISTER,
                post({
                    let state = self.state.clone();
                    move || api::account::post_register(state)
                }),
            )
            .route(
                api::account::PATH_LOGIN,
                post({
                    let state = self.state.clone();
                    move |body| api::account::post_login(body, state)
                }),
            )
            .route(
                api::account::PATH_SIGN_IN_WITH_LOGIN,
                post({
                    let state = self.state.clone();
                    move |body| api::account::post_sign_in_with_login(body, state)
                }),
            );

        public.merge(ConnectedApp::new(self.state.clone()).private_account_server_router())
    }

    pub fn create_calculator_server_router(&self) -> Router {
        let public = Router::new();

        public.merge(ConnectedApp::new(self.state.clone()).private_calculator_server_router())
    }
}
