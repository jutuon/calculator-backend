//! Bots for benchmarking

use std::{
    fmt::Debug,
    iter::Peekable,
    time::{Duration, Instant},
};

use api_client::apis::calculator_api::get_calculator_state;
use async_trait::async_trait;
use tokio::time::sleep;

use crate::test::client::TestError;

use super::{
    actions::{
        account::{Login, Register},
        calculator::ChangeCalculatorState,
        BotAction,
    },
    utils::{Counters, Timer},
    BotState, BotStruct, TaskState,
};

use error_stack::Result;

use tracing::log::info;

use crate::utils::IntoReportExt;

static COUNTERS: Counters = Counters::new();

#[derive(Debug)]
pub struct BenchmarkState {
    pub update_calculator_state_timer: Timer,
    pub print_info_timer: Timer,
    pub action_duration: Instant,
}

impl BenchmarkState {
    pub fn new() -> Self {
        Self {
            update_calculator_state_timer: Timer::new(Duration::from_millis(1000)),
            print_info_timer: Timer::new(Duration::from_millis(1000)),
            action_duration: Instant::now(),
        }
    }
}

pub struct Benchmark {
    state: BotState,
    actions: Peekable<Box<dyn Iterator<Item = &'static dyn BotAction> + Send + Sync>>,
}

impl Debug for Benchmark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Benchmark").finish()
    }
}

impl Benchmark {
    pub fn benchmark_get_calculator_state(state: BotState) -> Self {
        let setup = [&Register as &dyn BotAction, &Login];
        let benchmark = [
            &UpdateCalculatorStateBenchmark as &dyn BotAction,
            &ActionsBeforeIteration,
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
impl BotStruct for Benchmark {
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
pub struct UpdateCalculatorStateBenchmark;

#[async_trait]
impl BotAction for UpdateCalculatorStateBenchmark {
    async fn excecute_impl_task_state(
        &self,
        state: &mut BotState,
        task_state: &mut TaskState,
    ) -> Result<(), TestError> {
        let time = Instant::now();

        if state.config.update_calculator_state
            && state.benchmark.update_calculator_state_timer.passed()
        {
            ChangeCalculatorState { state: "0" }
                .excecute(state, task_state)
                .await?;

            if state.is_first_bot() {
                info!("update_calculator_state: {:?}", time.elapsed());
            }
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
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        COUNTERS.inc_get_calculator_state();

        if state.print_info() {
            info!(
                "{:?}: {:?}, total: {}",
                state.previous_action,
                state.benchmark.action_duration.elapsed(),
                COUNTERS.reset_get_calculator_state()
            );
        }
        Ok(())
    }
}
