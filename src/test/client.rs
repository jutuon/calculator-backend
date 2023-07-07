//! Access REST API from Rust

use std::fmt::Debug;

use api_client::apis::configuration::Configuration;
use error_stack::{IntoReport, Result};

use hyper::StatusCode;
use reqwest::{Client, Url};
use tracing::info;

#[derive(thiserror::Error, Debug)]
#[error("Wrong status code: {0}")]
pub struct StatusCodeError(StatusCode);

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum TestError {
    #[error("Reqwest error")]
    Reqwest,

    #[error("WebSocket error")]
    WebSocket,
    #[error("WebSocket wrong value received")]
    WebSocketWrongValue,

    // Other errors
    #[error("Serde deserialization error")]
    SerdeDeserialize,

    #[error("API URL not configured")]
    ApiUrlNotConfigured,

    #[error("Wrong status code")]
    StatusCode,

    #[error("Joining text to URL failed")]
    ApiUrlJoinError,

    #[error("Missing value")]
    MissingValue,

    #[error("API request failed")]
    ApiRequest,

    #[error("Account ID missing from bot state")]
    AccountIdMissing,

    #[error("Assert error. message: {0}")]
    AssertError(String),

    #[error("Not an error. Just an indication that bot is waiting.")]
    BotIsWaiting,
}

#[derive(Debug, Clone)]
pub struct PublicApiUrls {
    /// Account API url for register and login API
    pub register_base_url: Url,
    pub account_base_url: Url,
    pub calculator_base_url: Url,
}

impl PublicApiUrls {
    pub fn new(register_base_url: Url, account_base_url: Url, calculator_base_url: Url) -> Self {
        Self {
            register_base_url,
            account_base_url,
            calculator_base_url,
        }
    }
}

#[derive(Debug)]
pub struct ApiClient {
    /// Where Account API reqister and login is available
    register: Configuration,
    account: Configuration,
    calculator: Configuration,
}

impl ApiClient {
    pub fn new(base_urls: PublicApiUrls) -> Self {
        let client = reqwest::Client::new();

        Self {
            register: Self::create_configuration(&client, base_urls.register_base_url.as_str()),
            account: Self::create_configuration(&client, base_urls.account_base_url.as_str()),
            calculator: Self::create_configuration(&client, base_urls.calculator_base_url.as_str()),
        }
    }

    fn create_configuration(client: &Client, base_url: &str) -> Configuration {
        let path = base_url.trim_end_matches('/').to_string();
        Configuration {
            base_path: path,
            client: client.clone(),
            ..Configuration::default()
        }
    }

    pub fn print_to_log(&self) {
        info!("Register API base url: {}", self.register.base_path);
        info!("Account API base url: {}", self.account.base_path);
        info!("Calculator API base url: {}", self.calculator.base_path);
    }

    pub fn register(&self) -> &Configuration {
        &self.register
    }

    pub fn account(&self) -> &Configuration {
        &self.account
    }

    pub fn calculator(&self) -> &Configuration {
        &self.calculator
    }

    pub fn set_access_token(&mut self, token: String) {
        let token = api_client::apis::configuration::ApiKey {
            prefix: None,
            key: token,
        };
        self.account.api_key = Some(token.clone());
        self.calculator.api_key = Some(token.clone());
    }

    pub fn is_access_token_available(&self) -> bool {
        self.account.api_key.is_some() && self.calculator.api_key.is_some()
    }

    pub fn api_key(&self) -> Option<String> {
        self.account.api_key.clone().map(|k| k.key)
    }
}

pub fn get_api_url(url: &Option<Url>) -> Result<Url, TestError> {
    url.as_ref()
        .ok_or(TestError::ApiUrlNotConfigured)
        .map(Clone::clone)
        .into_report()
}
