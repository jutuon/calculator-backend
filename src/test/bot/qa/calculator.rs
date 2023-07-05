use crate::test::bot::actions::{
    calculator::{ChangeCalculatorState, GetCalculatorState},
    AssertEqualsFn, BotAction, RunActions, TO_NORMAL_STATE,
};

use super::SingleTest;

use crate::test;

pub const CALCULATOR_TESTS: &[SingleTest] = &[test!(
    "Calculator state: saving calculator state works multiple times",
    [
        RunActions(TO_NORMAL_STATE),
        ChangeCalculatorState { state: "0" },
        AssertEqualsFn(
            |v, _| v.calculator_state().as_deref() == Some("0"),
            true,
            &GetCalculatorState
        ),
        ChangeCalculatorState { state: "1" },
        AssertEqualsFn(
            |v, _| v.calculator_state().as_deref() == Some("1"),
            true,
            &GetCalculatorState
        ),
        ChangeCalculatorState { state: "2" },
        AssertEqualsFn(
            |v, _| v.calculator_state().as_deref() == Some("2"),
            true,
            &GetCalculatorState
        ),
    ]
)];
