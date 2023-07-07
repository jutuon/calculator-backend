//! Bots for fake clients

use std::{
    fmt::Debug,
    iter::Peekable,
    time::{Duration, Instant},
};

use api_client::{
    apis::{account_api::get_account_state, calculator_api::get_calculator_state},
    models::AccountState,
};
use async_trait::async_trait;
use tokio::time::sleep;

use crate::{
    action_array,
    test::{
        bot::actions::{account::CompleteAccountSetup, ActionArray},
        client::TestError,
    },
};

use super::{
    actions::{
        account::{AssertAccountState, Login, Register, SetAccountSetup},
        BotAction, RunActions,
    },
    BotState, BotStruct, TaskState,
};

use error_stack::Result;

use crate::utils::IntoReportExt;

pub struct ClientBot {
    state: BotState,
    actions: Peekable<Box<dyn Iterator<Item = &'static dyn BotAction> + Send + Sync>>,
}

impl Debug for ClientBot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ClientBot").finish()
    }
}

impl ClientBot {
    pub fn new(state: BotState) -> Self {
        let setup = [&Register as &dyn BotAction, &Login, &DoInitialSetupIfNeeded];
        let benchmark = [
            &ActionsBeforeIteration as &dyn BotAction,
            &GetCalculatorState,
            &ActionsAfterIteration,
        ];
        let iter = setup.into_iter().chain(benchmark.into_iter().cycle());
        Self {
            state,
            actions: (Box::new(iter)
                as Box<dyn Iterator<Item = &'static dyn BotAction> + Send + Sync>)
                .peekable(),
        }
    }
}

#[async_trait]
impl BotStruct for ClientBot {
    fn peek_action_and_state(&mut self) -> (Option<&'static dyn BotAction>, &mut BotState) {
        (self.actions.peek().copied(), &mut self.state)
    }
    fn next_action(&mut self) {
        self.actions.next();
    }
    fn state(&self) -> &BotState {
        &self.state
    }
}

#[derive(Debug)]
pub struct GetCalculatorState;

#[async_trait]
impl BotAction for GetCalculatorState {
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        get_calculator_state(state.api.calculator())
            .await
            .into_error(TestError::ApiRequest)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DoInitialSetupIfNeeded;

#[async_trait]
impl BotAction for DoInitialSetupIfNeeded {
    async fn excecute_impl_task_state(
        &self,
        state: &mut BotState,
        task_state: &mut TaskState,
    ) -> Result<(), TestError> {
        let account_state = get_account_state(state.api.account())
            .await
            .into_error(TestError::ApiRequest)?;

        if account_state.state == AccountState::InitialSetup {
            const ACTIONS: ActionArray = action_array!(
                SetAccountSetup::new(),
                CompleteAccountSetup,
                AssertAccountState(AccountState::Normal),
            );
            RunActions(ACTIONS)
                .excecute_impl_task_state(state, task_state)
                .await?;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct ActionsBeforeIteration;

#[async_trait]
impl BotAction for ActionsBeforeIteration {
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        if !state.config.no_sleep {
            sleep(Duration::from_millis(1000)).await;
        }

        state.benchmark.action_duration = Instant::now();

        Ok(())
    }
}

#[derive(Debug)]
struct ActionsAfterIteration;

#[async_trait]
impl BotAction for ActionsAfterIteration {
    async fn excecute_impl(&self, _state: &mut BotState) -> Result<(), TestError> {
        Ok(())
    }
}
