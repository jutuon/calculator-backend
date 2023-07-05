use super::{ResultSender, SendBack, WriteCommandRunner, WriteCommandRunnerHandle};

use error_stack::Result;

use crate::{
    api::{calculator::data::CalculatorStateInternal, model::AccountIdInternal},
    server::database::DatabaseError,
};

/// Synchronized write commands.
#[derive(Debug)]
pub enum CalculatorWriteCommand {
    UpdateCalculatorState {
        s: ResultSender<()>,
        account_id: AccountIdInternal,
        data: CalculatorStateInternal,
    },
}

#[derive(Debug, Clone)]
pub struct CalculatorWriteCommandRunnerHandle<'a> {
    pub handle: &'a WriteCommandRunnerHandle,
}

impl CalculatorWriteCommandRunnerHandle<'_> {
    pub async fn update_calculator_state(
        &self,
        account_id: AccountIdInternal,
        data: CalculatorStateInternal,
    ) -> Result<(), DatabaseError> {
        self.handle
            .send_event(|s| CalculatorWriteCommand::UpdateCalculatorState {
                s,
                account_id,
                data,
            })
            .await
    }
}

impl WriteCommandRunner {
    pub async fn handle_calculator_cmd(&self, cmd: CalculatorWriteCommand) {
        match cmd {
            CalculatorWriteCommand::UpdateCalculatorState {
                s,
                account_id,
                data,
            } => self.write().update_data(account_id, &data).await.send(s),
        }
    }
}
