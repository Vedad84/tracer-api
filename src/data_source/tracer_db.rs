use {
    solana_sdk::pubkey::Pubkey,
    std::{convert::TryFrom},
    neon_cli_lib::types::{TracerDb, block, PgError, PgResult},
};

pub trait TracerDbExtention{
    // Returns number of the slot with latest update event of the given account
    // on a closest moment before the given slot
    fn get_recent_update_slot( &self,  pubkey: &Pubkey, slot: u64) -> PgResult<Option<u64>>;
    fn get_earliest_slot(&self) -> PgResult<u64>;
 }

impl TracerDbExtention for TracerDb {

    fn get_recent_update_slot( &self, pubkey: &Pubkey, slot: u64) -> PgResult<Option<u64>> {
        let pubkey_bytes = pubkey.to_bytes();

        let rows = block(|| async {
            self.client.query(
                "SELECT slot, write_version FROM public.account_audit \
            WHERE pubkey = $1 AND slot <= $2 ORDER BY slot, write_version DESC LIMIT 1;",
                &[&pubkey_bytes.as_slice(), &(slot as i64)]
            ).await
        })?;

        if rows.is_empty() {
            Ok(None)
        } else {
            let slot: i64 = rows[0].try_get(0)?;
            let slot = u64::try_from(slot).map_err(|e| PgError::Custom(format!("slot cast error: {}", e)))?;
            Ok(Some(slot))
        }
    }

    fn get_earliest_slot(&self) -> PgResult<u64> {
        let row = block(|| async {
            self.client.query_one("SELECT MIN(slot) FROM public.slot", &[]).await
        })?;

        let slot: i64 = row.try_get(0)?;
        u64::try_from(slot).map_err(|e| PgError::Custom(format!("slot cast error: {}", e)))
    }
}

