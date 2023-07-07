pub mod app;
pub mod database;
pub mod internal;

use std::{net::SocketAddr, pin::Pin, sync::Arc};

use axum::Router;
use futures::future::poll_fn;
use hyper::server::{
    accept::Accept,
    conn::{AddrIncoming, Http},
};
use tokio::{
    net::TcpListener,
    signal,
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use tower::MakeService;
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    api::ApiDoc,
    config::Config,
    server::{
        app::{connection::WebSocketManager, App},
        database::DatabaseManager,
        internal::InternalApp,
    },
};

use self::app::connection::ServerQuitWatcher;

pub struct CalculatorServer {
    config: Arc<Config>,
}

impl CalculatorServer {
    pub fn new(config: Config) -> Self {
        Self {
            config: config.into(),
        }
    }

    pub async fn run(self) {
        tracing_subscriber::fmt::init();

        let (database_manager, router_database_handle) = DatabaseManager::new(
            self.config.database_dir().to_path_buf(),
            self.config.clone(),
        )
        .await
        .expect("Database init failed");

        let (server_quit_handle, server_quit_watcher) = broadcast::channel(1);
        let (ws_manager, mut ws_quit_ready) =
            WebSocketManager::new(server_quit_watcher.resubscribe());

        let mut app = App::new(router_database_handle, self.config.clone(), ws_manager).await;

        let server_task = self
            .create_public_api_server_task(&mut app, server_quit_watcher.resubscribe())
            .await;
        let internal_server_task = if self.config.debug_mode() {
            None
        } else {
            Some(
                self.create_internal_api_server_task(&app, server_quit_watcher.resubscribe())
                    .await,
            )
        };

        match signal::ctrl_c().await {
            Ok(()) => (),
            Err(e) => error!("Failed to listen CTRL+C. Error: {}", e),
        }

        info!("Server quit started");

        drop(server_quit_handle);

        // Wait until all tasks quit
        server_task
            .await
            .expect("Public API server task panic detected");
        if let Some(handle) = internal_server_task {
            handle
                .await
                .expect("Internal API server task panic detected");
        }

        loop {
            match ws_quit_ready.recv().await {
                Some(()) => (),
                None => break,
            }
        }

        drop(app);
        database_manager.close().await;

        info!("Server quit done");
    }

    /// Public API. This can have WAN access.
    pub async fn create_public_api_server_task(
        &self,
        app: &mut App,
        quit_notification: ServerQuitWatcher,
    ) -> JoinHandle<()> {
        let router = {
            let router = self.create_public_router(app);
            let router = if self.config.debug_mode() {
                router
                    .merge(Self::create_swagger_ui())
                    .merge(self.create_internal_router(&app))
            } else {
                router
            };
            let router = if self.config.debug_mode() {
                router.route_layer(TraceLayer::new_for_http())
            } else {
                router
            };
            router
        };

        let addr = self.config.socket().public_api;
        info!("Public API is available on {}", addr);
        if self.config.debug_mode() {
            info!("Internal API is available on {}", addr);
        }

        if let Some(tls_config) = self.config.public_api_tls_config() {
            self.create_server_task_with_tls(addr, router, tls_config.clone(), quit_notification)
                .await
        } else {
            self.create_server_task_no_tls(router, addr, "Public API")
        }
    }

