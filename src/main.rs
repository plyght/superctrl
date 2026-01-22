mod app;
mod automation;
mod cli;
mod computer_use;
mod config;
mod gui;
mod gui_integration;
mod hotkey;
mod ipc;
mod menu_bar;
mod preferences;
mod screenshot;

use anyhow::Result;
use tracing_subscriber;

use cli::Cli;
use config::Config;
use gui::create_shared_state;
use hotkey::EmergencyStop;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse_args();

    if cli.is_status_command() || cli.is_stop_command() || cli.get_execute_command().is_some() {
        return cli::handle_cli_command(&cli).await;
    }

    if ipc::is_daemon_running() {
        anyhow::bail!("Daemon is already running. Use 'superctrl status' to check or 'superctrl stop' to stop it.");
    }

    let _config = Config::load()?;

    let state = create_shared_state();

    let ipc_state = state.clone();
    tokio::spawn(async move {
        match ipc::IpcServer::new().await {
            Ok(ipc_server) => {
                tracing::info!("IPC server started");
                loop {
                    match ipc_server.accept_connection().await {
                        Ok(stream) => {
                            let state_clone = ipc_state.clone();
                            tokio::spawn(async move {
                                let state_for_execute = state_clone.clone();
                                let on_execute = move |command: String| {
                                    tracing::info!("Received execute command via IPC: {}", command);
                                    let mut gui_state = state_for_execute.lock().unwrap();
                                    gui_state
                                        .update_status(gui::AppState::Working(command.clone()));
                                    let action = gui::ActionRecord::new(
                                        "voice_command".to_string(),
                                        command.clone(),
                                    );
                                    gui_state.add_action(action);
                                    Ok(())
                                };

                                let state_clone_for_stop = state_clone.clone();
                                let on_stop = move || {
                                    tracing::info!("Received stop command via IPC");
                                    let gui_state = state_clone_for_stop.lock().unwrap();
                                    gui_state.trigger_stop();
                                    drop(gui_state);

                                    let mut gui_state = state_clone_for_stop.lock().unwrap();
                                    gui_state.update_status(gui::AppState::Idle);
                                    Ok(())
                                };

                                if let Err(e) =
                                    ipc::IpcServer::handle_connection(stream, on_execute, on_stop)
                                        .await
                                {
                                    tracing::error!("Error handling IPC connection: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Error accepting IPC connection: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to start IPC server: {}", e);
            }
        }
    });

    let emergency_stop = match EmergencyStop::new() {
        Ok(es) => {
            if let Err(e) = es.register_hotkey() {
                tracing::warn!("Failed to register emergency stop hotkey: {}", e);
                tracing::warn!(
                    "  The app will still work, but emergency stop (⌘⇧⎋) won't be available."
                );
                None
            } else {
                Some(es)
            }
        }
        Err(e) => {
            tracing::warn!("Failed to initialize emergency stop: {}", e);
            tracing::warn!(
                "  The app will still work, but emergency stop (⌘⇧⎋) won't be available."
            );
            None
        }
    };

    if let Some(ref es) = emergency_stop {
        let stop_flag = es.get_stop_flag();
        EmergencyStop::start_listener(stop_flag.clone());

        let state_for_listener = state.clone();
        std::thread::spawn(move || loop {
            if stop_flag.load(std::sync::atomic::Ordering::Acquire) {
                tracing::info!("🛑 Emergency stop triggered via hotkey");
                let mut gui_state = state_for_listener.lock().unwrap();
                gui_state.update_status(gui::AppState::Idle);
                drop(gui_state);

                stop_flag.store(false, std::sync::atomic::Ordering::Release);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        });
    }

    std::thread::spawn({
        let state = state.clone();
        move || {
            if let Err(e) = menu_bar::run_menu_bar_loop(state) {
                tracing::error!("Menu bar error: {}", e);
            }
        }
    });

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
