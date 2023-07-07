pub mod apple;
pub mod google;

use std::sync::Arc;

use error_stack::Result;

use crate::config::Config;

use self::{
    apple::{AppleAccountId, SignInWithAppleError, SignInWithAppleManager},
    google::{GoogleAccountInfo, SignInWithGoogleError, SignInWithGoogleManager},
};
pub struct SignInWithManager {
    google: SignInWithGoogleManager,
    apple: SignInWithAppleManager,
}

impl SignInWithManager {
    pub fn new(config: Arc<Config>) -> Self {
        let client = reqwest::Client::new();
        Self {
            google: SignInWithGoogleManager::new(config.clone(), client.clone()),
            apple: SignInWithAppleManager::new(config.clone(), client.clone()),
        }
    }

    pub async fn validate_google_token(
        &self,
        token: String,
    ) -> Result<GoogleAccountInfo, SignInWithGoogleError> {
        self.google.validate_google_token(token).await
    }

    pub async fn validate_apple_token(
        &self,
        token: String,
    ) -> Result<AppleAccountId, SignInWithAppleError> {
        self.apple.validate_apple_token(token).await
    }
}
