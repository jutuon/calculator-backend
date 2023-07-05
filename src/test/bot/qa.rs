//! QA testing
//!

pub mod account;
pub mod calculator;
pub mod common;

use std::{fmt::Debug, iter::Peekable, sync::atomic::AtomicBool};

use async_trait::async_trait;

use self::{account::ACCOUNT_TESTS, calculator::CALCULATOR_TESTS, common::COMMON_TESTS};

use super::{actions::BotAction, BotState, BotStruct};

pub type SingleTest = (&'static str, &'static [&'static [&'static dyn BotAction]]);

#[macro_export]
macro_rules! test {
    ($s:expr,[ $( $actions:expr, )* ] ) => {
        (
            $s,
            &[
                &[   $( &($actions) as &dyn BotAction, )*    ]
            ]
        )
    };
}

pub const ALL_QA_TESTS: &'static [&'static [SingleTest]] =
    &[ACCOUNT_TESTS, CALCULATOR_TESTS, COMMON_TESTS];

pub fn test_count() -> usize {
    ALL_QA_TESTS.iter().map(|tests| tests.len()).sum()
}

#[derive(Debug)]
pub struct QaState {}

impl QaState {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct Qa {
    state: BotState,
    test_name: &'static str,
    actions: Peekable<Box<dyn Iterator<Item = &'static dyn BotAction> + Send + Sync>>,
}

impl Debug for Qa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(self.test_name).finish()
    }
}

impl Qa {
    pub fn user_test(
        state: BotState,
        test_name: &'static str,
        actions: Box<dyn Iterator<Item = &'static dyn BotAction> + Send + Sync>,
    ) -> Self {
        Self {
            state,
            test_name,
            actions: actions.peekable(),
        }
    }
}

#[async_trait]
impl BotStruct for Qa {
    fn peek_action_and_state(&mut self) -> (Option<&'static dyn BotAction>, &mut BotState) {
        let action = self.actions.peek().map(|a| *a);
        (action, &mut self.state)
    }

    fn next_action(&mut self) {
        self.actions.next();
    }

    fn state(&self) -> &BotState {
        &self.state
    }

    fn notify_task_bot_count_decreased(&mut self, _bot_count: usize) {}
}
