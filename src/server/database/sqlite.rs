use crate::api::model::AccountIdInternal;

use async_trait::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use tracing::log::info;

use super::current::{CurrentDataWriteCommands, SqliteReadCommands};

use error_stack::Result;

use std::path::{Path, PathBuf};

use sqlx::{
    sqlite::{self, SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

use crate::utils::IntoReportExt;

pub const DATABASE_FILE_NAME: &str = "current.db";

#[derive(thiserror::Error, Debug)]
pub enum SqliteDatabaseError {
    #[error("Connecting to SQLite database failed")]
    Connect,
    #[error("Executing SQL query failed")]
    Execute,
    #[error("Error when streaming data from SQL query")]
    Fetch,
    #[error("Running sqlx database migrations failed")]
    Migrate,
    #[error("Starting transaction failed")]
    TransactionBegin,
    #[error("Rollbacking transaction failed")]
    TransactionRollback,
    #[error("Commiting transaction failed")]
    TransactionCommit,

    #[error("Deserialization error")]
    SerdeDeserialize,
    #[error("Serialization error")]
    SerdeSerialize,

    #[error("Time parsing error")]
    TimeParsing,

    #[error("TryFrom error")]
    TryFromError,
    #[error("Data format conversion error")]
    DataFormatConversion,

    #[error("Content slot not empty")]
    ContentSlotNotEmpty,
    #[error("Content slot is empty")]
    ContentSlotEmpty,
    #[error("ModerationRequestContentIsInvalid")]
    ModerationRequestContentInvalid,
}

/// Path to directory which contains Sqlite files.
#[derive(Debug, Clone)]
pub struct SqliteDatabasePath {
    database_dir: PathBuf,
}

impl SqliteDatabasePath {
    pub fn new(database_dir: PathBuf) -> Self {
        Self { database_dir }
    }

    pub fn path(&self) -> &Path {
        &self.database_dir
    }
}

pub struct SqliteWriteCloseHandle {
    pool: SqlitePool,
}

impl SqliteWriteCloseHandle {
    /// Call this before closing the server.
    pub async fn close(self) {
        self.pool.close().await
    }
}

#[derive(Debug, Clone)]
pub struct CurrentDataWriteHandle {
    handle: SqliteWriteHandle,
    read_handle: SqliteReadHandle,
}

impl CurrentDataWriteHandle {
    pub fn new(handle: SqliteWriteHandle) -> Self {
        Self {
            read_handle: SqliteReadHandle {
                pool: handle.pool.clone(),
            },
            handle,
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        self.handle.pool()
    }

    pub fn read(&self) -> SqliteReadCommands<'_> {
        SqliteReadCommands::new(&self.read_handle)
    }
}

#[derive(Debug, Clone)]
pub struct SqliteWriteHandle {
    pool: SqlitePool,
}

impl SqliteWriteHandle {
    pub async fn new(
        dir: SqliteDatabasePath,
        db_type: DatabaseType,
    ) -> Result<(Self, SqliteWriteCloseHandle), SqliteDatabaseError> {
        let db_path = dir.path().join(db_type.to_file_name());

        let run_initial_setup = !db_path.exists();

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(db_path)
                    .create_if_missing(true)
                    .foreign_keys(true)
                    .journal_mode(sqlite::SqliteJournalMode::Wal),
            )
            .await
            .into_error(SqliteDatabaseError::Connect)?;

        if run_initial_setup {
            sqlx::migrate!()
                .run(&pool)
                .await
                .into_error(SqliteDatabaseError::Migrate)?;
        }

        let write_handle = SqliteWriteHandle { pool: pool.clone() };

        let close_handle = SqliteWriteCloseHandle { pool };

        Ok((write_handle, close_handle))
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

pub struct SqliteReadCloseHandle {
    pool: SqlitePool,
}

impl SqliteReadCloseHandle {
    /// Call this before closing the server.
    pub async fn close(self) {
        self.pool.close().await
    }
}

#[derive(Debug, Clone)]
pub struct SqliteReadHandle {
    pool: SqlitePool,
}

impl SqliteReadHandle {
    pub async fn new(
        dir: SqliteDatabasePath,
        db_type: DatabaseType,
    ) -> Result<(Self, SqliteReadCloseHandle), SqliteDatabaseError> {
        let db_path = dir.path().join(db_type.to_file_name());

        let pool = SqlitePoolOptions::new()
            .max_connections(16)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(db_path)
                    .create_if_missing(false)
                    .foreign_keys(true)
                    .journal_mode(sqlite::SqliteJournalMode::Wal),
            )
            .await
            .into_error(SqliteDatabaseError::Connect)?;

        let handle = SqliteReadHandle { pool: pool.clone() };

        let close_handle = SqliteReadCloseHandle { pool };

        Ok((handle, close_handle))
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[derive(Debug, Clone)]
pub enum DatabaseType {
    Current,
}

impl DatabaseType {
    pub fn to_file_name(&self) -> &str {
        match self {
            DatabaseType::Current => DATABASE_FILE_NAME,
        }
    }
}

#[async_trait]
pub trait SqliteUpdateJson {
    async fn update_json(
        &self,
        id: AccountIdInternal,
        write: &CurrentDataWriteCommands,
    ) -> Result<(), SqliteDatabaseError>;
}

#[async_trait]
pub trait SqliteSelectJson: Sized {
    async fn select_json(
        id: AccountIdInternal,
        read: &SqliteReadCommands,
    ) -> Result<Self, SqliteDatabaseError>;
}

pub async fn print_sqlite_version(pool: &SqlitePool) -> Result<(), SqliteDatabaseError> {
    let q = sqlx::query("SELECT sqlite_version()")
        .map(|x: SqliteRow| {
            let r: String = x.get(0);
            r
        })
        .fetch_one(pool)
        .await
        .into_error(SqliteDatabaseError::Execute)?;

    info!("SQLite version: {}", q);
    Ok(())
}
