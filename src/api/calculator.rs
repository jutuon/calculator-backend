pub mod data;

use axum::{Extension, Json};

use hyper::StatusCode;

use self::data::{CalculatorState, CalculatorStateInternal};

use super::{model::AccountIdInternal, GetInternalApi, GetUsers};

use tracing::error;

use super::{GetApiKeys, ReadDatabase, WriteDatabase};

// TODO: Add timeout for database commands

pub const PATH_GET_CALCULATOR_STATE: &str = "/calculator_api/state";

/// Get account's current calculator state.
///
#[utoipa::path(
    get,
    path = "/calculator_api/state",
    responses(
        (status = 200, description = "Get current state.", body = CalculatorState),
        (status = 401, description = "Unauthorized."),
        (
            status = 500,
            description = "Internal server error",
        ),
    ),
    security(("api_key" = [])),
)]
pub async fn get_calculator_state<
    S: ReadDatabase + GetUsers + GetApiKeys + GetInternalApi + WriteDatabase,
>(
    Extension(account_id): Extension<AccountIdInternal>,
    state: S,
) -> Result<Json<CalculatorState>, StatusCode> {
    state
        .read_database()
        .read_json::<CalculatorStateInternal>(account_id)
        .await
        .map(|state| {
            let state: CalculatorState = state.into();
            state.into()
        })
        .map_err(|e| {
            error!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub const PATH_POST_CALCULATOR_STATE: &str = "/calculator_api/state";

/// Update calculator state.
#[utoipa::path(
    post,
    path = "/calculator_api/state",
    request_body = CalculatorState,
    responses(
        (status = 200, description = "Update state"),
        (status = 401, description = "Unauthorized."),
        (
            status = 500,
            description = "Internal server error."
        ),
    ),
    security(("api_key" = [])),
)]
pub async fn post_calculator_state<S: GetApiKeys + WriteDatabase + ReadDatabase>(
    Extension(account_id): Extension<AccountIdInternal>,
    Json(calculator_state): Json<CalculatorState>,
    state: S,
) -> Result<(), StatusCode> {
    let new = CalculatorStateInternal {
        state: calculator_state.state,
    };

    state
        .write_database()
        .calculator()
        .update_calculator_state(account_id, new)
        .await
        .map_err(|e| {
            error!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR // Database writing failed.
        })?;

    Ok(())
}
