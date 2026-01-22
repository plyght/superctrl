mod automation;
mod cli;
mod computer_use;
mod config;
mod gui;
mod hotkey;
mod ipc;
mod learning;
mod menu_bar;
mod notifications;
mod preferences;
mod screenshot;

use anyhow::{Context, Result};

use cli::Cli;
use config::Config;
use gui::create_shared_state;
use hotkey::EmergencyStop;
use learning::LearningCollector;
use std::sync::{Arc, Mutex};

fn check_macrowhisper_service() {
    use std::process::Command;

    if let Ok(output) = Command::new("macrowhisper")
        .arg("--service-status")
        .output()
    {
        let status_str = String::from_utf8_lossy(&output.stdout);
        if !status_str.contains("Running: Yes") && !status_str.contains("running") {
            tracing::warn!("⚠️  macrowhisper service is not running!");
            tracing::warn!("   Voice command integration requires macrowhisper to be running.");
            tracing::warn!("   Start it with: macrowhisper --start-service");
            tracing::warn!("   Check status: macrowhisper --service-status");
        } else {
            tracing::info!("✅ macrowhisper service is running");
        }
    } else {
        tracing::warn!("⚠️  Could not check macrowhisper service status");
        tracing::warn!("   Voice command integration requires macrowhisper.");
        tracing::warn!("   Install: brew install ognistik/formulae/macrowhisper");
        tracing::warn!("   Start: macrowhisper --start-service");
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse_args();

    if cli.is_status_command() || cli.is_stop_command() || cli.get_execute_command().is_some() {
        let rt = tokio::runtime::Runtime::new()?;
        return rt.block_on(cli::handle_cli_command(&cli));
    }

    if ipc::is_daemon_running() {
        anyhow::bail!("Daemon is already running. Use 'superctrl status' to check or 'superctrl stop' to stop it.");
    }

    check_macrowhisper_service();

    let config = Config::load()?;

    let state = create_shared_state();

    let learning_stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let learning_collector = if config.learning_enabled {
        Some(Arc::new(Mutex::new(LearningCollector::with_path(
            config.learning_db_path.clone(),
            learning_stop_flag.clone(),
        )?)))
    } else {
        tracing::info!("Learning feature is disabled via SUPERCTRL_LEARNING_ENABLED");
        None
    };

    let rt = tokio::runtime::Runtime::new()?;
    let _rt_guard = rt.enter();

    let ipc_state = state.clone();
    let api_key = config.api_key.clone();
    let learning_collector_for_ipc = learning_collector.clone();
    let system_prompt_path = config.system_prompt_path.clone();
    rt.spawn(async move {
        match ipc::IpcServer::new().await {
            Ok(ipc_server) => {
                tracing::info!("IPC server started");
                loop {
                    match ipc_server.accept_connection().await {
                        Ok(stream) => {
                            let state_clone = ipc_state.clone();
                            let api_key_clone = api_key.clone();
                            let learning_collector_clone = learning_collector_for_ipc.clone();
                            let system_prompt_path_clone = system_prompt_path.clone();
                            tokio::spawn(async move {
                                let state_for_execute = state_clone.clone();
                                let api_key_for_execute = api_key_clone.clone();
                                let on_execute = move |command: String| {
                                    tracing::info!("Received execute command via IPC: {}", command);
                                    let _ = notifications::notify_command_received(&command);
                                    let mut gui_state = state_for_execute.lock().unwrap();
                                    gui_state
                                        .update_status(gui::AppState::Working(command.clone()));
                                    let action = gui::ActionRecord::new(
                                        "voice_command".to_string(),
                                        command.clone(),
                                    );
                                    gui_state.add_action(action);
                                    drop(gui_state);

                                    let state_for_task = state_for_execute.clone();
                                    let api_key_for_task = api_key_for_execute.clone();
                                    std::thread::spawn(move || {
                                        let rt = tokio::runtime::Runtime::new().unwrap();
                                        rt.block_on(async {
                                            let stop_flag = {
                                                let gui_state = state_for_task.lock().unwrap();
                                                gui_state.get_stop_flag()
                                            };

                                            let mut agent = match computer_use::ComputerUseAgent::new(
                                                api_key_for_task,
                                                stop_flag,
                                            ) {
                                                Ok(agent) => agent,
                                                Err(e) => {
                                                    tracing::error!("Failed to create agent: {}", e);
                                                    let mut gui_state = state_for_task.lock().unwrap();
                                                    gui_state.update_status(gui::AppState::Error(
                                                        format!("Failed to create agent: {}", e),
                                                    ));
                                                    return;
                                                }
                                            };

                                            match agent.execute_command(&command).await {
                                                Ok(result) => {
                                                    tracing::info!("Command completed: {}", result);
                                                    let _ = notifications::notify_command_completed(&command);
                                                    let mut gui_state = state_for_task.lock().unwrap();
                                                    gui_state.update_status(gui::AppState::Idle);
                                                }
                                                Err(e) => {
                                                    tracing::error!("Command failed: {}", e);
                                                    let _ = notifications::notify_command_failed(&command, &e.to_string());
                                                    let mut gui_state = state_for_task.lock().unwrap();
                                                    gui_state.update_status(gui::AppState::Error(
                                                        format!("Command failed: {}", e),
                                                    ));
                                                }
                                            }
                                        });
                                    });

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

                                let learning_collector_for_start = learning_collector_clone.clone();
                                let on_learn_start = move || {
                                    tracing::info!("Received learn start command via IPC");
                                    match learning_collector_for_start.as_ref() {
                                        Some(collector) => {
                                            let mut c = collector.lock().unwrap();
                                            c.start()
                                        }
                                        None => anyhow::bail!("Learning feature is disabled"),
                                    }
                                };

                                let learning_collector_for_stop = learning_collector_clone.clone();
                                let on_learn_stop = move || {
                                    tracing::info!("Received learn stop command via IPC");
                                    match learning_collector_for_stop.as_ref() {
                                        Some(collector) => {
                                            let mut c = collector.lock().unwrap();
                                            c.stop()
                                        }
                                        None => anyhow::bail!("Learning feature is disabled"),
                                    }
                                };

                                let learning_collector_for_status = learning_collector_clone.clone();
                                let on_learn_status = move || {
                                    tracing::info!("Received learn status command via IPC");
                                    match learning_collector_for_status.as_ref() {
                                        Some(collector) => {
                                            let c = collector.lock().unwrap();
                                            let state = c.state();
                                            let is_active = state.is_active();
                                            let status_text = if is_active {
                                                "Learning is active"
                                            } else {
                                                "Learning is stopped"
                                            };
                                            Ok(status_text.to_string())
                                        }
                                        None => Ok("Learning feature is disabled".to_string()),
                                    }
                                };

                                let learning_collector_for_finish = learning_collector_clone.clone();
                                let api_key_for_finish = api_key_clone.clone();
                                let system_prompt_path_for_finish = system_prompt_path_clone.clone();
                                let on_learn_finish = async move {
                                    tracing::info!("Received learn finish command via IPC");
                                    match learning_collector_for_finish.as_ref() {
                                        Some(collector) => {
                                            let api_key = api_key_for_finish.clone();
                                            let path = system_prompt_path_for_finish.clone();
                                            let database = {
                                                let c = collector.lock().unwrap();
                                                c.database().clone()
                                            };
                                            let summary = {
                                                let db = database.lock().unwrap();
                                                db.aggregate_data()
                                            }?;

                                            let prompt_text = format!(
                                                "Analyze this workflow data and create a system prompt (max 2000 words) describing this user's working style, applications, patterns, and habits. Format as a system prompt for an AI assistant.\n\n{}",
                                                summary
                                            );

                                            let client = reqwest::Client::builder()
                                                .timeout(std::time::Duration::from_secs(30))
                                                .build()
                                                .context("Failed to create HTTP client")?;

                                            let request_body = serde_json::json!({
                                                "model": "claude-sonnet-4-20250514",
                                                "max_tokens": 4096,
                                                "messages": [{
                                                    "role": "user",
                                                    "content": prompt_text
                                                }]
                                            });

                                            let response = client
                                                .post("https://api.anthropic.com/v1/messages")
                                                .header("x-api-key", &api_key)
                                                .header("anthropic-version", "2023-06-01")
                                                .header("content-type", "application/json")
                                                .json(&request_body)
                                                .send()
                                                .await
                                                .context("Failed to call Anthropic API")?;

                                            if !response.status().is_success() {
                                                let status = response.status();
                                                let error_text = response.text().await.unwrap_or_default();
                                                anyhow::bail!("Anthropic API returned error: {} - {}", status, error_text);
                                            }

                                            let response_json: serde_json::Value = response
                                                .json()
                                                .await
                                                .context("Failed to parse Anthropic response")?;

                                            let generated_text = response_json["content"]
                                                .as_array()
                                                .and_then(|arr| arr.first())
                                                .and_then(|block| block["text"].as_str())
                                                .context("Failed to extract text from Anthropic response")?;

                                            if let Some(parent) = path.parent() {
                                                std::fs::create_dir_all(parent).context("Failed to create system prompt directory")?;
                                            }

                                            std::fs::write(&path, generated_text)
                                                .with_context(|| format!("Failed to write system prompt to {:?}", path))?;

                                            tracing::info!("System prompt saved to {:?}", path);

                                            Ok(())
                                        }
                                        None => anyhow::bail!("Learning feature is disabled"),
                                    }
                                };

                                let learning_collector_for_clear = learning_collector_clone.clone();
                                let on_learn_clear = move || {
                                    tracing::info!("Received learn clear command via IPC");
                                    match learning_collector_for_clear.as_ref() {
                                        Some(collector) => {
                                            let mut c = collector.lock().unwrap();
                                            c.clear_database()
                                        }
                                        None => anyhow::bail!("Learning feature is disabled"),
                                    }
                                };

                                if let Err(e) =
                                    ipc::IpcServer::handle_connection(
                                        stream,
                                        on_execute,
                                        on_stop,
                                        on_learn_start,
                                        on_learn_stop,
                                        on_learn_status,
                                        on_learn_finish,
                                        on_learn_clear,
                                    )
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
                let _ = notifications::notify_emergency_stop();
                let mut gui_state = state_for_listener.lock().unwrap();
                gui_state.update_status(gui::AppState::Idle);
                drop(gui_state);

                stop_flag.store(false, std::sync::atomic::Ordering::Release);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        });
    }

    std::thread::spawn(move || {
        rt.block_on(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        })
    });

    menu_bar::run_menu_bar_loop(state)
}
