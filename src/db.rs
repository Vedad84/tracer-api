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

pub struct DbClient {
    client: Client,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("account not found: {acc}")]
    AccNotFound{ acc: Pubkey },

    #[error("postgres: {}", .0)]
    Db(#[from] tokio_postgres::Error),
}

type DbResult<T> = std::result::Result<T, Error>;

impl DbClient {
    pub async fn new(
        host: &str,
        port: &str,
        user: Option<String>,
        password: Option<String>,
        database: Option<String>,
    ) -> Self {
        let connection_str= format!("host={} port={} dbname={} user={} password={}",
                                    host, port,
                                    database.unwrap_or_default(),
                                    user.unwrap_or_default(),
                                    password.unwrap_or_default());


        let (client, connection) =
            connect(&connection_str, postgres::NoTls).await.unwrap();

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Self {
            client
        }
    }

    fn block<F, Fu, R>(&self, f: F) -> R
        where
            F: FnOnce() -> Fu,
            Fu: std::future::Future<Output = R>,
    {
        block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(f())
        })
    }

    pub fn get_slot(&self) -> Result<u64, Error> {
        let slot: i64 = self.block(|| async {
            self.client.query_one("SELECT MAX(slot) FROM public.slot", &[])
                .await
        })?.try_get(0)?;

        Ok(slot as u64)
    }

    pub fn get_block_time(&self, slot: u64) -> Result<i64, Error> {
        let time = self.block(|| async {
            self.client.query_one(
                "SELECT block_time FROM public.block WHERE slot = $1",
                &[&(slot as i64)],
            ).await
        })?.try_get(0)?;

        Ok(time)
    }

    pub fn get_account_at_slot(&self, pubkey: &Pubkey, slot: u64) -> DbResult<Option<Account>> {
        let pubkey_bytes = pubkey.to_bytes();
        let rows = self.block(|| async {
            self.client.query(
                "SELECT * FROM get_account_at_slot($1, $2)",
                &[&pubkey_bytes.as_slice(), &(slot as i64)]
            ).await
        })?;

        if rows.len() != 1 {
            return Ok(None);
        }

        let row = &rows[0];
        let lamports: i64 = row.try_get(2)?;
        let rent_epoch: i64 = row.try_get(4)?;
        Ok(Some(Account {
                lamports: lamports as u64,
                data: row.try_get(5)?,
                owner: Pubkey::new(row.try_get(1)?),
                executable: row.try_get(3)?,
                rent_epoch: rent_epoch as u64,
        }))
    }
}
