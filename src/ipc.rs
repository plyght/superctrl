use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

const SOCKET_PATH: &str = "/tmp/superctrl.sock";

#[derive(Debug, Serialize, Deserialize)]
pub enum IpcCommand {
    Execute { command: String },
    Status,
    Stop,
    LearnStart,
    LearnStop,
    LearnStatus,
    LearnFinish,
    LearnClear,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub success: bool,
    pub message: String,
}

impl IpcResponse {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
        }
    }
}

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub async fn new() -> Result<Self> {
        let socket_path = Path::new(SOCKET_PATH);

        if socket_path.exists() {
            std::fs::remove_file(socket_path).context("Failed to remove existing socket file")?;
        }

        let listener = UnixListener::bind(socket_path).context("Failed to bind Unix socket")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(socket_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(socket_path, perms)?;
        }

        tracing::info!("IPC server listening on {}", SOCKET_PATH);

        Ok(Self { listener })
    }

    pub async fn accept_connection(&self) -> Result<UnixStream> {
        let (stream, _addr) = self
            .listener
            .accept()
            .await
            .context("Failed to accept connection")?;
        Ok(stream)
    }

    pub async fn handle_connection(
        mut stream: UnixStream,
        on_execute: impl Fn(String) -> Result<()>,
        on_stop: impl Fn() -> Result<()>,
        on_learn_start: impl Fn() -> Result<()>,
        on_learn_stop: impl Fn() -> Result<()>,
        on_learn_finish: impl Fn() -> Result<()>,
        on_learn_clear: impl Fn() -> Result<()>,
    ) -> Result<()> {
        let mut buffer = vec![0u8; 4096];
        let n = stream.read(&mut buffer).await?;

        if n == 0 {
            return Ok(());
        }

        let request = String::from_utf8_lossy(&buffer[..n]);
        let response = Self::process_command(&request, on_execute, on_stop, on_learn_start, on_learn_stop, on_learn_finish, on_learn_clear);

        let response_json = serde_json::to_string(&response)?;
        stream.write_all(response_json.as_bytes()).await?;
        stream.flush().await?;

        Ok(())
    }

    fn process_command(
        request: &str,
        on_execute: impl Fn(String) -> Result<()>,
        on_stop: impl Fn() -> Result<()>,
        on_learn_start: impl Fn() -> Result<()>,
        on_learn_stop: impl Fn() -> Result<()>,
        on_learn_finish: impl Fn() -> Result<()>,
        on_learn_clear: impl Fn() -> Result<()>,
    ) -> IpcResponse {
        let command: Result<IpcCommand, _> = serde_json::from_str(request);

        match command {
            Ok(IpcCommand::Execute { command }) => match on_execute(command) {
                Ok(_) => IpcResponse::success("Command execution started"),
                Err(e) => IpcResponse::error(format!("Failed to execute command: {}", e)),
            },
            Ok(IpcCommand::Status) => IpcResponse::success("Daemon is running"),
            Ok(IpcCommand::Stop) => match on_stop() {
                Ok(_) => IpcResponse::success("Emergency stop triggered"),
                Err(e) => IpcResponse::error(format!("Failed to stop: {}", e)),
            },
            Ok(IpcCommand::LearnStart) => match on_learn_start() {
                Ok(_) => IpcResponse::success("Learning mode started"),
                Err(e) => IpcResponse::error(format!("Failed to start learning: {}", e)),
            },
            Ok(IpcCommand::LearnStop) => match on_learn_stop() {
                Ok(_) => IpcResponse::success("Learning mode stopped"),
                Err(e) => IpcResponse::error(format!("Failed to stop learning: {}", e)),
            },
            Ok(IpcCommand::LearnStatus) => IpcResponse::success("Learning status: Not implemented yet"),
            Ok(IpcCommand::LearnFinish) => match on_learn_finish() {
                Ok(_) => IpcResponse::success("Learning session finished"),
                Err(e) => IpcResponse::error(format!("Failed to finish learning: {}", e)),
            },
            Ok(IpcCommand::LearnClear) => match on_learn_clear() {
                Ok(_) => IpcResponse::success("Learning history cleared"),
                Err(e) => IpcResponse::error(format!("Failed to clear learning: {}", e)),
            },
            Err(e) => IpcResponse::error(format!("Invalid command: {}", e)),
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(SOCKET_PATH);
    }
}

pub async fn send_execute_command(command: &str) -> Result<()> {
    let ipc_command = IpcCommand::Execute {
        command: command.to_string(),
    };
    let response = send_command(&ipc_command).await?;

    if response.success {
        println!("{}", response.message);
        Ok(())
    } else {
        anyhow::bail!("{}", response.message)
    }
}

pub async fn send_status_command() -> Result<String> {
    let ipc_command = IpcCommand::Status;
    let response = send_command(&ipc_command).await?;

    if response.success {
        Ok(response.message)
    } else {
        anyhow::bail!("{}", response.message)
    }
}

pub async fn send_stop_command() -> Result<()> {
    let ipc_command = IpcCommand::Stop;
    let response = send_command(&ipc_command).await?;

    if response.success {
        Ok(())
    } else {
        anyhow::bail!("{}", response.message)
    }
}

pub async fn send_learn_start_command() -> Result<()> {
    let ipc_command = IpcCommand::LearnStart;
    let response = send_command(&ipc_command).await?;

    if response.success {
        Ok(())
    } else {
        anyhow::bail!("{}", response.message)
    }
}

pub async fn send_learn_stop_command() -> Result<()> {
    let ipc_command = IpcCommand::LearnStop;
    let response = send_command(&ipc_command).await?;

    if response.success {
        Ok(())
    } else {
        anyhow::bail!("{}", response.message)
    }
}

pub async fn send_learn_status_command() -> Result<String> {
    let ipc_command = IpcCommand::LearnStatus;
    let response = send_command(&ipc_command).await?;

    if response.success {
        Ok(response.message)
    } else {
        anyhow::bail!("{}", response.message)
    }
}

pub async fn send_learn_finish_command() -> Result<()> {
    let ipc_command = IpcCommand::LearnFinish;
    let response = send_command(&ipc_command).await?;

    if response.success {
        Ok(())
    } else {
        anyhow::bail!("{}", response.message)
    }
}

pub async fn send_learn_clear_command() -> Result<()> {
    let ipc_command = IpcCommand::LearnClear;
    let response = send_command(&ipc_command).await?;

    if response.success {
        Ok(())
    } else {
        anyhow::bail!("{}", response.message)
    }
}

async fn send_command(command: &IpcCommand) -> Result<IpcResponse> {
    let mut stream = UnixStream::connect(SOCKET_PATH)
        .await
        .context("Failed to connect to daemon. Is superctrl daemon running?")?;

    let command_json = serde_json::to_string(command)?;
    stream.write_all(command_json.as_bytes()).await?;
    stream.flush().await?;

    let mut buffer = vec![0u8; 4096];
    let n = stream.read(&mut buffer).await?;

    let response: IpcResponse =
        serde_json::from_slice(&buffer[..n]).context("Failed to parse response from daemon")?;

    Ok(response)
}

pub fn is_daemon_running() -> bool {
    if !Path::new(SOCKET_PATH).exists() {
        return false;
    }

    let rt = tokio::runtime::Runtime::new().ok();
    if let Some(rt) = rt {
        rt.block_on(async {
            UnixStream::connect(SOCKET_PATH).await.is_ok()
        })
    } else {
        false
    }
}
