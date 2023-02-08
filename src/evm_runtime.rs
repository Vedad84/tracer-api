use {
    arrayref::array_ref,
    bollard::{
        container::{
            CreateContainerOptions, Config as ContainerConfig,
            LogOutput as DockerLogOutput,
        },
        exec::{CreateExecOptions, StartExecOptions, StartExecResults},
        errors::Error as DockerError,
        models::ContainerStateStatusEnum,
    },
    futures_util::{ future::join_all, StreamExt },
    goblin::elf::Elf,
    log::*,
    solana_sdk::pubkey::Pubkey,
    std::{
        collections::{ HashMap, HashSet },
        convert::TryFrom,
        default::Default,
        io::Read,
        ops::{ Deref, DerefMut },
        sync::Arc,
        sync::atomic::AtomicUsize,
        time::Duration,
    },
    tokio::{ io::AsyncWriteExt, sync::RwLock },
    serde::Deserialize,
    crate::{stop_handle::StopHandle, data_source::tracer_db::TracerDbExtention },
    neon_cli_lib::types::{TracerDb, PgError}
};

#[derive(Debug, thiserror::Error)]
pub enum EVMRuntimeError {
    #[error("Failed to connect to docker: {err}")]
    ClientCreationError{ err: String },

    #[error("Failed to update statuses of containers: {err}")]
    UpdateStatusesError{ err: DockerError },

    #[error("Failed to create new container {name}: {err}")]
    ContainerCreationError{ name: String, err: DockerError },

    #[error("Failed to execute command in container {name}: {err}")]
    ExecuteCommandError{ name: String, err: String },

    #[error("Failed to read container {name} status: {err}")]
    ContainerStatusReadError{ name: String, err: String },

    #[error("Unable to wakeup container {name}: {err}")]
    WakeupContainerError{ name: String, err: String },

    #[error("Failed to read EVM revision: {err}")]
    ReadEvmRevisionError{ err: String },

    #[error("Database error: {err}")]
    DbClientError { err: PgError },

    #[error("Known revisions data corrupted")]
    KnownRevisionsCorrupted,

    #[error("Failed to pull image {image_name}")]
    PullImageError{ image_name: String },

    #[error("Failed to upload DB config into container {name}: {err}")]
    UploadDBConfigError{ name: String, err: String },

    #[error("Failed to parse ELF params for EVM in slot {slot}: {err}")]
    ParseElfError{ slot: u64, err: String },

    #[error("Timeout reached")]
    TimeoutError,
}

// Aimed to store names of known containers - maps EVM revision to corresponding container name
type EVMContainerMap = HashMap<String, String>;

#[derive(Clone, Debug)]
pub struct EVMRuntimeConfig {
    pub docker_socket: String,
    pub docker_tout: u64,
    pub docker_version_minor: usize,
    pub docker_version_major: usize,
    pub update_interval_sec: u64,
    pub running_to_suspended_time_sec: u64,
    pub suspended_to_stopped_time_sec: u64,
    pub stopped_to_dead_time_sec: u64,
    pub known_revisions: String,
    pub db_config_tar: String,
    pub evm_loader: Pubkey,
    pub token_mint: Pubkey,
    pub chain_id: u16,
    pub network_name: Option<String>,
}

#[derive(Clone)]
pub struct EVMRuntime {
    pub config: EVMRuntimeConfig,
    docker: bollard::Docker,
    known_containers: Arc<RwLock<EVMContainerMap>>,
    known_revisions: Arc<RwLock<KnownRevisions>>,
    tracer_db: TracerDb,
}

pub struct ExecResult {
    pub stdin: Vec<u8>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub console: Vec<u8>,
}

// Constants
const CONTAINER_NAME_PREFIX: &str = "neon_tracer_evm_container_";
const IMAGE_NAME_PREFIX: &str = "neonlabsorg/evm_loader:";

// Single entry of known revision in config
#[derive(Debug, Deserialize)]
struct KnownRevision {
    pub slot: u64,
    pub revision: String,
}

