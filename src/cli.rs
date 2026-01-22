use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "superctrl")]
#[command(about = "Voice-controlled macOS automation via Computer Use API", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, value_name = "COMMAND")]
    pub execute: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Daemon,
    Status,
    Stop,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub fn is_daemon_mode(&self) -> bool {
        matches!(self.command, Some(Commands::Daemon))
    }

    pub fn is_status_command(&self) -> bool {
        matches!(self.command, Some(Commands::Status))
    }

    pub fn is_stop_command(&self) -> bool {
        matches!(self.command, Some(Commands::Stop))
    }

    pub fn get_execute_command(&self) -> Option<&String> {
        self.execute.as_ref()
    }
}

pub async fn handle_cli_command(cli: &Cli) -> Result<()> {
    if let Some(command_text) = cli.get_execute_command() {
        crate::ipc::send_execute_command(command_text).await?;
        return Ok(());
    }

    match &cli.command {
        Some(Commands::Daemon) => Ok(()),
        Some(Commands::Status) => {
            let status = crate::ipc::send_status_command().await?;
            println!("{}", status);
            Ok(())
        }
        Some(Commands::Stop) => {
            crate::ipc::send_stop_command().await?;
            println!("Emergency stop signal sent");
            Ok(())
        }
        None => Ok(()),
    }
}
