use {
    solana_sdk::{
        program_stubs::SyscallStubs,
        sysvar::rent::Rent,
        sysvar::clock::Clock,
        account::ReadableAccount,
    },
    anyhow::anyhow,
    crate::neon::provider::Provider
};


pub struct Stubs {
    rent: Rent,
    clock: Clock,
}

impl Stubs {
    pub fn new<P>(provider: &P, block_number: u64) -> Result<Box<Stubs>, anyhow::Error>
        where
            P: Provider
    {
        let rent = Stubs::load_rent_account(provider, block_number)?;
        let clock = Stubs::load_clock_account(provider, block_number)?;
        Ok(Box::new(Self { rent, clock }))
    }

    fn load_rent_account<P>(provider: &P, block_number: u64) -> Result<Rent, anyhow::Error>
        where
            P: Provider
    {
        let rent_pubkey = solana_sdk::sysvar::rent::id();
        // TODO: remove u64::MAX after fix get_slot_by_block
        let acc = provider.get_account_at_slot(&rent_pubkey, block_number)
            .map_err(|e| anyhow!("error load rent account {}", e))?;

        let acc = acc.ok_or_else(|| anyhow!("rent account is None"))?;
        let data = acc.data();
        bincode::deserialize(data).map_err(|e| anyhow!("error to deserialize rent account {}", e))
    }

    fn load_clock_account<P>(provider: &P, block_number: u64) -> Result<Clock, anyhow::Error>
        where
            P: Provider
    {
        let clock_pubkey = solana_sdk::sysvar::clock::id();
        // TODO: remove u64::MAX after fix get_slot_by_block
        let acc = provider.get_account_at_slot(&clock_pubkey, block_number)
            .map_err(|e| anyhow!("error load clock account {}", e))?;

        let acc = acc.ok_or_else(|| anyhow!("clock account is None"))?;
        let data = acc.data();
        bincode::deserialize(data).map_err(|e| anyhow!("error to deserialize clock account {}", e))
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

    fn sol_get_clock_sysvar(&self, pointer: *mut u8) -> u64 {
        unsafe {
            #[allow(clippy::cast_ptr_alignment)]
                let clock = pointer.cast::<Clock>();
            *clock = self.clock.clone();
        }

        0
    }
}
