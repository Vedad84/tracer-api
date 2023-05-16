use {
    neon_cli_lib::types::{TracerDb, block, ChResult},
};

pub trait TracerDbExtention{
    fn get_earliest_slot(&self) -> ChResult<u64>;
 }

impl TracerDbExtention for TracerDb {
    fn get_earliest_slot(&self) -> ChResult<u64> {
        block(|| async {
            let query = "SELECT min(slot) FROM events.update_slot";
            self.client
                .query(query)
                .fetch_one::<u64>()
                .await
                .map_err(std::convert::Into::into)
        })
    }
}

