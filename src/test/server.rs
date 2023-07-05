use std::{env, net::SocketAddrV4, os::unix::process::CommandExt, path::PathBuf, sync::Arc};

use crate::config::{
    args::TestMode,
    file::{Components, ConfigFile, ExternalServices, SocketConfig, CONFIG_FILE_NAME},
};

use nix::{sys::signal::Signal, unistd::Pid};
use reqwest::Url;
use tokio::process::Child;
use tracing::info;

pub const SERVER_INSTANCE_DIR_START: &str = "server_instance_";

pub struct ServerManager {
    servers: Vec<ServerInstance>,
    config: Arc<TestMode>,
}

impl ServerManager {
    pub async fn new(config: Arc<TestMode>) -> Self {
        let dir = config.server.test_database_dir.clone();
        if !dir.exists() {
            std::fs::create_dir_all(&dir).unwrap();
        }

        info!("data dir: {:?}", dir);

        let check_host = |url: &Url, name| {
            let host = url.host_str().unwrap();
            if !(host == "127.0.0.1" || host == "localhost") {
                panic!("{} address was not 127.0.0.1. value: {}", name, host);
            }
        };
        check_host(&config.server.api_urls.account_base_url, "account server");
        check_host(
            &config.server.api_urls.calculator_base_url,
            "calculator server",
        );

        let account_port = config.server.api_urls.account_base_url.port().unwrap();
        let calculator_port = config.server.api_urls.calculator_base_url.port().unwrap();

        let external_services = Some(ExternalServices {
            account_internal: format!("http://127.0.0.1:{}", account_port + 1)
                .parse::<Url>()
                .unwrap()
                .into(),
        });

        let localhost_ip = "127.0.0.1".parse().unwrap();

        let account_config = new_config(
            &config,
            SocketAddrV4::new(localhost_ip, account_port),
            SocketAddrV4::new(localhost_ip, account_port + 1),
            Components {
                account: true,
                calculator: !config.server.microservice_calculator,
            },
            external_services.clone(),
        );
        let mut servers = vec![ServerInstance::new(dir.clone(), account_config, &config)];

        if config.server.microservice_calculator {
            let server_config = new_config(
                &config,
                SocketAddrV4::new(localhost_ip, calculator_port),
                SocketAddrV4::new(localhost_ip, calculator_port + 1),
                Components {
                    calculator: true,
                    ..Components::default()
                },
                external_services,
            );
            servers.push(ServerInstance::new(dir.clone(), server_config, &config));
        }

        Self { servers, config }
    }

    pub async fn close(self) {
        for s in self.servers {
            s.close_and_maeby_remove_data(!self.config.no_clean).await;
        }
    }
}

fn new_config(
    _config: &TestMode,
    public_api: SocketAddrV4,
    internal_api: SocketAddrV4,
    components: Components,
    external_services: Option<ExternalServices>,
) -> ConfigFile {
    ConfigFile {
        debug: Some(true),
        components,
        database: crate::config::file::DatabaseConfig {
            dir: "database_dir".into(),
        },
        socket: SocketConfig {
            public_api: public_api.into(),
            internal_api: internal_api.into(),
        },
        external_services,
        sign_in_with_google: None,
        tls: None,
    }
}

pub struct ServerInstance {
    server: Child,
    dir: PathBuf,
}

impl ServerInstance {
    pub fn new(dir: PathBuf, config: ConfigFile, args_config: &TestMode) -> Self {
        let id = uuid::Uuid::new_v4();
        let dir = dir.join(format!(
            "{}{}_{}",
            SERVER_INSTANCE_DIR_START,
            time::OffsetDateTime::now_utc(),
            id.hyphenated()
        ));
        std::fs::create_dir(&dir).unwrap();

        let config = toml::to_string_pretty(&config).unwrap();
        std::fs::write(dir.join(CONFIG_FILE_NAME), config).unwrap();

        let start_cmd = env::args().next().unwrap();
        let start_cmd = std::fs::canonicalize(&start_cmd).unwrap();

        if !start_cmd.is_file() {
            panic!("First argument does not point to a file {:?}", &start_cmd);
        }

        info!("start_cmd: {:?}", &start_cmd);

        let log_value = if args_config.server.log_debug {
            "debug"
        } else {
            "warn"
        };

        let mut command = std::process::Command::new(start_cmd);
        command
            .current_dir(&dir)
            .env("RUST_LOG", log_value)
            .process_group(0);

        let mut tokio_command: tokio::process::Command = command.into();
        let server = tokio_command.kill_on_drop(true).spawn().unwrap();

        Self { server, dir }
    }

    fn running(&mut self) -> bool {
        self.server.try_wait().unwrap().is_none()
    }

    async fn close_and_maeby_remove_data(mut self, remove: bool) {
        let id = self.server.id().unwrap();
        nix::sys::signal::kill(Pid::from_raw(id.try_into().unwrap()), Signal::SIGINT).unwrap(); // CTRL-C
        self.server.wait().await.unwrap();

        if remove {
            let dir = self.dir.file_name().unwrap().to_string_lossy();
            if dir.starts_with(SERVER_INSTANCE_DIR_START) {
                std::fs::remove_dir_all(self.dir).unwrap();
            } else {
                panic!("Not database instance dir {}", dir);
            }
        }
    }
}
