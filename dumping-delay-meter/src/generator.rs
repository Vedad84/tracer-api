use {
    crate::{
        contracts::{BaseContract, TestFactoryContract},
        stop_handle::StopHandle,
    },
    tracing::info,
    web3::transports::Http,
};

pub struct Generator {
    factory: BaseContract,
    caller: secp256k1::SecretKey,
}

impl Generator {
    pub fn new(
        factory_contract: web3::contract::Contract<Http>,
        caller: secp256k1::SecretKey,
    ) -> Self {
        Self {
            factory: BaseContract::new(factory_contract),
            caller,
        }
    }

    pub fn run(self, generation_interval_ms: u64) -> StopHandle {
        info!("Starting Generator...");
        let (stop_snd, mut stop_rcv) = tokio::sync::mpsc::channel::<()>(1);
        StopHandle::new(
            tokio::spawn(async move {
                let sleep_int = std::time::Duration::from_millis(generation_interval_ms);
                let mut interval = tokio::time::interval(sleep_int);
                interval.tick().await;
                let mut num_calls = 0;
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            info!("Creating new contract...");
                            let result = self.factory.create_new_contract(&self.caller).await;
                            info!("Generator iteration {} result: {:?}", num_calls, result);
                            num_calls += 1;
                        }
                        _ = stop_rcv.recv() => {
                            break;
                        }
                    }
                }
                info!("Generator stopped");
            }),
            stop_snd,
        )
    }
}
