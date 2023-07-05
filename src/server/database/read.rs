use std::{fmt::Debug, marker::PhantomData};

use tokio_stream::StreamExt;

use crate::{
    api::model::{AccountIdInternal, AccountIdLight, ApiKey, RefreshToken},
    utils::{ConvertCommandError, ErrorConversion},
};

use super::{
    cache::{CacheError, DatabaseCache, ReadCacheJson},
    current::SqliteReadCommands,
    sqlite::{SqliteDatabaseError, SqliteReadHandle, SqliteSelectJson},
    write::NoId,
    DatabaseError,
};

use error_stack::Result;

pub type ReadResult<T, Err, WriteContext = T> =
    std::result::Result<T, ReadError<error_stack::Report<Err>, WriteContext>>;

#[derive(Debug)]
pub struct ReadError<Err, Target = ()> {
    pub e: Err,
    pub t: PhantomData<Target>,
}

impl<Target> From<error_stack::Report<SqliteDatabaseError>>
    for ReadError<error_stack::Report<SqliteDatabaseError>, Target>
{
    fn from(value: error_stack::Report<SqliteDatabaseError>) -> Self {
        Self {
            t: PhantomData,
            e: value,
        }
    }
}

impl<Target> From<error_stack::Report<CacheError>>
    for ReadError<error_stack::Report<CacheError>, Target>
{
    fn from(value: error_stack::Report<CacheError>) -> Self {
        Self {
            t: PhantomData,
            e: value,
        }
    }
}

impl<Target> From<SqliteDatabaseError>
    for ReadError<error_stack::Report<SqliteDatabaseError>, Target>
{
    fn from(value: SqliteDatabaseError) -> Self {
        Self {
            t: PhantomData,
            e: value.into(),
        }
    }
}

impl<Target> From<CacheError> for ReadError<error_stack::Report<CacheError>, Target> {
    fn from(value: CacheError) -> Self {
        Self {
            t: PhantomData,
            e: value.into(),
        }
    }
}

pub struct ReadCommands<'a> {
    sqlite: SqliteReadCommands<'a>,
    cache: &'a DatabaseCache,
}

impl<'a> ReadCommands<'a> {
    pub fn new(sqlite: &'a SqliteReadHandle, cache: &'a DatabaseCache) -> Self {
        Self {
            sqlite: SqliteReadCommands::new(sqlite),
            cache,
        }
    }

    pub async fn account_access_token(
        &self,
        id: AccountIdLight,
    ) -> Result<Option<ApiKey>, DatabaseError> {
        let id = self.cache.to_account_id_internal(id).await.convert(id)?;
        self.sqlite.account().access_token(id).await.convert(id)
    }

    pub async fn account_refresh_token(
        &self,
        id: AccountIdInternal,
    ) -> Result<Option<RefreshToken>, DatabaseError> {
        self.sqlite.account().refresh_token(id).await.convert(id)
    }

    pub async fn account_ids<T: FnMut(AccountIdInternal)>(
        &self,
        mut handler: T,
    ) -> Result<(), DatabaseError> {
        let account = self.sqlite.account();
        let mut users = account.account_ids_stream();
        while let Some(user_id) = users.try_next().await.convert(NoId)? {
            handler(user_id)
        }

        Ok(())
    }

    pub async fn read_json<T: SqliteSelectJson + Debug + ReadCacheJson + Send + Sync + 'static>(
        &self,
        id: AccountIdInternal,
    ) -> Result<T, DatabaseError> {
        if T::CACHED_JSON {
            T::read_from_cache(id.as_light(), self.cache)
                .await
                .with_info_lazy(|| {
                    format!("Cache read {:?} failed, id: {:?}", PhantomData::<T>, id)
                })
        } else {
            T::select_json(id, &self.sqlite)
                .await
                .with_info_lazy(|| format!("Read {:?} failed, id: {:?}", PhantomData::<T>, id))
        }
    }
}