/// Type alias for list of known revisions passed through config
type KnownRevisionsData = Vec<KnownRevision>;

#[derive(Debug)]
struct KnownRevisions {
    data: KnownRevisionsData, // revisions sorted by deploy slot number
}

impl KnownRevisions {
    pub fn new() -> Self {
        Self {
            data: Vec::new()
        }
    }

    /// Finds closest slot number < given slot and returns revision associated with it (or error)
    fn search_closest_update_index(&self, slot: u64) -> Result<Option<usize>, EVMRuntimeError> {
        debug!("search_closest_update_index({slot})");
        let mut start_idx = 0;
        let mut stop_idx = (self.data.len() as i64 - 1) as usize;
        let mut int_len = (stop_idx as i64 - start_idx as i64 + 1) as usize;

        // using binary search
        while int_len != 0 {
            let slot_start = self.data[start_idx].slot;
            let slot_stop = if int_len == 1 {
                u64::MAX
            } else {
                self.data[stop_idx].slot
            };

            if slot < slot_start {
                return Ok(None);
            } else if slot_stop <= slot {
                return Ok(Some(stop_idx));
            }

            if int_len <= 2 {
                if slot_start <= slot && slot < slot_stop {
                    return Ok(Some(start_idx));
                }

                return Ok(None);
            }

            let middle = start_idx + int_len / 2;
            let slot_middle = self.data[middle].slot;
            if slot_start <= slot && slot < slot_middle {
                stop_idx = middle;
            } else if slot_middle <= slot && slot < slot_stop {
                start_idx = middle;
            } else {
                return Err(EVMRuntimeError::KnownRevisionsCorrupted);
            }

            int_len = stop_idx - start_idx + 1;
        }

        Ok(None)
    }

    pub fn get_slot_revision(&self, slot: u64) -> Result<Option<String>, EVMRuntimeError> {
        self.search_closest_update_index(slot).map(
            |idx| idx.map(|idx| self.data[idx].revision.clone())
        )
    }

    pub fn set_slot_revision(&mut self, slot: u64, revision: &str) -> Result<(), EVMRuntimeError> {
        let idx = self.search_closest_update_index(slot)?;
        if let Some(idx) = idx {
            if self.data[idx].revision == revision {
                // the same revision - skip
                return Ok(());
            }
            self.data.insert(
                idx + 1,
                KnownRevision{
                    slot,
                    revision: revision.to_string()
                });
        } else {
            self.data.insert(
                0,
                KnownRevision {
                    slot,
                    revision: revision.to_string()
                });
        }

        return Ok(())
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn last(&self) -> Option<String> {
        self.data.last().map(|e| e.revision.clone())
    }
}

fn create_image_name(revision: &str) -> String {
    format!("{}{}", IMAGE_NAME_PREFIX, revision)
}

fn create_container_name(revision: &str) -> String {
    format!("{}{}", CONTAINER_NAME_PREFIX, revision)
}

impl EVMRuntime {
    pub async fn new(
        config: &EVMRuntimeConfig,
        tracer_db: TracerDb,
    ) -> Result<Self, EVMRuntimeError> {
        let client_version = bollard::ClientVersion::from(
            &(AtomicUsize::new(config.docker_version_major), AtomicUsize::new(config.docker_version_minor))
        );
        let docker = bollard::Docker::connect_with_local(
            &config.docker_socket,
            config.docker_tout,
            &client_version
        ).map_err(|err| EVMRuntimeError::ClientCreationError {
            err: format!("Failed to create docker connection to {}, {:?}", config.docker_socket.clone(), err)
        })?;

        Ok(Self {
            config: config.clone(),
            docker: docker.negotiate_version().await
                .map_err(|err| EVMRuntimeError::ClientCreationError {
                    err: format!("Failed to negotiate client version: {:?}", err)
                })?,
            known_containers: Arc::new(RwLock::new(HashMap::new())),
            known_revisions: Arc::new(RwLock::new(KnownRevisions::new())),
            tracer_db,
        })
    }

