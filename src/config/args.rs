use std::{
    convert::{TryFrom, TryInto},
    path::PathBuf,
};

use clap::{arg, command, value_parser, Command, PossibleValue};
use reqwest::Url;

use crate::test::client::PublicApiUrls;

// Config given as command line arguments
pub struct ArgsConfig {
    pub database_dir: Option<PathBuf>,
    pub test_mode: Option<TestMode>,
}

pub fn get_config() -> ArgsConfig {
    let matches = command!()
        .arg(
            arg!(--database <DIR> "Set database directory. Overrides config file value.")
                .required(false)
                .value_parser(value_parser!(PathBuf)),
        )
        .subcommand(
            Command::new("test")
                .about("Run tests and benchmarks")
                .arg(
                    arg!(--bots <COUNT> "Bot count per task")
                        .value_parser(value_parser!(u32))
                        .default_value("1")
                        .required(false),
                )
                .arg(
                    arg!(--tasks <COUNT> "Task count")
                        .value_parser(value_parser!(u32))
                        .default_value("1")
                        .required(false),
                )
                .arg(
                    arg!(--"url-register" <URL> "Base URL for account API for register and login")
                        .value_parser(value_parser!(Url))
                        .default_value("http://127.0.0.1:3001")
                        .required(false),
                )
                .arg(
                    arg!(--"url-account" <URL> "Base URL for account API")
                        .value_parser(value_parser!(Url))
                        .default_value("http://127.0.0.1:3000")
                        .required(false),
                )
                .arg(
                    arg!(--"url-calculator" <URL> "Base URL for calculator API")
                        .value_parser(value_parser!(Url))
                        .default_value("http://127.0.0.1:3000")
                        .required(false),
                )
                .arg(
                    arg!(--"test-database" <DIR> "Directory for test database")
                        .value_parser(value_parser!(PathBuf))
                        .default_value("tmp_databases")
                        .required(false),
                )
                .arg(arg!(--"microservice-calculator" "Start calculator API as microservice"))
                .arg(arg!(--"no-sleep" "Make bots to make requests constantly"))
                .arg(arg!(--"no-clean" "Do not remove created database files"))
                .arg(arg!(--"no-servers" "Do not start new server instances"))
                .arg(arg!(--"save-state" "Save and load state"))
                .arg(arg!(--"update-calculator" "Update calculator state continuously"))
                .arg(arg!(--"print-speed" "Print some speed information"))
                .arg(arg!(--"log-debug" "Enable debug logging for server instances"))
                .arg(arg!(--"early-quit" "First error quits"))
                .arg(
                    arg!(--"test" <NAME> "Select custom test")
                        .value_parser(value_parser!(Test))
                        .required(false)
                        .default_value(TEST_NAME_QA),
                )
                .arg(arg!(--forever "Run tests forever")),
        )
        .get_matches();

    let test_mode = match matches.subcommand() {
        Some(("test", sub_matches)) => {
            let api_urls = PublicApiUrls::new(
                sub_matches.get_one::<Url>("url-register").unwrap().clone(),
                sub_matches.get_one::<Url>("url-account").unwrap().clone(),
                sub_matches
                    .get_one::<Url>("url-calculator")
                    .unwrap()
                    .clone(),
            );

            Some(TestMode {
                bot_count: *sub_matches.get_one::<u32>("bots").unwrap(),
                task_count: *sub_matches.get_one::<u32>("tasks").unwrap(),
                forever: sub_matches.is_present("forever"),
                no_sleep: sub_matches.is_present("no-sleep"),
                no_clean: sub_matches.is_present("no-clean"),
                no_servers: sub_matches.is_present("no-servers"),
                update_calculator_state: sub_matches.is_present("update-calculator"),
                save_state: sub_matches.is_present("save-state"),
                print_speed: sub_matches.is_present("print-speed"),
                early_quit: sub_matches.is_present("early-quit"),
                test: sub_matches
                    .get_one::<Test>("test")
                    .map(ToOwned::to_owned)
                    .unwrap(),
                server: ServerConfig {
                    api_urls,
                    test_database_dir: sub_matches
                        .get_one::<PathBuf>("test-database")
                        .map(ToOwned::to_owned)
                        .unwrap(),
                    microservice_calculator: sub_matches.is_present("microservice-calculator"),
                    log_debug: sub_matches.is_present("log-debug"),
                },
            })
        }
        _ => None,
    };

    ArgsConfig {
        database_dir: matches
            .get_one::<PathBuf>("database")
            .map(ToOwned::to_owned),
        test_mode,
    }
}

#[derive(Debug, Clone)]
pub struct TestMode {
    pub bot_count: u32,
    pub task_count: u32,
    pub forever: bool,
    pub no_sleep: bool,
    pub no_clean: bool,
    pub no_servers: bool,
    pub save_state: bool,
    pub update_calculator_state: bool,
    pub print_speed: bool,
    pub early_quit: bool,
    pub test: Test,
    pub server: ServerConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub api_urls: PublicApiUrls,
    pub test_database_dir: PathBuf,
    pub microservice_calculator: bool,
    pub log_debug: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Test {
    Qa,
    BenchmarkGetCalculatorState,
    Bot,
}

const TEST_NAME_QA: &str = "qa";
const TEST_NAME_BENCHMARK_GET_CALCUALTOR_STATE: &str = "benchmark-get-calculator-state";
const TEST_NAME_BOT: &str = "bot";

impl Test {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Qa => TEST_NAME_QA,
            Self::BenchmarkGetCalculatorState => TEST_NAME_BENCHMARK_GET_CALCUALTOR_STATE,
            Self::Bot => TEST_NAME_BOT,
        }
    }
}

impl TryFrom<&str> for Test {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            TEST_NAME_QA => Self::Qa,
            TEST_NAME_BENCHMARK_GET_CALCUALTOR_STATE => Self::BenchmarkGetCalculatorState,
            TEST_NAME_BOT => Self::Bot,
            _ => return Err(()),
        })
    }
}

impl clap::builder::ValueParserFactory for Test {
    type Parser = TestNameParser;
    fn value_parser() -> Self::Parser {
        TestNameParser
    }
}

#[derive(Debug, Clone)]
pub struct TestNameParser;

impl clap::builder::TypedValueParser for TestNameParser {
    type Value = Test;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        value
            .to_str()
            .ok_or(clap::Error::raw(
                clap::ErrorKind::InvalidUtf8,
                "Text was not UTF-8.",
            ))?
            .try_into()
            .map_err(|_| clap::Error::raw(clap::ErrorKind::InvalidValue, "Unknown test"))
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = clap::PossibleValue<'static>> + '_>> {
        Some(Box::new(
            [Test::Qa, Test::BenchmarkGetCalculatorState, Test::Bot]
                .iter()
                .map(|value| PossibleValue::new(value.as_str())),
        ))
    }
}
