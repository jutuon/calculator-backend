use api_client::models::AccountState;

use crate::test::bot::actions::BotAction;

use super::{
    super::actions::{
        account::{AssertAccountState, CompleteAccountSetup, Login, Register, SetAccountSetup},
        AssertFailure,
    },
    SingleTest,
};

use crate::test;

pub const ACCOUNT_TESTS: &[SingleTest] = &[
    test!(
        "Initial setup: correct account state after login",
        [
            Register,
            Login,
            AssertAccountState(AccountState::InitialSetup),
        ]
    ),
    test!(
        "Initial setup: complete setup fails if no setup info is set",
        [
            Register,
            Login,
            AssertFailure(CompleteAccountSetup),
            AssertAccountState(AccountState::InitialSetup),
        ]
    ),
    test!(
        "Initial setup: successful",
        [
            Register,
            Login,
            SetAccountSetup::new(),
            CompleteAccountSetup,
            AssertAccountState(AccountState::Normal),
        ]
    ),
];
