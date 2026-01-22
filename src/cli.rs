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
    Learn {
        #[command(subcommand)]
        action: LearnAction,
    },
}

#[derive(Subcommand)]
pub enum LearnAction {
    Start,
    Stop,
    Status,
    Finish,
    Clear,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub fn is_status_command(&self) -> bool {
        matches!(self.command, Some(Commands::Status))
    }

    pub fn is_stop_command(&self) -> bool {
        matches!(self.command, Some(Commands::Stop))
    }

    pub fn is_learn_command(&self) -> bool {
        matches!(self.command, Some(Commands::Learn { .. }))
    }

    pub fn get_learn_action(&self) -> Option<&LearnAction> {
        if let Some(Commands::Learn { action }) = &self.command {
            Some(action)
        } else {
            None
        }
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
        Some(Commands::Learn { action }) => match action {
            LearnAction::Start => {
                crate::ipc::send_learn_start_command().await?;
                println!("Learning mode started");
                Ok(())
            }
            LearnAction::Stop => {
                crate::ipc::send_learn_stop_command().await?;
                println!("Learning mode stopped");
                Ok(())
            }
            LearnAction::Status => {
                let status = crate::ipc::send_learn_status_command().await?;
                println!("{}", status);
                Ok(())
            }
            LearnAction::Finish => {
                crate::ipc::send_learn_finish_command().await?;
                println!("Learning session finished");
                Ok(())
            }
            LearnAction::Clear => {
                crate::ipc::send_learn_clear_command().await?;
                println!("Learning history cleared");
                Ok(())
            }
        },
        None => Ok(()),
    }
}
