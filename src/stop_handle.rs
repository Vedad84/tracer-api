use {
    tokio::{ task::JoinHandle, sync::mpsc::Sender },
};

pub struct StopHandle {
    join_handle: JoinHandle<()>,
    stop_snd: tokio::sync::mpsc::Sender<()>,
}

impl StopHandle {
    pub fn new(
        join_handle: JoinHandle<()>,
        stop_snd: tokio::sync::mpsc::Sender<()>,
    ) -> Self {
        Self {
            join_handle,
            stop_snd,
        }
    }

    pub fn stop(self) -> Result<tokio::task::JoinHandle<()>, ()> {
        self.stop_snd.try_send(()).map_err(|_| ())?;
        Ok(self.join_handle)
    }
}