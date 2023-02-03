use {
    log::{error},
    tokio_postgres::{ connect, Client },
    parity_bytes::ToPretty,
    serde::{ Serialize, Deserialize },
    solana_sdk::{
        account::Account,
        pubkey::Pubkey,
    },
    std::{ fs::File, io::self, path::Path },
    thiserror::Error,
    tokio::task::block_in_place,
};

pub struct DbClient {
    client: Client,
    pub transaction_logs_column_list: Vec<&'static str>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("postgres: {}", .0)]
    Db(#[from] tokio_postgres::Error),

    #[error("Failed to get recent update for account {account} before slot {slot}: {err}")]
    GetRecentUpdateSlotErr{ account: String, slot: u64, err: tokio_postgres::Error },
}

type DbResult<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DBConfig{
    pub tracer_host: String,
    pub tracer_port: String,
    pub tracer_database: String,
    pub tracer_user: String,
    pub tracer_password: String,
    pub indexer_host: String,
    pub indexer_port: String,
    pub indexer_database: String,
    pub indexer_user: String,
    pub indexer_password: String,
}

pub fn load_config_file<T, P>(config_file: P) -> Result<T, io::Error>
    where
        T: serde::de::DeserializeOwned,
        P: AsRef<Path>,
{
    let file = File::open(config_file)?;
    let config = serde_yaml::from_reader(file)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{:?}", err)))?;
    Ok(config)
}

impl DbClient {
    pub async fn new(host: &String, port: &String, db: &String, user: &String, pass: &String) -> Self {
        let connection_str= format!("host={} port={} dbname={} user={} password={}",
                                    host, port, db, user, pass);


        let (client, connection) =
            connect(&connection_str, postgres::NoTls).await.unwrap();

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Self {
            client,
            transaction_logs_column_list: vec![
                "block_slot", "tx_idx", "tx_log_idx", "log_idx",
                "address", "log_data", "tx_hash", "topic", "topic_list"
            ],
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

    pub fn get_earliest_slot(&self) -> Result<u64, Error> {
        let slot: i64 = self.block(|| async {
            self.client.query_one("SELECT MIN(slot) FROM public.slot", &[])
                .await
        })?.try_get(0)?;

        Ok(slot as u64)
    }

    // Returns number of the slot with latest update event of the given account
    // on a closest moment before the given slot
    pub async fn get_recent_update_slot(
        &self,
        pubkey: &Pubkey,
        slot: u64
    ) -> Result<Option<u64>, Error> {
        let pubkey_bytes = pubkey.to_bytes();
        let rows = self.client.query(
            "SELECT slot, write_version FROM account_audit \
            WHERE pubkey = $1 AND slot <= $2 ORDER BY slot, write_version DESC LIMIT 1;",
            &[&pubkey_bytes.as_slice(), &(slot as i64)]
        ).await.map_err(|err| Error::GetRecentUpdateSlotErr { account: pubkey.to_hex(), slot, err })?;

        if rows.is_empty() {
            Ok(None)
        } else {
            let slot: i64 = rows[0].try_get(0)?;
            Ok(Some(slot as u64))
        }
    }
}
