pub mod data;
pub mod internal;

use axum::{Extension, Json, TypedHeader};

use futures::FutureExt;
use hyper::StatusCode;

use self::data::{
    Account, AccountIdInternal, AccountIdLight, AccountSetup, AccountState, ApiKey, AuthPair,
    GoogleAccountId, LoginResult, RefreshToken, SignInWithInfo, SignInWithLoginInfo,
};

use super::{GetConfig, GetInternalApi, SignInWith};

use tracing::error;

use super::{utils::ApiKeyHeader, GetApiKeys, GetUsers, ReadDatabase, WriteDatabase};

use tokio_stream::StreamExt;

pub const PATH_REGISTER: &str = "/account_api/register";

/// Register new account. Returns new account ID which is UUID.
#[utoipa::path(
    post,
    path = "/account_api/register",
    security(),
    responses(
        (status = 200, description = "New account created.", body = AccountIdLight),
        (status = 500, description = "Internal server error."),
    )
)]
pub async fn post_register<S: WriteDatabase + GetConfig>(
    state: S,
) -> Result<Json<AccountIdLight>, StatusCode> {
    register_impl(&state, SignInWithInfo::default())
        .await
        .map(|id| id.into())
}

pub async fn register_impl<S: WriteDatabase + GetConfig>(
    state: &S,
    sign_in_with: SignInWithInfo,
) -> Result<AccountIdLight, StatusCode> {
    // New unique UUID is generated every time so no special handling needed
    // to avoid database collisions.
    let id = AccountIdLight::new(uuid::Uuid::new_v4());

    let a = state.write_database().account();
    let register = a.register(id, sign_in_with);
    match register.await {
        Ok(id) => Ok(id.as_light().into()),
        Err(e) => {
            error!("Error: {e:?}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub const PATH_LOGIN: &str = "/account_api/login";

/// Get new ApiKey.
#[utoipa::path(
    post,
    path = "/account_api/login",
    security(),
    request_body = AccountIdLight,
    responses(
        (status = 200, description = "Login successful.", body = LoginResult),
        (status = 500, description = "Internal server error."),
    ),
)]
pub async fn post_login<S: GetApiKeys + WriteDatabase + GetUsers>(
    Json(id): Json<AccountIdLight>,
    state: S,
) -> Result<Json<LoginResult>, StatusCode> {
    login_impl(id, state).await.map(|d| d.into())
}

async fn login_impl<S: GetApiKeys + WriteDatabase + GetUsers>(
    id: AccountIdLight,
    state: S,
) -> Result<LoginResult, StatusCode> {
    let access = ApiKey::generate_new();
    let refresh = RefreshToken::generate_new();

    let id = state.users().get_internal_id(id).await.map_err(|e| {
        error!("Login error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let account = AuthPair { access, refresh };

    state
        .write_database()
        .set_new_auth_pair(id, account.clone(), None)
        .await
        .map_err(|e| {
            error!("Login error: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR // Database writing failed.
        })?;

    // TODO: microservice support

    let result = LoginResult {
        account,
        calculator: None,
    };
    Ok(result.into())
}

pub const PATH_SIGN_IN_WITH_LOGIN: &str = "/account_api/sign_in_with_login";

/// Start new session with sign in with Apple or Google. Creates new account if
/// it does not exists.
#[utoipa::path(
    post,
    path = "/account_api/sign_in_with_login",
    security(),
    request_body = SignInWithLoginInfo,
    responses(
        (status = 200, description = "Login or account creation successful.", body = LoginResult),
        (status = 500, description = "Internal server error."),
    ),
)]
pub async fn post_sign_in_with_login<
    S: GetApiKeys + WriteDatabase + GetUsers + SignInWith + GetConfig,
>(
    Json(tokens): Json<SignInWithLoginInfo>,
    state: S,
) -> Result<Json<LoginResult>, StatusCode> {
    if let Some(google) = tokens.google_token {
        let info = state
            .sign_in_with_manager()
            .validate_google_token(google)
            .await
            .map_err(|e| {
                error!("{e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        let google_id = GoogleAccountId(info.id);
        let already_existing_account = state
            .users()
            .get_account_with_google_account_id(google_id.clone())
            .await
            .map_err(|e| {
                error!("{e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if let Some(already_existing_account) = already_existing_account {
            login_impl(already_existing_account.as_light(), state)
                .await
                .map(|d| d.into())
        } else {
            let id = register_impl(
                &state,
                SignInWithInfo {
                    google_account_id: Some(google_id),
                },
            )
            .await?;
            login_impl(id, state).await.map(|d| d.into())
        }
    } else if let Some(apple) = tokens.apple_token {
        let _info = state
            .sign_in_with_manager()
            .validate_apple_token(apple)
            .await
            .map_err(|e| {
                error!("{e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // if validate_sign_in_with_apple_token(apple).await.unwrap() {
        //     let key = ApiKey::generate_new();
        //     Ok(key.into())
        // } else {
        //     Err(StatusCode::INTERNAL_SERVER_ERROR)
        // }
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub const PATH_ACCOUNT_STATE: &str = "/account_api/state";

/// Get current account state.
#[utoipa::path(
    get,
    path = "/account_api/state",
    responses(
        (status = 200, description = "Request successfull.", body = Account),
        (status = 401, description = "Unauthorized."),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn get_account_state<S: GetApiKeys + ReadDatabase>(
    TypedHeader(api_key): TypedHeader<ApiKeyHeader>,
    state: S,
) -> Result<Json<Account>, StatusCode> {
    let id = state
        .api_keys()
        .api_key_exists(api_key.key())
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    state
        .read_database()
        .read_json::<Account>(id)
        .await
        .map(|account| account.into())
        .map_err(|e| {
            error!("Get account state: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR // Database reading failed.
        })
}

pub const PATH_ACCOUNT_SETUP: &str = "/account_api/setup";

/// Setup non-changeable user information during `initial setup` state.
#[utoipa::path(
    post,
    path = "/account_api/setup",
    request_body(content = AccountSetup),
    responses(
        (status = 200, description = "Request successfull."),
        (status = 406, description = "Current state is not initial setup."),
        (status = 401, description = "Unauthorized."),
        (
            status = 500,
            description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn post_account_setup<S: GetApiKeys + ReadDatabase + WriteDatabase>(
    Extension(id): Extension<AccountIdInternal>,
    Json(data): Json<AccountSetup>,
    state: S,
) -> Result<(), StatusCode> {
    let account = state
        .read_database()
        .read_json::<Account>(id)
        .await
        .map_err(|e| {
            error!("error: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR // Database reading failed.
        })?;

    if account.state() == AccountState::InitialSetup {
        state
            .write_database()
            .account()
            .update_account_setup(id, data)
            .await
            .map_err(|e| {
                error!("Write database error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR // Database writing failed.
            })
    } else {
        Err(StatusCode::NOT_ACCEPTABLE)
    }
}

pub const PATH_ACCOUNT_COMPLETE_SETUP: &str = "/account_api/complete_setup";

/// Complete initial setup.
///
/// Request to this handler will complete if client is in `initial setup`,
/// setup information is set.
///
#[utoipa::path(
    post,
    path = "/account_api/complete_setup",
    responses(
        (status = 200, description = "Request successfull."),
        (status = 406, description = "Current state is not initial setup or AccountSetup is empty."),
        (status = 401, description = "Unauthorized."),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn post_complete_setup<
    S: GetApiKeys + ReadDatabase + WriteDatabase + GetInternalApi + GetConfig,
>(
    Extension(id): Extension<AccountIdInternal>,
    state: S,
) -> Result<(), StatusCode> {
    let account_setup = state
        .read_database()
        .read_json::<AccountSetup>(id)
        .await
        .map_err(|e| {
            error!("Complete setup error: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR // Database reading failed.
        })?;

    if account_setup.email().is_empty() {
        return Err(StatusCode::NOT_ACCEPTABLE);
    }

    let mut account = state
        .read_database()
        .read_json::<Account>(id)
        .await
        .map_err(|e| {
            error!("Complete setup error: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR // Database reading failed.
        })?;

    if account.state() == AccountState::InitialSetup {
        account.complete_setup();

        state
            .write_database()
            .account()
            .update_account(id, account)
            .await
            .map_err(|e| {
                error!("Write database error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR // Database writing failed.
            })
    } else {
        Err(StatusCode::NOT_ACCEPTABLE)
    }
}

pub const PATH_POST_DELETE: &str = "/account_api/delete";

/// Delete account.
#[utoipa::path(
    put,
    path = "/account_api/delete",
    responses(
        (status = 200, description = "All account data is now deleted."),
        (status = 401, description = "Unauthorized."),
        (status = 500, description = "Internal server error."),
    ),
    security(("api_key" = [])),
)]
pub async fn post_delete<S: GetApiKeys + WriteDatabase + ReadDatabase>(
    _state: S,
) -> Result<(), StatusCode> {
    // TODO: implement
    Err(StatusCode::INTERNAL_SERVER_ERROR)
}