    async fn remove_container(&self, container_name: &str) {
        let options = bollard::container::RemoveContainerOptions {
            force: true,
            ..Default::default()
        };
        match self.docker.remove_container(container_name, Some(options)).await {
            Ok(_) => { info!("Container {} killed", container_name); },
            Err(err) => { warn!("Failed to killcontainer {}: {:?}", container_name, err); },
        };
    }

    async fn clear_containers(&self) {
        info!("Cleaning containers...");
        let mut known_containers_lock = self.known_containers.write().await;
        let known_containers = known_containers_lock.deref_mut();
        join_all(known_containers.iter().map(|(_, container)| async move {
            info!("Stopping container {}", container);
            self.remove_container(container).await;
        })).await;
    }

    async fn heartbeat(&self) {
        info!("EVM Runtime Heartbeat");
        let mut known_containers_lock = self.known_containers.write().await;
        let known_containers = known_containers_lock.deref_mut();

        if let Err(err) = self.update_known_containers(known_containers).await {
            warn!("Failed to update known containers: {:?}", err);
        }
    }

    pub async fn run(self, mut stop_rcv: tokio::sync::mpsc::Receiver<()>) {
        info!("Starting EVM Runtime...");
        let known_revisions: KnownRevisionsData = serde_json::from_str(&self.config.known_revisions)
            .map_err(|err| EVMRuntimeError::ClientCreationError {
                err: format!("Failed to deserialize known_revisions: {}, {}", self.config.known_revisions, err),
            }).unwrap();

        for entry in known_revisions {
            info!("Set known revision for slot {}: {}", entry.slot, entry.revision);
            self.set_known_slot_revision(entry.slot, &entry.revision).await
                .expect(&format!("error set_known_slot_revision {} {}", entry.slot, &entry.revision));
        }

        let sleep_duration = Duration::from_secs(self.config.update_interval_sec);
        let mut interval = tokio::time::interval(sleep_duration);
        interval.tick().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.heartbeat().await;
                }
                _ = stop_rcv.recv() => {
                    break;
                }
            }
        }

        self.clear_containers().await;
        info!("EVM runtime stopped");
    }

    pub fn start(self) -> StopHandle {
        let (stop_snd, stop_rcv) = tokio::sync::mpsc::channel::<()>(1);
        StopHandle::new(
            tokio::spawn( self.run(stop_rcv)),
            stop_snd,
        )
    }

    async fn update_known_containers(&self, known_containers: &mut EVMContainerMap) -> Result<(), EVMRuntimeError> {
        debug!("Update known containers");
        let all_containers = self.docker.list_containers::<&str>(
            Some(bollard::container::ListContainersOptions {
                all: true,
                ..Default::default()
            }),
        )
            .await
            .map_err(|err| EVMRuntimeError::UpdateStatusesError { err })?;

        let mut found_containers: HashSet<String> = HashSet::new();

        all_containers.iter().for_each(|entry| {
            if entry.names == None ||
                entry.names.as_ref().unwrap().len() != 1 ||
                entry.command == None ||
                entry.image == None {
                return;
            }

            let container_name = {
                let tmp = &entry.names.as_ref().unwrap()[0];
                let first_char = tmp.chars().next();
                if let Some(first_char) = first_char {
                    if first_char == '/' { // docker (or bollard) adds prefix slash for some reason
                        &tmp[1..]
                    } else {
                        &tmp
                    }
                } else {
                    ""
                }
            };

            if !container_name.starts_with(CONTAINER_NAME_PREFIX) {
                return;
            }
            let container_revision = &container_name[CONTAINER_NAME_PREFIX.len()..];

            let image_name = &entry.image.as_ref().unwrap();
            if !image_name.starts_with(IMAGE_NAME_PREFIX) {
                return;
            }
            let image_revision = &image_name[IMAGE_NAME_PREFIX.len()..];

            if container_revision != image_revision {
                return;
            }

            debug!("Found container {}", container_name);
            found_containers.insert(container_revision.to_string());

            if let Some(state) = &entry.state {
                if let Some(known_container) = known_containers.get_mut(container_revision) {
                    debug!("Container {} state {}", known_container, state);
                } else {
                    debug!("New container found {}. Current state {}", container_name.clone(), state);
                    known_containers.insert(container_revision.to_string(), container_name.to_string());
                };
            } else {
                warn!("Container {} found but its status is not defined.", container_name);
            }
        });

        // remove containers which are not exist anymore
        let to_remove: HashSet<String> = known_containers.iter().filter_map(|(revision, _)| {
            if !found_containers.contains(revision) {
                return Some(revision.clone());
            }
            None
        }).collect();

        for revision in to_remove {
            warn!("Container {} vanished!", create_container_name(&revision));
            known_containers.remove(&revision);
        }

        Ok(())
    }

    async fn wakeup_container_impl(
        &self,
        container_name: &String
    ) -> Result<(), EVMRuntimeError> {
        let docker_err_to_cache_err = |err: DockerError| {
            EVMRuntimeError::WakeupContainerError {
                name: container_name.clone(),
                err: format!("{:?}", err),
            }
        };

        let current_status = self.get_container_status(container_name).await?;
        match current_status {
            ContainerStateStatusEnum::EMPTY =>
                Err(EVMRuntimeError::WakeupContainerError {
                    name: container_name.clone(),
                    err: "Current status is empty!".to_string(),
                }),

            ContainerStateStatusEnum::RUNNING |
            ContainerStateStatusEnum::CREATED |
            ContainerStateStatusEnum::RESTARTING => Ok(()),

            ContainerStateStatusEnum::PAUSED => {
                info!("Container {} will be unpaused", container_name);
                self.docker.unpause_container(container_name.as_str()).await
                    .map_err(docker_err_to_cache_err)
            }

            ContainerStateStatusEnum::EXITED => {
                info!("Container {} will be started", container_name);
                self.docker.start_container::<&str>(container_name.as_str(), None).await
                    .map_err(docker_err_to_cache_err)
            }

            ContainerStateStatusEnum::DEAD => {
                info!("Container {} is DEAD. It will be restarted", container_name);
                self.docker.restart_container(container_name.as_str(), None).await
                    .map_err(docker_err_to_cache_err)
            }

            ContainerStateStatusEnum::REMOVING =>
                Err(EVMRuntimeError::WakeupContainerError {
                    name: container_name.clone(),
                    err: "Unable to start removing container".to_string(),
                })
        }
    }

    async fn upload_db_config(&self, container_name: &str) -> Result<(), EVMRuntimeError> {
        debug!("upload_db_config({}, {})", container_name, self.config.db_config_tar);

        let mut file = std::fs::File::open(self.config.db_config_tar.clone())
            .map_err(|err| EVMRuntimeError::UploadDBConfigError {
                name: container_name.to_string(),
                err: format!("Unable to open tarbal file {}: {:?}", self.config.db_config_tar.clone(), err),
            })?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|err| EVMRuntimeError::UploadDBConfigError {
                name: container_name.to_string(),
                err: format!("Failed to read tarbal file {} content: {:?}", self.config.db_config_tar.clone(), err),
            })?;

        let options = Some(bollard::container::UploadToContainerOptions{
            path: "/opt",
            ..Default::default()
        });

        self.docker.upload_to_container(container_name, options, contents.into())
            .await.map_err(
            |err| EVMRuntimeError::UploadDBConfigError {
                name: container_name.to_string(),
                err: format!("upload error: {:?}", err),
            })
    }

    async fn new_container(&self, revision: &str) -> Result<String, EVMRuntimeError> {
        let image_name = create_image_name(revision);
        let container_name = create_container_name(revision);
        info!("Creating container {} from image {}", container_name, image_name);

        let response = self.docker.create_container(
            Some(CreateContainerOptions {
                name: container_name.clone()
            }),
            ContainerConfig {
                image: Some(image_name),
                cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
                tty: Some(true),
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                ..Default::default()
            }
        )
            .await
            .map_err(|err| EVMRuntimeError::ContainerCreationError { name: container_name.to_string(), err })?;

        if !response.warnings.is_empty() {
            warn!(
                    "There were warnings while creating container {}: {:?}",
                    container_name, response.warnings
                );
        }

        {
            let mut known_containers_lock = self.known_containers.write().await;
            let known_containers = known_containers_lock.deref_mut();
            known_containers.insert(revision.to_string(), container_name.clone());
        };

        self.upload_db_config(&container_name).await?;

        if let Some(network_name) = &self.config.network_name {
            info!("Connecting container {} to network {}", container_name, network_name);
            self.docker.connect_network(
                &network_name,
                bollard::network::ConnectNetworkOptions {
                    container: &container_name,
                    endpoint_config: bollard::models::EndpointSettings::default(),
                })
                .await
                .map_err(|err| EVMRuntimeError::ContainerCreationError { name: container_name.clone(), err  })?;
        }

        info!("Starting container {} ...", &container_name);
        if let Err(err) = self.docker.start_container::<&str>(&container_name, None).await {
            return Err(EVMRuntimeError::ContainerCreationError {
                name: container_name.to_string(),
                err,
            })
        }

        Ok(container_name)
    }

    async fn get_container_status(&self, container_name: &String) -> Result<ContainerStateStatusEnum, EVMRuntimeError> {
        let res = self.docker.inspect_container(&container_name, None).await
            .map_err(|err| EVMRuntimeError::ContainerStatusReadError {
                name: container_name.clone(),
                err: format!("{:?}", err),
            })?;
        if let Some(state) = res.state {
            if let Some(status) = state.status {
                return Ok(status)
            }
        }

        Err(EVMRuntimeError::ContainerStatusReadError {
            name: container_name.clone(),
            err: "Status not set".to_string()
        })
    }

    async fn wakeup_container(
        &self,
        container_name: &String,
        tout: &std::time::Duration,
    ) -> Result<(), EVMRuntimeError> {
        self.wakeup_container_impl(container_name).await?;
        self.wait_container_running(container_name, tout).await
    }

    async fn wait_container_running(
        &self,
        container_name: &String,
        tout: &std::time::Duration
    ) -> Result<(), EVMRuntimeError> {
        let task = async {
            let sleep_int = std::time::Duration::from_secs(1);
            let mut interval = tokio::time::interval(sleep_int);
            interval.tick().await;
            loop {
                let current_status = self.get_container_status(container_name).await?;
                if current_status == ContainerStateStatusEnum::RUNNING {
                    break;
                }
                interval.tick().await;
            }

            Ok::<(), EVMRuntimeError>(())
        };

        let res = tokio::time::timeout(*tout, task).await;
        return if let Ok(res) = res {
            res
        } else {
            Err(EVMRuntimeError::TimeoutError)
        }
    }

    async fn run_command_private(
        &self,
        container_name: &String,
        command: Vec<&str>,
        stdin_data: Option<Vec<u8>>,
    ) -> Result<ExecResult, EVMRuntimeError> {

        let config = CreateExecOptions {
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(command),
            ..Default::default()
        };

        let result = self.docker.create_exec(
            &container_name, config
        ).await.map_err(|err| EVMRuntimeError::ExecuteCommandError {
            name: container_name.clone(),
            err: format!("create exec error: {:?}", err),
        })?;

        let exec_res = self.docker.start_exec(
            &result.id,
            Some(StartExecOptions { detach: false })
        ).await;

        if let Ok(result) = exec_res {
            match result {
                StartExecResults::Attached { mut output, mut input } => {
                    if let Some(stdin_data) = stdin_data {
                        let stdin_data = Vec::from(hex::encode(stdin_data).as_bytes());
                        info!("Write stdin: {:?}", &stdin_data);
                        if let Err(err) = input.write_all(&stdin_data).await {
                            return Err(EVMRuntimeError::ExecuteCommandError {
                                name: container_name.clone(),
                                err: format!("writing stdin: {:?}", err),
                            });
                        }

                        match input.shutdown().await {
                            Ok(()) => info!("stdin shutdown"),
                            Err(err) =>
                                return Err(EVMRuntimeError::ExecuteCommandError {
                                    name: container_name.clone(),
                                    err: format!("Failed to write stdin data: {:?}", err),
                                }),
                        }
                    }

                    let mut result = ExecResult {
                        stdin: Vec::new(),
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                        console: Vec::new(),
                    };

                    while let Some(item) = output.next().await {
                        match item {
                            Ok(item) => match item {
                                DockerLogOutput::StdErr { message } => {
                                    let mut tmp: Vec<u8> = message.into_iter().collect();
                                    result.stdout.append(&mut tmp);
                                },
                                DockerLogOutput::StdOut { message } => {
                                    let mut tmp: Vec<u8> = message.into_iter().collect();
                                    result.stderr.append(&mut tmp);
                                },
                                DockerLogOutput::Console { message } => {
                                    let mut tmp: Vec<u8> = message.into_iter().collect();
                                    result.console.append(&mut tmp);
                                },
                                DockerLogOutput::StdIn { message } => {
                                    let mut tmp: Vec<u8> = message.into_iter().collect();
                                    result.stdin.append(&mut tmp);
                                }
                            },
                            Err(err) => {
                                warn!("Failed to fetch data from output: {:?}", err);
                            },
                        }
                    }

                    return Ok(result)
                },
                StartExecResults::Detached =>
                    return Err(EVMRuntimeError::ExecuteCommandError {
                        name: container_name.clone(),
                        err: "Container detached".to_string(),
                    })
            }
        }

        return Err(EVMRuntimeError::ExecuteCommandError {
            name: container_name.clone(),
            err: format!("start exec error: {:?}", exec_res.err().unwrap()),
        });
    }

    pub async fn run_command_with_revision(
        &self,
        command: Vec<&str>,
        stdin_data: Option<Vec<u8>>,
        revision: &str,
        tout: &std::time::Duration,
    ) -> Result<ExecResult, EVMRuntimeError> {
        debug!("run_command_with_revision('{:?}', '{}', {:?})", command, revision, tout);
        {
            let known_containers_lock = self.known_containers.read().await;
            let known_containers = known_containers_lock.deref();

            if let Some(container_name) = known_containers.get(revision) {
                debug!("using existing container {}", container_name);
                self.wakeup_container(container_name, tout).await?;
                return self.run_command_private(container_name, command, stdin_data).await;
            }
        }

        debug!("create new container");
        let container = self.new_container(revision).await?;
        self.wait_container_running(&container, tout).await?;
        return self.run_command_private(&container, command, stdin_data).await;
    }

    fn parse_elf_params(program_data: &[u8]) -> Result<HashMap<String,String>, EVMRuntimeError> {
        debug!("parse_elf_params(<DATA>)");
        let elf = Elf::parse(program_data)
            .map_err(|err| EVMRuntimeError::ReadEvmRevisionError{ err: format!("Failed to parse program data: {:?}", err) })?;
        let mut elf_params: HashMap::<String,String> = HashMap::new();

        elf.dynsyms.iter().for_each(|sym| {
            let name = String::from(&elf.dynstrtab[sym.st_name]);
            if name.starts_with("NEON_") {
                let end = program_data.len();
                let from = usize::try_from(sym.st_value);
                if from.is_err() {
                    warn!("Unable to cast usize from u64:{:?}", sym.st_value);
                    return;
                }
                let from = from.unwrap();

                let to= usize::try_from(sym.st_value + sym.st_size);
                if to.is_err() {
                    warn!("Unable to cast usize from u64:{:?}. Error: {}", sym.st_value + sym.st_size, to.err().unwrap());
                    return;
                }
                let to = to.unwrap();

                if to < end && from < end {
                    let buf = &program_data[from..to];
                    if let Ok(value) = std::str::from_utf8(buf) {
                        elf_params.insert(name, String::from(value));
                    } else {
                        warn!("{} unable to parse from utf-8", name);
                    }
                } else {
                    warn!("{} is out of bounds", name);
                }
            };
        });

        Ok(elf_params)
    }

    async fn get_evm_revision(&self, slot: u64) -> Result<Option<String>, EVMRuntimeError> {
        const BPF_LOADER_HEADER_SIZE: usize = 0x2d;

        debug!("Reading evm revision for slot {}", slot);
        if let Some(loader_account) = self.tracer_db.get_account_at(&self.config.evm_loader, slot)
            .map_err(|err| EVMRuntimeError::ParseElfError { slot, err: format!("Failed to read EVM loader account: {:?}", err) })? {

            if loader_account.data.len() != 36 {
                return Err(EVMRuntimeError::ParseElfError { slot, err: "Wrong EVM loader account data (len != 36)".to_string() });
            }

            let code_addr_bytes = array_ref![loader_account.data.as_slice(), 4, 32];
            let code_pubkey = Pubkey::from(code_addr_bytes.clone());
            info!("Code account is {}", code_pubkey.to_string());

            if let Some(code_acc) = self.tracer_db.get_account_at(&code_pubkey, slot)
                .map_err(|err| EVMRuntimeError::ParseElfError {
                    slot, err: format!("Failed to read EVM Code account: {:?}", err)
                })? {
                info!("Code account size {}", code_acc.data.len());
                let params = Self::parse_elf_params(&code_acc.data.as_slice()[BPF_LOADER_HEADER_SIZE..])?;
                if let Some(revision) = params.get("NEON_REVISION") {
                    return Ok(Some(revision.clone()));
                }
            }

            warn!("Code account not found");
        }

        Ok(None)
    }

    async fn get_known_slot_revision(&self, slot: u64) -> Result<Option<String>, EVMRuntimeError> {
        debug!("get_known_slot_revision({slot})");
        let known_revisions_lock = self.known_revisions.read().await;
        let known_revisions = known_revisions_lock.deref();
        known_revisions.get_slot_revision(slot)
    }

    async fn prepare_image(&self, revision: &str) -> Result<(), EVMRuntimeError> {
        let image_name = create_image_name(revision);
        info!("Trying to pull image {}", image_name);
        let mut stream = self.docker.create_image(
            Some(
                bollard::image::CreateImageOptions {
                    from_image: "neonlabsorg/evm_loader",
                    tag: revision,
                    ..Default::default()
                }
            ),
            None,
            None
        );

        while let Some(res) = stream.next().await {
            match res {
                Ok(create_info) => {
                    if let Some(ref image_id) = create_info.id {
                        if let Some(ref progress) = create_info.progress {
                            info!("{} {}", image_id, progress);
                        }
                    }
                }
                Err(err) => {
                    warn!("Failed to pull image: {:?}", err);
                    return Err(EVMRuntimeError::PullImageError{ image_name });
                }
            }
        }

        info!("Image {} pulled successfully", image_name);
        Ok(())
    }

    async fn set_known_slot_revision(&self, slot: u64, revision: &str) -> Result<(), EVMRuntimeError> {
        debug!("set_known_slot_revision({slot}, {revision})");
        let mut known_revisions_lock = self.known_revisions.write().await;
        let known_revisions = known_revisions_lock.deref_mut();
        known_revisions.set_slot_revision(slot, revision)?;
        self.prepare_image(revision).await
    }

    async fn get_db_slot_revision(&self, slot: u64) -> Result<Option<String>, EVMRuntimeError> {
        debug!("get_db_slot_revision({slot})");
        let evm_update_slot = self.tracer_db.get_recent_update_slot(&self.config.evm_loader, slot)
            .map_err(|err| EVMRuntimeError::DbClientError{ err })?;

        if let Some(evm_update_slot) = evm_update_slot {
            // new revision found - parse account and add to list of known revisions
            if let Some(revision) = self.get_evm_revision(evm_update_slot).await? {
                self.set_known_slot_revision(evm_update_slot, &revision).await?;
                return Ok(Some(revision));
            }
        }

        Ok(None)
    }

    async fn get_slot_revision(&self, slot: u64) -> Result<String, EVMRuntimeError> {
        debug!("get_slot_revision({})", slot);

        let latest_revision: Option<String>;
        {
            let known_revisions_lock = self.known_revisions.read().await;
            let known_revisions = known_revisions_lock.deref();
            latest_revision = known_revisions.last();
        };

        if let Some(latest_revision) = latest_revision {
            // known revisions is not empty
            debug!("latest_revision = {}", latest_revision);
            let rev = self.get_known_slot_revision(slot).await;
            debug!("known_revision = {:?}", rev);
            if let Ok(rev) = rev {
                if let Some(rev) = rev {
                    // if we found latest revision, maybe something changed onchain - force dump reading
                    if rev != latest_revision {
                        // otherwise - return this known revision
                        return Ok(rev);
                    }

                    if let Some(db_revision) = self.get_db_slot_revision(slot).await? {
                        return Ok(db_revision);
                    }

                    debug!("get_slot_revision: return latest revision");
                    return Ok(rev);
                }
            } else {
                let mut known_revisions_lock = self.known_revisions.write().await;
                let known_revisions = known_revisions_lock.deref_mut();
                error!("Failed to read known revision. Data maybe corrupted: {:?}. \
                Clear known revisions. It will be read from chain next time.", known_revisions);
                known_revisions.clear();
            }
        }

        if let Some(db_revision) = self.get_db_slot_revision(slot).await? {
            return Ok(db_revision);
        }

        Err(EVMRuntimeError::ReadEvmRevisionError{ err: format!("Revision for slot {} not found", slot) },)
    }

    pub async fn run_command_with_slot_revision(
        &self,
        command: Vec<&str>,
        stdin_data: Option<Vec<u8>>,
        slot: u64,
        tout: &std::time::Duration,
    ) -> Result<ExecResult, EVMRuntimeError> {
        let revision = self.get_slot_revision(slot).await?;
        self.run_command_with_revision(command, stdin_data, &revision, tout).await
    }

}

