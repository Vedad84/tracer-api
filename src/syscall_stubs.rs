use solana_sdk::{
    program_error::ProgramError,
    program_stubs::SyscallStubs,
    sysvar::rent::Rent,
    account::Account,
};

use anyhow::anyhow;
use solana_sdk::account::ReadableAccount;
use crate::neon::provider::Provider;

pub struct Stubs {
    rent: Rent,
}

impl Stubs {
    pub fn new<P>(provider: &P, block_number: Option<u64>) -> Result<Box<Stubs>, anyhow::Error>
    where
        P: Provider
    {
        let rent_pubkey = solana_sdk::sysvar::rent::id();

        let mut acc  = provider.get_account_at_slot(&rent_pubkey, block_number.unwrap())
            .map_err(|e| anyhow!("error load rent account {}", e))?;

        let acc = acc.ok_or(anyhow!("rent account is None"))?;
        let data = acc.data();
        let rent = bincode::deserialize(data).map_err(|e| anyhow!("error to deserialize rent account {}", e))?;

        Ok(Box::new(Self { rent }))
    }
}

impl SyscallStubs for Stubs {
    fn sol_get_rent_sysvar(&self, pointer: *mut u8) -> u64 {
        unsafe {
            #[allow(clippy::cast_ptr_alignment)]
                let rent = pointer.cast::<Rent>();
            *rent = self.rent;
        }

        0
    }
}
