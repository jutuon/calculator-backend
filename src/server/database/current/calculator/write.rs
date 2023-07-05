use async_trait::async_trait;
use error_stack::Result;

use crate::server::database::current::CurrentDataWriteCommands;
use crate::server::database::sqlite::{
    CurrentDataWriteHandle, SqliteDatabaseError, SqliteSelectJson, SqliteUpdateJson,
};

use crate::api::model::*;

use crate::server::database::write::WriteResult;
use crate::utils::IntoReportExt;

pub struct CurrentWriteCalculatorCommands<'a> {
    handle: &'a CurrentDataWriteHandle,
}

impl<'a> CurrentWriteCalculatorCommands<'a> {
    pub fn new(handle: &'a CurrentDataWriteHandle) -> Self {
        Self { handle }
    }

    pub async fn init_calculator_state(
        &self,
        id: AccountIdInternal,
    ) -> WriteResult<CalculatorStateInternal, SqliteDatabaseError, CalculatorState> {
        sqlx::query!(
            r#"
            INSERT INTO CurrentState (account_row_id)
            VALUES (?)
            "#,
            id.account_row_id,
        )
        .execute(self.handle.pool())
        .await
        .into_error(SqliteDatabaseError::Execute)?;

        let state = CalculatorStateInternal::select_json(id, &self.handle.read()).await?;
        Ok(state)
    }
}

#[async_trait]
impl SqliteUpdateJson for CalculatorStateInternal {
    async fn update_json(
        &self,
        id: AccountIdInternal,
        write: &CurrentDataWriteCommands,
    ) -> Result<(), SqliteDatabaseError> {
        sqlx::query!(
            r#"
            UPDATE CurrentState
            SET calculation = ?
            WHERE account_row_id = ?
            "#,
            self.state,
            id.account_row_id,
        )
        .execute(write.handle.pool())
        .await
        .into_error(SqliteDatabaseError::Execute)?;

        Ok(())
    }
}
