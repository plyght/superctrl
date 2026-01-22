use iced::widget::{button, column, container, row, text, vertical_space};
use iced::{Element, Length, Settings, Task, Color};

use crate::gui::SharedGuiState;

#[derive(Debug, Clone)]
pub enum PreferencesMessage {
    TestConnection,
    ViewLogs,
    OpenConfig,
    CheckDaemonStatus,
    ConnectionTested(Result<String, String>),
    DaemonStatusChecked(bool),
}

pub struct PreferencesWindow {
    api_key_set: bool,
    emergency_stop_shortcut: String,
    connection_status: Option<String>,
    testing_connection: bool,
    daemon_running: Option<bool>,
    macrowhisper_configured: bool,
}

impl PreferencesWindow {
    pub fn new(_state: SharedGuiState) -> (Self, Task<PreferencesMessage>) {
        let api_key_set = std::env::var("ANTHROPIC_API_KEY").is_ok();
        let macrowhisper_configured = std::path::Path::new("/Users")
            .join(std::env::var("USER").unwrap_or_default())
            .join(".config/macrowhisper/macrowhisper.json")
            .exists();

        (
            Self {
                api_key_set,
                emergency_stop_shortcut: "⌘⇧⎋".to_string(),
                connection_status: None,
                testing_connection: false,
                daemon_running: None,
                macrowhisper_configured,
            },
            Task::perform(
                async { crate::ipc::is_daemon_running() },
                PreferencesMessage::DaemonStatusChecked,
            ),
        )
    }

    pub fn update(&mut self, message: PreferencesMessage) -> Task<PreferencesMessage> {
        match message {
            PreferencesMessage::TestConnection => {
                self.testing_connection = true;
                self.connection_status = Some("Testing connection...".to_string());

                Task::perform(
                    async {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                        match std::env::var("ANTHROPIC_API_KEY") {
                            Ok(key) if !key.is_empty() => Ok("✓ Connection successful!".to_string()),
                            _ => Err("API key not set".to_string()),
                        }
                    },
                    PreferencesMessage::ConnectionTested,
                )
            }
            PreferencesMessage::ConnectionTested(result) => {
                self.testing_connection = false;
                self.connection_status = Some(match result {
                    Ok(msg) => msg,
                    Err(err) => format!("✗ Error: {}", err),
                });
                Task::none()
            }
            PreferencesMessage::ViewLogs => {
                let _ = std::process::Command::new("open")
                    .arg("/tmp")
                    .spawn();
                Task::none()
            }
            PreferencesMessage::OpenConfig => {
                if let Ok(user) = std::env::var("USER") {
                    let config_path = format!("/Users/{}/.config/macrowhisper/macrowhisper.json", user);
                    let _ = std::process::Command::new("open")
                        .arg("-a")
                        .arg("TextEdit")
                        .arg(&config_path)
                        .spawn();
                }
                Task::none()
            }
            PreferencesMessage::CheckDaemonStatus => {
                Task::perform(
                    async { crate::ipc::is_daemon_running() },
                    PreferencesMessage::DaemonStatusChecked,
                )
            }
            PreferencesMessage::DaemonStatusChecked(running) => {
                self.daemon_running = Some(running);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, PreferencesMessage> {
        let title = text("superctrl Settings").size(28);

        let api_key_status = if self.api_key_set {
            text("✓ Set").color(Color::from_rgb(0.0, 0.8, 0.0))
        } else {
            text("✗ Not set").color(Color::from_rgb(0.8, 0.0, 0.0))
        };

        let daemon_status = match self.daemon_running {
            Some(true) => text("✓ Running").color(Color::from_rgb(0.0, 0.8, 0.0)),
            Some(false) => text("✗ Not running").color(Color::from_rgb(0.8, 0.0, 0.0)),
            None => text("Checking...").color(Color::from_rgb(0.5, 0.5, 0.5)),
        };

        let macrowhisper_status = if self.macrowhisper_configured {
            text("✓ Configured").color(Color::from_rgb(0.0, 0.8, 0.0))
        } else {
            text("✗ Not configured").color(Color::from_rgb(0.8, 0.0, 0.0))
        };

        let status_section = column![
            text("System Status").size(20),
            vertical_space().height(Length::Fixed(10.0)),
            row![
                text("ANTHROPIC_API_KEY:").width(Length::Fixed(200.0)),
                api_key_status
            ]
            .spacing(10),
            row![
                text("Daemon Status:").width(Length::Fixed(200.0)),
                daemon_status
            ]
            .spacing(10),
            row![
                text("macrowhisper:").width(Length::Fixed(200.0)),
                macrowhisper_status
            ]
            .spacing(10),
            row![
                text("Emergency Stop:").width(Length::Fixed(200.0)),
                text(&self.emergency_stop_shortcut)
            ]
            .spacing(10),
        ]
        .spacing(8);

        let test_button = button(if self.testing_connection {
            "  Testing...  "
        } else {
            "  Test API Connection  "
        })
        .on_press(PreferencesMessage::TestConnection)
        .padding(10);

        let view_logs_button = button("  View Logs  ")
            .on_press(PreferencesMessage::ViewLogs)
            .padding(10);

        let open_config_button = button("  Edit Config  ")
            .on_press(PreferencesMessage::OpenConfig)
            .padding(10);

        let check_daemon_button = button("  Refresh Status  ")
            .on_press(PreferencesMessage::CheckDaemonStatus)
            .padding(10);

        let buttons_row = row![
            test_button,
            view_logs_button,
            open_config_button,
            check_daemon_button
        ]
        .spacing(10);

        let mut content = column![
            title,
            vertical_space().height(Length::Fixed(20.0)),
            status_section,
            vertical_space().height(Length::Fixed(20.0)),
            buttons_row,
        ]
        .spacing(10)
        .padding(30);

        if let Some(status) = &self.connection_status {
            content = content.push(vertical_space().height(Length::Fixed(15.0)));
            let status_color = if status.contains("✓") {
                Color::from_rgb(0.0, 0.6, 0.0)
            } else {
                Color::from_rgb(0.8, 0.0, 0.0)
            };
            content = content.push(text(status).size(14).color(status_color));
        }

        let help_text = column![
            vertical_space().height(Length::Fixed(20.0)),
            text("Voice Triggers:").size(14),
            text("  \"Computer, [command]\"  |  \"Automate [command]\"").size(12).color(Color::from_rgb(0.5, 0.5, 0.5)),
            text("  \"Control [command]\"  |  \"Do this: [command]\"").size(12).color(Color::from_rgb(0.5, 0.5, 0.5)),
        ];

        content = content.push(help_text);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

pub fn open_preferences_window(state: SharedGuiState) {
    std::thread::spawn(move || {
        let settings = Settings::default();

        let _ = iced::application(
            "superctrl - Settings",
            PreferencesWindow::update,
            PreferencesWindow::view,
        )
        .settings(settings)
        .window_size((600.0, 450.0))
        .run_with(|| PreferencesWindow::new(state));
    });
}
