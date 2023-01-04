use {
    crate::v1::types::{FilterAddress, FilterObject, LogObject},
    evm::{H160, H256, U256},
    log::*,
    openssl::ssl::{SslConnector, SslFiletype, SslMethod},
    tokio_postgres::{ connect, Client, Row },
    parity_bytes::ToPretty,
    postgres::{ NoTls },
    postgres_openssl::MakeTlsConnector,
    serde::{ Serialize, Deserialize },
    serde_yaml,
    solana_sdk::{
        account::Account,
        pubkey::Pubkey,
    },
    std::{ error, collections::HashSet, fs::File, io::self, path::Path, str::FromStr },
    thiserror::Error,
    tokio::task::block_in_place,
};
use crate::geth::{H160T, H256T, U256T};

pub struct DbClient {
    config: DBConfig,
    client: Client,
    pub transaction_logs_column_list: Vec<&'static str>,
}

const TRANSACTION_LOGS_TABLE_NAME: &str = "neon_transaction_logs";
const BLOCKS_TABLE_NAME: &str = "solana_blocks";

#[derive(Error, Debug)]
pub enum Error {
    #[error("account not found: {acc}")]
    AccNotFound{ acc: Pubkey },

    #[error("postgres: {}", .0)]
    Db(#[from] tokio_postgres::Error),

    #[error("Failed to parse topics: {entity}")]
    ParseErr{ entity: &'static str },

    #[error("Failed to get recent update for account {account} before slot {slot}: {err}")]
    GetRecentUpdateSlotErr{ account: String, slot: u64, err: tokio_postgres::Error },
}

type DbResult<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DBConfig{
    pub host: String,
    pub port: String,
    pub database: String,
    pub user: String,
    pub password: String,
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
    pub async fn new(config: &DBConfig) -> Self {
        let connection_str= format!("host={} port={} dbname={} user={} password={}",
                                    config.host, config.port, config.database, config.user, config.password);


        let (client, connection) =
            connect(&connection_str, postgres::NoTls).await.unwrap();

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Self {
            config: config.clone(),
            client,
            transaction_logs_column_list: vec![
                "block_slot", "tx_idx", "tx_log_idx", "log_idx",
                "address", "log_data", "tx_hash", "topic", "topic_list"
            ],
        }
    }

    pub fn get_config(&self) -> &DBConfig { &self.config }

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

    pub fn get_earliest_slot(&self) -> Result<u64, Error> {
        let slot: i64 = self.block(|| async {
            self.client.query_one("SELECT MIN(slot) FROM public.slot", &[])
                .await
        })?.try_get(0)?;

        Ok(slot as u64)
    }

    pub fn get_logs(
        &self,
        block_hash: Option<H256T>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        topics: Option<Vec<H256T>>,
        address: Option<FilterAddress>,
    ) -> Result<Vec<LogObject>, Error> {
        let mut query_list = Vec::new();

        if let Some(block_hash) = block_hash {
            query_list.push(format!("b.block_hash = '0x{}'", block_hash.0.to_hex()));
        } else {
            if from_block.is_none() || to_block.is_none() {
                warn!("from_block AND to_block must be specified");
                return Ok(Vec::new());
            }

            let from_block = from_block.unwrap();
            let to_block = to_block.unwrap();

            if from_block > to_block {
                warn!("from_block [{}] > to_block [{}]", from_block, to_block);
                return Ok(Vec::new());
            }

            query_list.push(format!("a.block_slot >= {}", from_block));
            query_list.push(format!("a.block_slot <= {}", to_block));
        }

        if let Some(topic_list) = topics {
            let topic_list = topic_list.iter()
                .map(|entry| format!("'0x{}'", entry.0.to_hex()))
                .collect::<Vec<String>>();
            query_list.push(format!("a.topic IN ({})", topic_list.join(", ")))
        }

        if let Some(address) = address {
            let address_list = match address {
                FilterAddress::Single(address) => vec![format!("'0x{}'", address.0.to_hex())],
                FilterAddress::Many(address_list) =>
                    address_list.iter()
                        .map(|entry| format!("'0x{}'", entry.0.to_hex()))
                        .collect::<Vec<String>>(),
            };
            query_list.push(format!("a.address IN ({})", address_list.join(", ")));
        }

        let column_list = self.transaction_logs_column_list
            .iter()
            .map(|&entry| format!("a.{}", entry))
            .collect::<Vec<String>>()
            .join(", ");

        let query_string = format!("
            SELECT {}, b.block_hash
            FROM {} AS a
            INNER JOIN {} AS b
            ON b.block_slot = a.block_slot
            AND b.is_active = True
            WHERE {}
            ORDER BY a.block_slot DESC
            LIMIT 1000",
                                   column_list,
                                   TRANSACTION_LOGS_TABLE_NAME,
                                   BLOCKS_TABLE_NAME,
                                   query_list.join(" AND "),
        );

        debug!("Querying logs: {}", query_string);

        let rows = self.block(|| async {
            self.client.query(&query_string, &[]).await
        })?;

        debug!("Found {} results", rows.len());

        let mut log_list = Vec::new();
        let mut unique_log_set: HashSet<String> = HashSet::new();
        for row in rows {
            let block_slot: i64 = row.try_get(0)?;
            let tx_idx: i32 = row.try_get(1)?;
            let tx_log_idx: i32 = row.try_get(2)?;
            let ident = format!("{}:{}:{}", block_slot, tx_idx, tx_log_idx);

            if unique_log_set.contains(&ident) {
                continue;
            }

            unique_log_set.insert(ident);
            log_list.push(self.log_from_row(row)?);
        }

        Ok(log_list)
    }

    fn log_from_row(&self, row: Row) -> Result<LogObject, Error> {
        let blocknumber: i64 = row.try_get(0)?;

        let address: String = row.try_get(4)?;
        let address = H160T(H160::from_str(&address)
            .map_err(|_| Error::ParseErr { entity: "address" })?);

        let logindex: i32 = row.try_get(3)?;

        let transactionindex: i32 = row.try_get(1)?;

        let transactionlogindex: i32 = row.try_get(2)?;

        let transactionhash: String = row.try_get(6)?;
        let transactionhash = U256T(U256::from_str(&transactionhash)
            .map_err(|_| Error::ParseErr { entity: "transactionhash" })?);

        let blockhash: String = row.try_get(9)?;
        let blockhash = U256T(U256::from_str(&blockhash)
            .map_err(|_| Error::ParseErr { entity: "blockhash" })?);

        let data: String = row.try_get(5)?;

        let topics: Vec<u8> = row.try_get(8)?;

        let topics: Vec<String> = serde_pickle::from_slice(&topics, serde_pickle::DeOptions::new())
            .map_err(|_| Error::ParseErr { entity: "topics" })?;

        let mut topic_list: Vec<U256T> = Vec::new();
        for topic in topics {
            let v = U256::from_str(&topic)
                .map_err(|_| Error::ParseErr { entity: "topics" })?;
            topic_list.push(U256T(v));
        }

        Ok(LogObject {
            removed: false,
            log_index: format!("0x{:X}", logindex),
            transaction_index: format!("0x{:X}", transactionindex),
            transaction_log_index: format!("0x{:X}", transactionlogindex),
            transaction_hash: transactionhash,
            block_hash: blockhash,
            block_number: format!("0x{:X}", blocknumber),
            address,
            data,
            topics: topic_list,
        })
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