#[cfg(test)]
mod test {
    use crate::evm_runtime::KnownRevisions;

    #[test]
    fn test_known_revisions() {
        let mut testee = KnownRevisions::new();
        let rev0 = "Revision-20".to_string();

        assert!(testee.set_slot_revision(20, &rev0).is_ok());
        let res = testee.get_slot_revision(21);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res.is_some());
        let res = res.unwrap();
        assert_eq!(res, rev0);
        assert_eq!(testee.last(), Some(rev0.clone()));

        let res = testee.get_slot_revision(19);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res.is_none());
        assert_eq!(testee.last(), Some(rev0.clone()));

        let rev2 = "Revision-40".to_string();
        assert!(testee.set_slot_revision(40, &rev2).is_ok());
        let res = testee.get_slot_revision(44);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res.is_some());
        let res = res.unwrap();
        assert_eq!(res, rev2);
        assert_eq!(testee.last(), Some(rev2.clone()));

        let rev1 = "Revision-30".to_string();
        assert!(testee.set_slot_revision(30, &rev1).is_ok());
        let res = testee.get_slot_revision(34);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res.is_some());
        let res = res.unwrap();
        assert_eq!(res, rev1);
        assert_eq!(testee.last(), Some(rev2.clone()));

        testee.clear();
        let res = testee.get_slot_revision(30);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res.is_none());

        assert!(testee.last().is_none());
    }
}

