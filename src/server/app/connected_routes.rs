use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::api::{self};

use super::AppState;

/// Private routes only accessible when WebSocket is connected.
/// Debug mode allows also connection without the WebSocket connection.
pub struct ConnectedApp {
    state: AppState,
}

impl ConnectedApp {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> AppState {
        self.state.clone()
    }

    pub fn private_account_server_router(&self) -> Router {
        let private = Router::new()
            .route(
                api::account::PATH_ACCOUNT_STATE,
                get({
                    let state = self.state.clone();
                    move |body| api::account::get_account_state(body, state)
                }),
            )
            .route(
                api::account::PATH_ACCOUNT_SETUP,
                post({
                    let state = self.state.clone();
                    move |arg1, arg2| api::account::post_account_setup(arg1, arg2, state)
                }),
            )
            .route(
                api::account::PATH_ACCOUNT_COMPLETE_SETUP,
                post({
                    let state = self.state.clone();
                    move |arg1| api::account::post_complete_setup(arg1, state)
                }),
            )
            .route_layer({
                middleware::from_fn({
                    let state = self.state.clone();
                    move |addr, req, next| {
                        api::utils::authenticate_with_api_key(state.clone(), addr, req, next)
                    }
                })
            });

        Router::new().merge(private)
    }

    pub fn private_calculator_server_router(&self) -> Router {
        let private = Router::new()
            .route(
                api::calculator::PATH_GET_CALCULATOR_STATE,
                get({
                    let state = self.state.clone();
                    move |param1| api::calculator::get_calculator_state(param1, state)
                }),
            )
            .route(
                api::calculator::PATH_POST_CALCULATOR_STATE,
                post({
                    let state = self.state.clone();
                    move |header, body| api::calculator::post_calculator_state(header, body, state)
                }),
            )
            .route_layer({
                middleware::from_fn({
                    let state = self.state.clone();
                    move |addr, req, next| {
                        api::utils::authenticate_with_api_key(state.clone(), addr, req, next)
                    }
                })
            });

        Router::new().merge(private)
    }
}
