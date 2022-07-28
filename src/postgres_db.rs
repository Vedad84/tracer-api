use byte_slice_cast::AsByteSlice;
use itertools::Itertools;
use {
    log::*,
    openssl::ssl::{SslConnector, SslFiletype, SslMethod},
    tokio_postgres::{ connect, Client },
    postgres::{ NoTls },
    postgres_openssl::MakeTlsConnector,
    solana_sdk::{
        account::Account,
        pubkey::Pubkey,
    },
    std::error,
    thiserror::Error,
    tokio::task::block_in_place,
};

type Slot = i64;
type DbResult<T> = std::result::Result<T, anyhow::Error>;

const DEFAULT_POSTGRES_PORT: u16 = 5432;

pub struct DbClient {
    client: Client,
}

pub struct DbClientConfig {
    /// The host name or IP of the PostgreSQL server
    pub host: Option<String>,

    /// The user name of the PostgreSQL server.
    pub user: Option<String>,

    /// The port number of the PostgreSQL database, the default is 5432
    pub port: Option<u16>,

    /// The connection string of PostgreSQL database, if this is set
    /// `host`, `user` and `port` will be ignored.
    pub connection_str: Option<String>,
}

#[derive(Error, Debug)]
pub enum PostgresDbError {
    #[error("Error preparing data store schema. Error message: ({msg})")]
    ConfigurationError { msg: String },
}

impl DbClient {
    pub async fn new(config: DbClientConfig) -> Self {
        Self {
            client: Self::connect_to_db(config).await.unwrap(),
        }
    }

    async fn connect_to_db(config: DbClientConfig) -> Result<Client, PostgresDbError>  {
        let port = config.port.unwrap_or(DEFAULT_POSTGRES_PORT);

        let connection_str = if let Some(connection_str) = &config.connection_str {
            connection_str.clone()
        } else {
            if config.host.is_none() || config.user.is_none() {
                let msg = format!(
                    "\"connection_str\": {:?}, or \"host\": {:?} \"user\": {:?} must be specified",
                    config.connection_str, config.host, config.user
                );
                return Err(PostgresDbError::ConfigurationError { msg });
            }
            format!(
                "host={} user={} port={}",
                config.host.as_ref().unwrap(),
                config.user.as_ref().unwrap(),
                port
            )
        };

        let result =
            connect(&connection_str, postgres::NoTls).await;

        match result {
            Err(err) => {
                let msg = format!(
                    "Error in connecting to the PostgreSQL database: {:?} connection_str: {:?}",
                    err, connection_str
                );
                error!("{}", msg);
                Err(PostgresDbError::ConfigurationError { msg })
            }
            Ok((client, connection)) => {
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("connection error: {}", e);
                    }
                });
                Ok(client)
            },
        }
    }

    fn block<F, Fu, R>(&self, f: F) -> R
        where
            F: FnOnce(&Client) -> Fu,
            Fu: std::future::Future<Output = R>,
    {
        block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(f(&self.client))
        })
    }

    pub fn get_slot(&self) -> Result<Slot, anyhow::Error> {
        let slot = self.block(|client| async {
            self.client.query_one("SELECT MAX(slot) FROM public.slot", &[])
                .await?
                .try_get(0)
        })?;

        Ok(slot)
    }

    pub fn get_block_time(&self, slot: Slot) -> Result<i64, anyhow::Error> {
        let time = self.block(|client| async {
            self.client.query_one(
                "SELECT block_time FROM public.block WHERE slot = $1",
                &[&slot],
            ).await?.try_get(0)
        })?;

        Ok(time)
    }

    pub fn get_accounts_at_slot(
        &self,
        pubkeys: impl Iterator<Item = Pubkey>,
        slot: Slot,
    ) -> DbResult<Vec<(Pubkey, Account)>> {
        // SELECT * FROM get_accounts_at_slot(ARRAY[decode('5991510ef1cc9da133f4dd51e34ef00318ab4dfa517a4fd00baef9e83f7a7751', 'hex')], 10000000)
        let pubkey_bytes = pubkeys
            .map(|entry| entry.to_bytes())
            .collect_vec();

        let pubkey_slices = pubkey_bytes
            .iter()
            .map(|entry| entry.as_byte_slice())
            .collect_vec();
        let mut result = Vec::new();

        let rows = self.block(|client| async {
            self.client.query(
                "SELECT * FROM get_accounts_at_slot($1, $2)",
                &[&pubkey_slices, &slot]
            ).await
        })?;

        for row in rows {
            let lamports: i64 = row.try_get(2)?;
            let rent_epoch: i64 = row.try_get(4)?;
            result.push((
                Pubkey::new(row.try_get(0)?),
                Account {
                    lamports: lamports as u64,
                    data: row.try_get(5)?,
                    owner: Pubkey::new(row.try_get(1)?),
                    executable: row.try_get(3)?,
                    rent_epoch: rent_epoch as u64,
                }
            ));
        }

        Ok(result)
    }

    pub fn get_account_at_slot(&self, pubkey: &Pubkey, slot: Slot) -> DbResult<Option<Account>> {
        let accounts = self.get_accounts_at_slot(std::iter::once(pubkey.to_owned()), slot)?;
        let account = accounts.get(0).map(|(_, account)| account).cloned();
        Ok(account)
    }
}
