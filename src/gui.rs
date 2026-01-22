use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppState {
    Idle,
    Working(String),
    Error(String),
}

impl AppState {
    pub fn status_text(&self) -> &str {
        match self {
            AppState::Idle => "Idle",
            AppState::Working(_) => "Working...",
            AppState::Error(_) => "Error",
        }
    }

    pub fn icon_symbol(&self) -> &str {
        match self {
            AppState::Idle => "⚪",
            AppState::Working(_) => "🔵",
            AppState::Error(_) => "🔴",
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            AppState::Working(task) => Some(task),
            AppState::Error(err) => Some(err),
            AppState::Idle => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub timestamp: DateTime<Local>,
    pub command: String,
    pub description: String,
}

impl ActionRecord {
    pub fn new(command: String, description: String) -> Self {
        Self {
            timestamp: Local::now(),
            command,
            description,
        }
    }

    pub fn format(&self) -> String {
        format!(
            "{} | {} | {}",
            self.timestamp.format("%H:%M:%S"),
            self.command,
            self.description
        )
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub api_key_set: bool,
    pub emergency_stop_shortcut: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key_set: std::env::var("OPENAI_API_KEY").is_ok(),
            emergency_stop_shortcut: "⌘⇧⎋".to_string(),
        }
    }
}

pub struct GuiState {
    pub app_state: AppState,
    pub action_history: Vec<ActionRecord>,
    pub settings: Settings,
    pub max_history: usize,
    pub stop_flag: Arc<AtomicBool>,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            app_state: AppState::Idle,
            action_history: Vec::new(),
            settings: Settings::default(),
            max_history: 5,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl GuiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_status(&mut self, state: AppState) {
        self.app_state = state;
    }

    pub fn add_action(&mut self, action: ActionRecord) {
        self.action_history.push(action);
        if self.action_history.len() > self.max_history {
            self.action_history.remove(0);
        }
    }

    pub fn clear_actions(&mut self) {
        self.action_history.clear();
    }

    pub fn get_recent_actions(&self) -> Vec<String> {
        self.action_history
            .iter()
            .rev()
            .take(5)
            .map(|a| a.format())
            .collect()
    }

    pub fn trigger_stop(&self) {
        self.stop_flag.store(true, Ordering::Release);
        tracing::info!("Emergency stop flag set");
    }

    pub fn reset_stop(&self) {
        self.stop_flag.store(false, Ordering::Release);
    }

    pub fn get_stop_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop_flag)
    }
}

pub type SharedGuiState = Arc<Mutex<GuiState>>;

pub fn create_shared_state() -> SharedGuiState {
    Arc::new(Mutex::new(GuiState::new()))
}
