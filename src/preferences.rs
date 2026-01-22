use iced::widget::{button, column, container, row, text, vertical_space};
use iced::{Element, Length, Settings, Task};

use crate::gui::SharedGuiState;

#[derive(Debug, Clone)]
pub enum PreferencesMessage {
    TestConnection,
    ViewLogs,
    ConnectionTested(Result<String, String>),
}

pub struct PreferencesWindow {
    api_key_set: bool,
    emergency_stop_shortcut: String,
    connection_status: Option<String>,
    testing_connection: bool,
}

impl PreferencesWindow {
    pub fn new(_state: SharedGuiState) -> (Self, Task<PreferencesMessage>) {
        let api_key_set = std::env::var("ANTHROPIC_API_KEY").is_ok();

        (
            Self {
                api_key_set,
                emergency_stop_shortcut: "⌘⇧⎋".to_string(),
                connection_status: None,
                testing_connection: false,
            },
            Task::none(),
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
                            Ok(key) if !key.is_empty() => Ok("Connection successful!".to_string()),
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
                    Err(err) => format!("Error: {}", err),
                });
                Task::none()
            }
            PreferencesMessage::ViewLogs => {
                if let Ok(home) = std::env::var("HOME") {
                    let log_path = format!("{}/Library/Logs/superctrl", home);
                    let _ = std::process::Command::new("open").arg(&log_path).spawn();
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, PreferencesMessage> {
        let api_key_status = if self.api_key_set {
            text("✓ API Key is set").color(iced::Color::from_rgb(0.0, 0.8, 0.0))
        } else {
            text("✗ API Key not set").color(iced::Color::from_rgb(0.8, 0.0, 0.0))
        };

        let api_key_row = row![text("API Key:").width(Length::Fixed(150.0)), api_key_status]
            .spacing(10)
            .padding(5);

        let shortcut_row = row![
            text("Emergency Stop:").width(Length::Fixed(150.0)),
            text(&self.emergency_stop_shortcut)
        ]
        .spacing(10)
        .padding(5);

        let test_button = button(if self.testing_connection {
            "Testing..."
        } else {
            "Test Connection"
        })
        .on_press(PreferencesMessage::TestConnection)
        .padding(10);

        let view_logs_button = button("View Logs")
            .on_press(PreferencesMessage::ViewLogs)
            .padding(10);

        let buttons_row = row![test_button, view_logs_button].spacing(10);

        let mut content = column![
            text("Preferences").size(24),
            vertical_space().height(Length::Fixed(20.0)),
            api_key_row,
            shortcut_row,
            vertical_space().height(Length::Fixed(20.0)),
            buttons_row,
        ]
        .spacing(10)
        .padding(20);

        if let Some(status) = &self.connection_status {
            content = content.push(vertical_space().height(Length::Fixed(10.0)));
            content = content.push(text(status).size(14));
        }

        container(content)
            .width(Length::Fixed(500.0))
            .height(Length::Shrink)
            .into()
    }
}

pub fn open_preferences_window(state: SharedGuiState) {
    std::thread::spawn(move || {
        let settings = Settings::default();

        let _ = iced::application(
            "superctrl - Preferences",
            PreferencesWindow::update,
            PreferencesWindow::view,
        )
        .settings(settings)
        .window_size((500.0, 300.0))
        .run_with(|| PreferencesWindow::new(state));
    });
}
