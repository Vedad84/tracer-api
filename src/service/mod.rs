pub mod eip1898;

use {
    crate::{ neon::TracerCore, v1::types::BlockNumber },
    jsonrpsee::types::error::Error,
};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct ServerImpl {
    tracer_core: TracerCore,
}

impl ServerImpl {
    pub fn new(tracer_core: TracerCore) -> Self {
        Self {
            tracer_core
        }
    }

    fn get_slot_by_block(&self, bn: BlockNumber) -> Option<u64> {
        match bn {
            BlockNumber::Num(num) => Some(num),
            BlockNumber::Latest => None,
            _ => todo!(),
        }
    }
}
