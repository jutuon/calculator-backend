use std::fmt::Debug;

use api_client::{
    apis::account_api::{
        get_account_state, post_account_setup, post_complete_setup, post_login, post_register,
    },
    models::{auth_pair, AccountSetup, AccountState},
};
use async_trait::async_trait;

use base64::Engine;
use error_stack::{IntoReport, Result};
use futures::SinkExt;
use headers::HeaderValue;
use tokio_stream::StreamExt;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
use url::Url;

use super::{super::super::client::TestError, BotAction};

use crate::{
    api::{common::PATH_CONNECT, utils::API_KEY_HEADER_STR},
    test::bot::{utils::assert::bot_assert_eq, WsConnection},
    utils::IntoReportExt,
};

use super::BotState;

#[derive(Debug)]
pub struct Register;

#[async_trait]
impl BotAction for Register {
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        if state.id.is_some() {
            return Ok(());
        }

        let id = post_register(state.api.account())
            .await
            .into_error(TestError::ApiRequest)?;
        state.id = Some(id);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Login;

#[async_trait]
impl BotAction for Login {
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        if state.api.is_access_token_available() {
            return Ok(());
        }
        let login_result = post_login(state.api.account(), state.id()?)
            .await
            .into_error(TestError::ApiRequest)?;

        state
            .api
            .set_access_token(login_result.account.access.api_key.clone());

        let url = state
            .config
            .server
            .api_urls
            .account_base_url
            .join(PATH_CONNECT)
            .into_error(TestError::WebSocket)?;
        state.connections.account = connect_websocket(*login_result.account, url, state)
            .await?
            .into();

        if let Some(calculator) = login_result.calculator.flatten() {
            let url = state
                .config
                .server
                .api_urls
                .calculator_base_url
                .join(PATH_CONNECT)
                .into_error(TestError::WebSocket)?;
            state.connections.calculator = connect_websocket(*calculator, url, state).await?.into();
        }

        Ok(())
    }
}

async fn connect_websocket(
    auth: auth_pair::AuthPair,
    mut url: Url,
    state: &mut BotState,
) -> Result<WsConnection, TestError> {
    if url.scheme() == "https" {
        url.set_scheme("wss")
            .map_err(|_| TestError::WebSocket)
            .into_report()?;
    }
    if url.scheme() == "http" {
        url.set_scheme("ws")
            .map_err(|_| TestError::WebSocket)
            .into_report()?;
    }

    let mut r = url.into_client_request().into_error(TestError::WebSocket)?;
    r.headers_mut().insert(
        API_KEY_HEADER_STR,
        HeaderValue::from_str(&auth.access.api_key).into_error(TestError::WebSocket)?,
    );
    let (mut stream, _) = tokio_tungstenite::connect_async(r)
        .await
        .into_error(TestError::WebSocket)?;

    let binary_token = base64::engine::general_purpose::STANDARD
        .decode(auth.refresh.token)
        .into_error(TestError::WebSocket)?;
    stream
        .send(Message::Binary(binary_token))
        .await
        .into_error(TestError::WebSocket)?;

    let refresh_token = stream
        .next()
        .await
        .ok_or(TestError::WebSocket)
        .into_report()?
        .into_error(TestError::WebSocket)?;
    match refresh_token {
        Message::Binary(refresh_token) => state.refresh_token = Some(refresh_token),
        _ => return Err(TestError::WebSocketWrongValue).into_report(),
    }

    let access_token = stream
        .next()
        .await
        .ok_or(TestError::WebSocket)
        .into_report()?
        .into_error(TestError::WebSocket)?;
    match access_token {
        Message::Text(access_token) => state.api.set_access_token(access_token),
        _ => return Err(TestError::WebSocketWrongValue).into_report(),
    }
    Ok(stream)
}

#[derive(Debug)]
pub struct AssertAccountState(pub AccountState);

#[async_trait]
impl BotAction for AssertAccountState {
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        let state = get_account_state(state.api.account())
            .await
            .into_error(TestError::ApiRequest)?;

        bot_assert_eq(state.state, self.0)
    }
}

#[derive(Debug)]
pub struct SetAccountSetup {
    pub email: Option<&'static str>,
}

impl SetAccountSetup {
    pub const fn new() -> Self {
        Self { email: None }
    }
}

#[async_trait]
impl BotAction for SetAccountSetup {
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        let setup = AccountSetup {
            email: self
                .email
                .map(|email| email.to_string())
                .unwrap_or(format!("test@example.com")),
        };
        post_account_setup(state.api.account(), setup)
            .await
            .into_error(TestError::ApiRequest)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct CompleteAccountSetup;

#[async_trait]
impl BotAction for CompleteAccountSetup {
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        post_complete_setup(state.api.account())
            .await
            .into_error(TestError::ApiRequest)?;

        Ok(())
    }
}
