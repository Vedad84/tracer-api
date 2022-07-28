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
    #[error("Error connecting to the backend data store. Error message: ({msg})")]
    DataStoreConnectionError { msg: String },

    #[error("Error preparing data store schema. Error message: ({msg})")]
    DataSchemaError { msg: String },

    #[error("Error preparing data store schema. Error message: ({msg})")]
    ConfigurationError { msg: String },

    #[error("Replica account V0.0.1 not supported anymore")]
    ReplicaAccountV001NotSupported,

    #[error("Failed to parse account key from transaction message")]
    AccountKeyParseError,

    #[error("Failed to update transaction-account linkage: ({msg})")]
    TransactionAccountUpdateError { msg: String },
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

    pub async fn get_slot(&mut self) -> Result<Slot, anyhow::Error> {
        let slot = self.client.query_one("SELECT MAX(slot) FROM public.slot", &[])
            .await?
            .try_get(0)?;
        Ok(slot)
    }


    pub async fn get_block_time(&self, slot: Slot) -> Result<i64, anyhow::Error> {
        let time = self.client.query_one(
            "SELECT block_time FROM public.block WHERE slot = $1",
            &[&slot],
        ).await?.try_get(0)?;

        Ok(time)
    }

    /*
    pub fn get_accounts_at_slot(
        &self,
        pubkeys: impl Iterator<Item = Pubkey>,
        slot: Slot,
    ) -> DbResult<Vec<(Pubkey, Account)>> {
        // SELECT * FROM get_accounts_at_slot(ARRAY[decode('5991510ef1cc9da133f4dd51e34ef00318ab4dfa517a4fd00baef9e83f7a7751', 'hex')], 10000000)
    }

    pub fn get_account_at_slot(&self, pubkey: &Pubkey, slot: Slot) -> DbResult<Option<Account>> {

    }*/
}
