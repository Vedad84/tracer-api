use {
    std::{time::Duration, sync::Arc, str::FromStr},
    parity_bytes::ToPretty,
    log::*,
    evm_loader::{H160, U256},
    crate::{evm_runtime::EVMRuntime},
    super::{EthereumError, Result, INTERNAL_SERVER_ERROR, ETHEREUM_ERROR_MAP, ETHEREUM_FATAL_ERROR_MAP,},
};

const NUM_STEPS_TO_EXECUTE: u32 = 500_000;


#[derive(Clone)]
pub struct NeonCli {
    chain_id: String,
    steps_to_execute: String,
    db_config: String,
    evm_loader: String,
    token_mint: String,
    pub evm_runtime: Arc<EVMRuntime>,
}

impl NeonCli{
    pub fn new(evm_runtime: Arc<EVMRuntime>) -> Self {
        NeonCli{
            chain_id : format!("{}", evm_runtime.config.chain_id),
            steps_to_execute: format!("{}", NUM_STEPS_TO_EXECUTE),
            db_config: format!("/opt/db_config/tracer_db_config.yml"),
            evm_loader: evm_runtime.config.evm_loader.to_string(),
            token_mint: evm_runtime.config.token_mint.to_string(),
            evm_runtime,
        }
    }

    pub async fn emulate (
        &self,
        from: Option<H160>,
        to: H160,
        value: Option<U256>,
        data: Option<Vec<u8>>,
        slot: u64,
        tout: &Duration,
    ) -> Result<String> {
        let slot_ = slot.to_string();
        let from = from.unwrap_or_default().to_hex();
        let to = to.to_hex();
        let value = value.unwrap_or_default().to_string();

        let command = vec![
            "neon-cli",
            "--db_config", &self.db_config,
            "--slot", &slot_,
            "--evm_loader", &self.evm_loader,
            "emulate",
            "--token_mint", &self.token_mint,
            "--chain_id", &self.chain_id,
            "--max_steps_to_execute", &self.steps_to_execute,
            &from,
            &to,
            &value,
        ];
        match self.evm_runtime.run_command_with_slot_revision(command, data, slot, tout).await {
            Ok(result) => {
                let std_out = std::str::from_utf8(&result.stdout);
                let std_err = std::str::from_utf8(&result.stderr);

                if let (Ok(stdout), Ok(stderr)) = (std_out, std_err) {
                    info!("STDOUT: {}", stdout);
                    info!("STDERR: {}", stderr);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout){
                        return self.decode_emulation_result(json)
                    }
                    warn!("Failed to parse stdout. Trying to parse from stderr");
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(stderr){
                        return self.decode_emulation_result(json)
                    }

                    warn!("Emulation failed: unable to parse emulation result");
                    Err(INTERNAL_SERVER_ERROR())

                } else {
                    warn!("Emulation error: failed to parse neon-cli result\n stdout: {:?}\n stderr: {:?}", result.stdout, result.stderr);
                    Err(INTERNAL_SERVER_ERROR())
                }
            },
            Err(err) => {
                warn!("Emulation failed: {:?}", err);
                Err(INTERNAL_SERVER_ERROR())
            },
        }
    }

    pub async fn get_storage_at(&self, to: H160, index: U256, slot: u64, tout: &Duration) -> Result<U256> {
        let slot_ = slot.to_string();
        let to = to.to_hex();
        let mut value = vec![0; 32];
        index.to_big_endian(value.as_mut_slice());
        let index = hex::encode(value);

        let command = vec![
            "neon-cli",
            "--db_config", &self.db_config,
            "--slot", &slot_,
            "--evm_loader", &self.evm_loader,
            "get-storage-at",
            &to,
            &index
        ];
        match self.evm_runtime.run_command_with_slot_revision(command, None, slot, tout).await {
            Ok(result) => {
                if let Ok(stderr) = std::str::from_utf8(&result.stderr) {
                    info!("STDERR: {}", stderr);
                    let value = U256::from_str(stderr.trim_start_matches("0x")).map_err(|e| {
                        warn!("get_strorage_at cast error: {:?}", e);
                        INTERNAL_SERVER_ERROR()
                    })?;
                    Ok(value)
                } else {
                    warn!("get_strorage_at error: failed to read neon-cli result\n stdout: {:?}\n stderr: {:?}", result.stdout, result.stderr);
                    Err(INTERNAL_SERVER_ERROR())
                }
            },
            Err(err) => {
                warn!("get_strorage_at failed: {:?}", err);
                Err(INTERNAL_SERVER_ERROR())
            },
        }
    }

    pub async fn get_balance(&self, address: H160, slot: u64, tout: &Duration) -> Result<U256> {
        self.get_ether_account_data(address, slot, tout, U256::zero(), &NeonCli::parse_balance).await
    }

    pub async fn get_trx_count(&self, address: H160, slot: u64, tout: &Duration) -> Result<U256> {
        self.get_ether_account_data(address, slot, tout, U256::zero(), &NeonCli::parse_nonce).await
    }

    pub async fn get_code(&self, address: H160, slot: u64, tout: &Duration) -> Result<String> {
        self.get_ether_account_data(address, slot, tout, "".to_string(), &NeonCli::parse_code).await
    }

    async fn get_ether_account_data <F, D>(&self, address: H160, slot: u64, tout: &Duration, default: D, parse: F) -> Result<D>
        where F: FnOnce(&str) -> Result<D>
    {
        let slot_ = slot.to_string();
        let address = address.to_hex();

        let command = vec![
            "neon-cli",
            "--db_config", &self.db_config,
            "--slot", &slot_,
            "--evm_loader", &self.evm_loader,
            "get-ether-account-data",
            &address,
        ];
        match self.evm_runtime.run_command_with_slot_revision(command, None, slot, tout).await {
            Ok(result) => {
                if let Ok(stderr) = std::str::from_utf8(&result.stderr) {
                    info!("STDERR: {}", stderr);
                    if stderr.starts_with("Account not found") {
                        return Ok(default)
                    }
                    parse(stderr)
                } else {
                    warn!("get-ether-account-data error: failed to read neon_cli result\n stdout: {:?}\n stderr: {:?}", result.stdout, result.stderr);
                    Err(INTERNAL_SERVER_ERROR())
                }
            },
            Err(err) => {
                warn!("get-ether-account-data failed: {:?}", err);
                Err(INTERNAL_SERVER_ERROR())
            },
        }
    }

    fn parse_balance_nonce (output: &str, param: &str) -> Result<U256>{
        for line in output.split('\n') {
            let line = line.trim();
            if line.starts_with(&format!("{}: ", param)){
                let value= line.split(&format!("{}: ", param)).last().expect(&format!("{} error", param));
                return Ok(U256::from_dec_str(value).expect(&format!("{} parse error", param)))
            }
        }
        warn!("get_{} error: failed to parse neon-cli result\n output: {:?}", param, output);
        Err(INTERNAL_SERVER_ERROR())
    }

    fn parse_balance(output: &str) -> Result<U256> {
        NeonCli::parse_balance_nonce(output, "balance")
    }

    fn parse_nonce(output: &str) -> Result<U256> {
        NeonCli::parse_balance_nonce(output, "trx_count")
    }

    fn parse_code (output: &str) -> Result<String> {
        let mut found = false;
        let mut code  = String::new();

        for line in output.split('\n') {
            let line = line.trim();

            if found {
                code = code + line;
            } else{
                if line.starts_with("code_size:"){
                    found = true;
                }
            }
        }
        Ok(code)
    }

    fn decode_result(&self, obj: &serde_json::Map<String, serde_json::Value>) -> Result<String> {
        return if let Some(data) = obj.get("result") {
            if let serde_json::Value::String(string) = data {
                Ok(string.clone())
            } else {
                warn!("result in emulator output is not a string");
                Err(INTERNAL_SERVER_ERROR())
            }
        } else {
            warn!("result is absend in emulator output");
            Err(INTERNAL_SERVER_ERROR())
        }
    }

    fn decode_revert_message(&self, data: &String) -> Result<Option<String>> {
        let data_len = data.len();
        if data_len == 0 {
            return Ok(None);
        }

        if data_len < 8 {
            warn!("Too less bytes to decode revert signature: {data_len}, data: 0x{data}");
            return Err(INTERNAL_SERVER_ERROR());
        }

        if &data[..8] == "4e487b71" { // keccak256("Panic(uint256)")
            return Ok(None)
        }

        if &data[..8] == "08c379a0" { // keccak256("Error(string)")
        warn!("Failed to decode revert_message, unknown revert signature: {}", data[..8].to_string());
            return Ok(None)
        }

        if data_len < 8 + 64 {
            warn!("Too less bytes to decode revert msg offset: {data_len}, data: 0x{data}");
            return Err(INTERNAL_SERVER_ERROR())
        }

        let offset = usize::from_str_radix(&data[8..(8 + 64)], 16)
            .map_err(|err| {
                warn!("Failed to parse rever reason offset: {data_len}, data: 0x{data}: {err:?}");
                INTERNAL_SERVER_ERROR()
            })?;
        let offset = offset * 2;

        if data_len < 8 + offset + 64 {
            warn!("Too less bytes to decode revert msg len: {data_len}, data: 0x{data}");
            return Err(INTERNAL_SERVER_ERROR());
        }

        let length = usize::from_str_radix(&data[(8 + offset)..(8 + offset + 64)], 16)
            .map_err(|err| {
                warn!("Failed to parse revert reason length: {data_len}, data: 0x{data}: {err:?}");
                INTERNAL_SERVER_ERROR()
            })?;
        let length = length * 2;

        if data_len < 8 + offset + 64 + length {
            warn!("Too less bytes to decode revert msg: {data_len}, data: 0x{data}");
            return Err(INTERNAL_SERVER_ERROR());
        }

        let message_bytes = hex::decode(&data[(8 + offset + 64)..(8 + offset + 64 + length)])
            .map_err(|err| {
                warn!("Failed to decode revert from hex length: {data_len}, data: 0x{data}: {err:?}");
                INTERNAL_SERVER_ERROR()
            })?;

        let message = std::str::from_utf8(&message_bytes).map_err(|err| {
            warn!("Failed to decode UTF-8 from revert message bytes: {data_len}, data: 0x{data}: {err:?}");
            INTERNAL_SERVER_ERROR()
        })?;

        Ok(Some(message.to_string()))
    }

    fn decode_revert_result(&self, obj: &serde_json::Map<String, serde_json::Value>) -> Result<String> {
        let revert_data = self.decode_result(obj)?;
        let error =
            if let Some(result_value) = self.decode_revert_message(&revert_data)? {
                EthereumError {
                    code: 3,
                    message: Some(format!("execution reverted: {}", result_value)),
                    data: Some(format!("0x{}", revert_data)),
                }
            } else {
                EthereumError {
                    code: 3,
                    message: Some(format!("execution reverted")),
                    data: Some(format!("0x{}", revert_data)),
                }
            };

        serde_json::to_string(&error).map_err(|err| {
            warn!("Failed to serialize error message: {:?}", err);
            INTERNAL_SERVER_ERROR()
        })
    }

    fn decode_error_result(&self, exit_status: &String, obj: &serde_json::Map<String, serde_json::Value>) -> Result<String> {
        let error = if let Some(reason) = obj.get("exit_reason") {
            match reason {
                serde_json::Value::String(reason) => {
                    EthereumError {
                        code: 3,
                        message: Some(format!("execution finished with error: {reason}")),
                        data: None
                    }
                },
                serde_json::Value::Object(_obj) => {
                    let mut error: Option<String> = None;
                    if let Some(err) = reason.get("Error") {
                        error = ETHEREUM_ERROR_MAP.get(err.to_string().as_str()).map(|s| s.to_string());
                    }

                    if error.is_none() {
                        if let Some(fatal) = reason.get("Fatal") {
                            error = ETHEREUM_FATAL_ERROR_MAP.get(fatal.to_string().as_str()).map(|s| s.to_string());
                        }
                    }

                    if let Some(error) = error {
                        EthereumError {
                            code: 3,
                            message: Some(format!("execution finished with error: {error}")),
                            data: None
                        }
                    } else {
                        EthereumError {
                            code: 3,
                            message: Some(exit_status.clone()),
                            data: None
                        }
                    }
                },
                _ => {
                    EthereumError {
                        code: 3,
                        message: Some(exit_status.clone()),
                        data: None
                    }
                }
            }
        } else {
            EthereumError {
                code: 3,
                message: Some(exit_status.clone()),
                data: None
            }
        };

        serde_json::to_string(&error).map_err(|err| {
            warn!("Failed to serialize error {:?} to string: {:?}", error, err);
            INTERNAL_SERVER_ERROR()
        })
    }

    fn decode_succeed_result(&self, obj: &serde_json::Map<String, serde_json::Value>) -> Result<String> {
        if let Some(result) = obj.get("result") {
            if let serde_json::Value::String(result) = result {
                return Ok(format!("0x{result}"));
            }

            warn!("Unexpected result type in JSON");
            return Err(INTERNAL_SERVER_ERROR())
        }

        Ok("0x".to_string())
    }

    fn decode_emulation_result(&self, result: serde_json::Value) -> Result<String> {
        return match result {
            serde_json::Value::Object(obj) => {
                if let Some(exit_status) = obj.get("exit_status") {
                    if let serde_json::Value::String(exit_status) = exit_status {
                        if exit_status == "revert" {
                            self.decode_revert_result(&obj)
                        } else if exit_status != "succeed" {
                            self.decode_error_result(exit_status, &obj)
                        } else {
                            self.decode_succeed_result(&obj)
                        }
                    } else {
                        error!("exit_status expected to be a String");
                        Err(INTERNAL_SERVER_ERROR())
                    }
                } else {
                    error!("Emulation exit_status undefined");
                    Err(INTERNAL_SERVER_ERROR())
                }

            },
            _ => {
                error!("Emulation result expected to be JSON Object");
                Err(INTERNAL_SERVER_ERROR())
            },
        }
    }

}

