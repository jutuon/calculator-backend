use async_trait::async_trait;
use error_stack::Result;

use crate::server::database::current::SqliteReadCommands;
use crate::server::database::sqlite::{SqliteDatabaseError, SqliteReadHandle, SqliteSelectJson};

use crate::api::model::*;

use crate::utils::IntoReportExt;

pub struct CurrentReadCalculatorCommands<'a> {
    handle: &'a SqliteReadHandle,
}

impl<'a> CurrentReadCalculatorCommands<'a> {
    pub fn new(handle: &'a SqliteReadHandle) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl SqliteSelectJson for CalculatorStateInternal {
    async fn select_json(
        id: AccountIdInternal,
        read: &SqliteReadCommands,
    ) -> Result<Self, SqliteDatabaseError> {
        let request = sqlx::query_as!(
            CalculatorStateInternal,
            r#"
            SELECT
                calculation as "state: _"
            FROM CurrentState
            WHERE account_row_id = ?
            "#,
            id.account_row_id,
        )
        .fetch_one(read.handle.pool())
        .await
        .into_error(SqliteDatabaseError::Fetch)?;

        Ok(request)
    }
}