    pub async fn create_server_task_with_tls(
        &self,
        addr: SocketAddr,
        router: Router,
        tls_config: Arc<ServerConfig>,
        mut quit_notification: ServerQuitWatcher,
    ) -> JoinHandle<()> {
        let listener = TcpListener::bind(addr)
            .await
            .expect("Address not available");
        let mut listener =
            AddrIncoming::from_listener(listener).expect("AddrIncoming creation failed");
        listener.set_sleep_on_errors(true);

        let protocol = Arc::new(Http::new());
        let acceptor = TlsAcceptor::from(tls_config);

        let mut app_service = router.into_make_service_with_connect_info::<SocketAddr>();

        tokio::spawn(async move {
            let (drop_after_connection, mut wait_all_connections) = mpsc::channel::<()>(1);

            loop {
                let next_addr_stream = poll_fn(|cx| Pin::new(&mut listener).poll_accept(cx));

                let stream = tokio::select! {
                    _ = quit_notification.recv() => {
                        break;
                    }
                    addr = next_addr_stream => {
                        match addr {
                            None => {
                                error!("Socket closed");
                                break;
                            }
                            Some(Err(e)) => {
                                error!("Address stream error {e}");
                                continue;
                            }
                            Some(Ok(stream)) => {
                                stream
                            }
                        }
                    }
                };

                let acceptor = acceptor.clone();
                let protocol = protocol.clone();
                let service = app_service.make_service(&stream);

                let mut quit_notification = quit_notification.resubscribe();
                let drop_on_quit = drop_after_connection.clone();
                tokio::spawn(async move {
                    tokio::select! {
                        _ = quit_notification.recv() => {} // Graceful shutdown for connections?
                        connection = acceptor.accept(stream) => {
                            match connection {
                                Ok(connection) => {
                                    if let Ok(service) = service.await {
                                        let _ = protocol.serve_connection(connection, service).with_upgrades().await;
                                    }
                                }
                                Err(_) => {},
                            }
                        }
                    }

                    drop(drop_on_quit);
                });
            }
            drop(drop_after_connection);
            drop(quit_notification);

            loop {
                match wait_all_connections.recv().await {
                    Some(()) => (),
                    None => break,
                }
            }
        })
    }

    pub fn create_server_task_no_tls(
        &self,
        router: Router,
        addr: SocketAddr,
        name_for_log_message: &'static str,
    ) -> JoinHandle<()> {
        let normal_api_server = {
            axum::Server::bind(&addr)
                .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        };

        tokio::spawn(async move {
            let shutdown_handle = normal_api_server.with_graceful_shutdown(async {
                match signal::ctrl_c().await {
                    Ok(()) => (),
                    Err(e) => error!("Failed to listen CTRL+C. Error: {}", e),
                }
            });

            match shutdown_handle.await {
                Ok(()) => {
                    info!("{name_for_log_message} server future returned Ok()");
                }
                Err(e) => {
                    error!("{name_for_log_message} server future returned error: {}", e);
                }
            }
        })
    }

    // Internal server to server API. This must be only LAN accessible.
    pub async fn create_internal_api_server_task(
        &self,
        app: &App,
        quit_notification: ServerQuitWatcher,
    ) -> JoinHandle<()> {
        let router = self.create_internal_router(&app);
        let router = if self.config.debug_mode() {
            router.merge(Self::create_swagger_ui())
        } else {
            router
        };

        let addr = self.config.socket().internal_api;
        info!("Internal API is available on {}", addr);
        if let Some(tls_config) = self.config.internal_api_tls_config() {
            self.create_server_task_with_tls(addr, router, tls_config.clone(), quit_notification)
                .await
        } else {
            self.create_server_task_no_tls(router, addr, "Internal API")
        }
    }

    pub fn create_public_router(&self, app: &mut App) -> Router {
        let mut router = app.create_common_server_router();

        if self.config.components().account {
            router = router.merge(app.create_account_server_router())
        }

        if self.config.components().calculator {
            router = router.merge(app.create_calculator_server_router())
        }

        router
    }

    pub fn create_internal_router(&self, app: &App) -> Router {
        let mut router = Router::new();
        if self.config.components().account {
            router = router.merge(InternalApp::create_account_server_router(app.state()))
        }

        router
    }

    pub fn create_swagger_ui() -> SwaggerUi {
        SwaggerUi::new("/swagger-ui").url("/api-doc/calculator_api.json", ApiDoc::openapi())
    }
}
