use std::{fmt::Debug, marker::PhantomData, net::SocketAddr};

use error_stack::Result;

use crate::{
    api::model::{
        Account, AccountIdInternal, AccountIdLight, AccountSetup, AuthPair, SignInWithInfo,
    },
    config::Config,
    server::database::DatabaseError,
    utils::{ConvertCommandError, ErrorConversion},
};

use super::{
    cache::{CacheError, DatabaseCache, WriteCacheJson},
    current::CurrentDataWriteCommands,
    sqlite::{CurrentDataWriteHandle, SqliteDatabaseError, SqliteUpdateJson},
};

pub struct NoId;

#[derive(Debug, Clone, Copy)]
pub enum DatabaseId {
    Light(AccountIdLight),
    Internal(AccountIdInternal),
    Empty,
}

impl From<AccountIdLight> for DatabaseId {
    fn from(value: AccountIdLight) -> Self {
        DatabaseId::Light(value)
    }
}

impl From<AccountIdInternal> for DatabaseId {
    fn from(value: AccountIdInternal) -> Self {
        DatabaseId::Internal(value)
    }
}

impl From<NoId> for DatabaseId {
    fn from(_: NoId) -> Self {
        DatabaseId::Empty
    }
}

pub type WriteResult<T, Err, WriteContext = T> =
    std::result::Result<T, WriteError<error_stack::Report<Err>, WriteContext>>;

#[derive(Debug)]
pub struct WriteError<Err, Target = ()> {
    pub e: Err,
    pub t: PhantomData<Target>,
}

impl<Target> From<error_stack::Report<SqliteDatabaseError>>
    for WriteError<error_stack::Report<SqliteDatabaseError>, Target>
{
    fn from(value: error_stack::Report<SqliteDatabaseError>) -> Self {
        Self {
            t: PhantomData,
            e: value,
        }
    }
}

impl<Target> From<error_stack::Report<CacheError>>
    for WriteError<error_stack::Report<CacheError>, Target>
{
    fn from(value: error_stack::Report<CacheError>) -> Self {
        Self {
            t: PhantomData,
            e: value,
        }
    }
}

impl<Target> From<CacheError> for WriteError<error_stack::Report<CacheError>, Target> {
    fn from(value: CacheError) -> Self {
        Self {
            t: PhantomData,
            e: value.into(),
        }
    }
}

// TODO: If one commands does multiple writes to database, move writes to happen
// in a transaction.

/// One Account can do only one write command at a time.
pub struct AccountWriteLock;

/// Globally synchronous write commands.
pub struct WriteCommands<'a> {
    current_write: &'a CurrentDataWriteHandle,
    cache: &'a DatabaseCache,
}

impl<'a> WriteCommands<'a> {
    pub fn new(current_write: &'a CurrentDataWriteHandle, cache: &'a DatabaseCache) -> Self {
        Self {
            current_write,
            cache,
        }
    }

    pub async fn register(
        id_light: AccountIdLight,
        sign_in_with_info: SignInWithInfo,
        config: &Config,
        current_data_write: CurrentDataWriteHandle,
        cache: &DatabaseCache,
    ) -> Result<AccountIdInternal, DatabaseError> {
        let current = CurrentDataWriteCommands::new(&current_data_write);
        let account_commands = current.clone().account();

        let account = Account::default();
        let account_setup = AccountSetup::default();

        // TODO: Use transactions here.

        let id = account_commands
            .store_account_id(id_light)
            .await
            .convert(id_light)?;

        cache.insert_account_if_not_exists(id).await.convert(id)?;

        account_commands.store_api_key(id, None).await.convert(id)?;
        account_commands
            .store_refresh_token(id, None)
            .await
            .convert(id)?;

        if config.components().account {
            account_commands
                .store_account(id, &account)
                .await
                .convert(id)?;

            cache
                .write_cache(id.as_light(), |cache| {
                    cache.account = Some(account.clone().into());
                    Ok(())
                })
                .await
                .convert(id)?;

            account_commands
                .store_account_setup(id, &account_setup)
                .await
                .convert(id)?;

            account_commands
                .store_sign_in_with_info(id, &sign_in_with_info)
                .await
                .convert(id)?;
        }
        if config.components().calculator {
            let _ = current
                .calculator()
                .init_calculator_state(id)
                .await
                .convert(id)?;
        }

        Ok(id)
    }

    pub async fn set_new_auth_pair(
        &self,
        id: AccountIdInternal,
        pair: AuthPair,
        address: Option<SocketAddr>,
    ) -> Result<(), DatabaseError> {
        let current_access_token = self
            .current_write
            .read()
            .account()
            .access_token(id)
            .await
            .convert(id)?;

        self.current()
            .account()
            .update_api_key(id, Some(&pair.access))
            .await
            .convert(id)?;

        self.current()
            .account()
            .update_refresh_token(id, Some(&pair.refresh))
            .await
            .convert(id)?;

        self.cache
            .update_access_token_and_connection(
                id.as_light(),
                current_access_token,
                pair.access,
                address,
            )
            .await
            .convert(id)
    }

    /// Remove current connection address, access and refresh tokens.
    pub async fn logout(&self, id: AccountIdInternal) -> Result<(), DatabaseError> {
        self.current()
            .account()
            .update_refresh_token(id, None)
            .await
            .convert(id)?;

        self.end_connection_session(id, true).await?;

        Ok(())
    }

    /// Remove current connection address and access token.
    pub async fn end_connection_session(
        &self,
        id: AccountIdInternal,
        remove_access_token: bool,
    ) -> Result<(), DatabaseError> {
        let current_access_token = if remove_access_token {
            self.current_write
                .read()
                .account()
                .access_token(id)
                .await
                .convert(id)?
        } else {
            None
        };

        self.cache
            .delete_access_token_and_connection(id.as_light(), current_access_token)
            .await
            .convert(id)?;

        self.current()
            .account()
            .update_api_key(id, None)
            .await
            .convert(id)?;

        Ok(())
    }

    pub async fn update_data<
        T: Clone + Debug + Send + SqliteUpdateJson + WriteCacheJson + Sync + 'static,
    >(
        &mut self,
        id: AccountIdInternal,
        data: &T,
    ) -> Result<(), DatabaseError> {
        data.update_json(id, &self.current())
            .await
            .with_info_lazy(|| format!("Update {:?} failed, id: {:?}", PhantomData::<T>, id))?;

        // Empty implementation if not really cacheable.
        data.write_to_cache(id.as_light(), &self.cache)
            .await
            .with_info_lazy(|| format!("Cache update {:?} failed, id: {:?}", PhantomData::<T>, id))
    }

    fn current(&self) -> CurrentDataWriteCommands {
        CurrentDataWriteCommands::new(&self.current_write)
    }
}

/// Commands that can run concurrently with other write commands, but which have
/// limitation that one account can execute only one command at a time.
/// It possible to run this and normal write command concurrently for
/// one account.
pub struct WriteCommandsAccount<'a> {
    current_write: &'a CurrentDataWriteHandle,
    cache: &'a DatabaseCache,
}

impl<'a> WriteCommandsAccount<'a> {
    pub fn new(current_write: &'a CurrentDataWriteHandle, cache: &'a DatabaseCache) -> Self {
        Self {
            current_write,
            cache,
        }
    }

    fn current(&self) -> CurrentDataWriteCommands {
        CurrentDataWriteCommands::new(&self.current_write)
    }
}
