use std::fmt::Debug;

use api_client::{apis::calculator_api, models::CalculatorState};
use async_trait::async_trait;
use error_stack::Result;

use super::{super::super::client::TestError, BotAction, PreviousValue};

use crate::utils::IntoReportExt;

use super::BotState;

#[derive(Debug)]
pub struct ChangeCalculatorState {
    pub state: &'static str,
}

#[async_trait]
impl BotAction for ChangeCalculatorState {
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        let s = CalculatorState::new(self.state.to_string());
        api_client::apis::calculator_api::post_calculator_state(state.api.calculator(), s)
            .await
            .into_error(TestError::ApiRequest)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct GetCalculatorState;

#[async_trait]
impl BotAction for GetCalculatorState {
    async fn excecute_impl(&self, state: &mut BotState) -> Result<(), TestError> {
        let data = calculator_api::get_calculator_state(state.api.calculator())
            .await
            .into_error(TestError::ApiRequest)?;
        state.previous_value = PreviousValue::CalculatorState(data.state);
        Ok(())
    }

    fn previous_value_supported(&self) -> bool {
        true
    }
}
