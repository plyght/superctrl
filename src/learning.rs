use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    KeyPress {
        key: String,
        timestamp: DateTime<Local>,
        app_name: Option<String>,
    },
    AppSwitch {
        from_app: Option<String>,
        to_app: String,
        timestamp: DateTime<Local>,
    },
    WindowFocus {
        app_name: String,
        window_title: Option<String>,
        timestamp: DateTime<Local>,
    },
    ClipboardChange {
        content_type: String,
        content_preview: String,
        timestamp: DateTime<Local>,
        source_app: Option<String>,
    },
}

impl Event {
    pub fn timestamp(&self) -> DateTime<Local> {
        match self {
            Event::KeyPress { timestamp, .. } => *timestamp,
            Event::AppSwitch { timestamp, .. } => *timestamp,
            Event::WindowFocus { timestamp, .. } => *timestamp,
            Event::ClipboardChange { timestamp, .. } => *timestamp,
        }
    }

    pub fn event_type(&self) -> &str {
        match self {
            Event::KeyPress { .. } => "keypress",
            Event::AppSwitch { .. } => "app_switch",
            Event::WindowFocus { .. } => "window_focus",
            Event::ClipboardChange { .. } => "clipboard_change",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningState {
    Active,
    Stopped,
}

impl LearningState {
    pub fn is_active(&self) -> bool {
        matches!(self, LearningState::Active)
    }
}

pub struct LearningDatabase {
    conn: Connection,
}

impl LearningDatabase {
    pub fn new(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open database at {:?}", path))?;

        let mut db = Self { conn };
        db.init_schema()?;

        Ok(db)
    }

    pub fn init_schema(&mut self) -> Result<()> {
        self.conn.pragma_update(None, "journal_mode", "WAL")?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                data_json TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                active INTEGER NOT NULL DEFAULT 1
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS app_usage (
                app_name TEXT PRIMARY KEY,
                bundle_id TEXT,
                total_time INTEGER NOT NULL DEFAULT 0,
                switch_count INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS key_patterns (
                key_combination TEXT PRIMARY KEY,
                count INTEGER NOT NULL DEFAULT 0,
                context TEXT
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)",
            [],
        )?;

        Ok(())
    }

    pub fn insert_event(&mut self, event: &Event) -> Result<()> {
        let timestamp = event.timestamp().timestamp();
        let event_type = event.event_type();
        let data_json =
            serde_json::to_string(event).context("Failed to serialize event to JSON")?;

        self.conn.execute(
            "INSERT INTO events (timestamp, event_type, data_json) VALUES (?1, ?2, ?3)",
            params![timestamp, event_type, data_json],
        )?;

        Ok(())
    }

    pub fn get_session_stats(&self) -> Result<SessionStats> {
        let total_events: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .unwrap_or(0);

        let keypress_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE event_type = 'keypress'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let app_switch_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE event_type = 'app_switch'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let clipboard_change_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE event_type = 'clipboard_change'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let active_session_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE active = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(SessionStats {
            total_events,
            keypress_count,
            app_switch_count,
            clipboard_change_count,
            active_session_count,
        })
    }

    pub fn aggregate_data(&self) -> Result<String> {
        let stats = self.get_session_stats()?;

        let mut summary = String::new();
        summary.push_str("=== Learning Data Summary ===\n\n");

        summary.push_str(&format!("Total Events: {}\n", stats.total_events));
        summary.push_str(&format!("  - Key Presses: {}\n", stats.keypress_count));
        summary.push_str(&format!("  - App Switches: {}\n", stats.app_switch_count));
        summary.push_str(&format!(
            "  - Clipboard Changes: {}\n",
            stats.clipboard_change_count
        ));
        summary.push_str(&format!(
            "Active Sessions: {}\n\n",
            stats.active_session_count
        ));

        let mut stmt = self.conn.prepare(
            "SELECT app_name, total_time, switch_count FROM app_usage ORDER BY total_time DESC LIMIT 10"
        )?;

        let app_usage = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;

        summary.push_str("Top Applications:\n");
        for (idx, app) in app_usage.enumerate() {
            if let Ok((name, time, switches)) = app {
                summary.push_str(&format!(
                    "  {}. {} - {} seconds, {} switches\n",
                    idx + 1,
                    name,
                    time,
                    switches
                ));
            }
        }

        let mut stmt = self.conn.prepare(
            "SELECT key_combination, count FROM key_patterns ORDER BY count DESC LIMIT 10",
        )?;

        let key_patterns = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        summary.push_str("\nTop Key Patterns:\n");
        for (idx, pattern) in key_patterns.enumerate() {
            if let Ok((combo, count)) = pattern {
                summary.push_str(&format!(
                    "  {}. {} - {} occurrences\n",
                    idx + 1,
                    combo,
                    count
                ));
            }
        }

        Ok(summary)
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_events: i64,
    pub keypress_count: i64,
    pub app_switch_count: i64,
    pub clipboard_change_count: i64,
    pub active_session_count: i64,
}

pub struct LearningCollector {
    database: Arc<Mutex<LearningDatabase>>,
    state: LearningState,
    stop_flag: Arc<AtomicBool>,
    disable_clipboard_monitoring: bool,
    keyboard_thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl LearningCollector {
    pub fn new(database: LearningDatabase, stop_flag: Arc<AtomicBool>) -> Self {
        Self {
            database: Arc::new(Mutex::new(database)),
            state: LearningState::Stopped,
            stop_flag,
            disable_clipboard_monitoring: false,
            keyboard_thread_handle: None,
        }
    }

    pub fn with_path(path: PathBuf, stop_flag: Arc<AtomicBool>) -> Result<Self> {
        Self::with_path_and_clipboard_setting(path, stop_flag, false)
    }

    pub fn with_path_and_clipboard_setting(
        path: PathBuf,
        stop_flag: Arc<AtomicBool>,
        disable_clipboard_monitoring: bool,
    ) -> Result<Self> {
        let database = LearningDatabase::new(path)?;
        Ok(Self {
            database: Arc::new(Mutex::new(database)),
            state: LearningState::Stopped,
            stop_flag,
            disable_clipboard_monitoring,
            keyboard_thread_handle: None,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        if self.state.is_active() {
            anyhow::bail!("Learning collector is already active");
        }

        if self.keyboard_thread_handle.is_some() {
            anyhow::bail!("Keyboard monitor thread already exists");
        }

        self.stop_flag.store(false, Ordering::Release);
        self.state = LearningState::Active;

        let db_for_keyboard = self.database.clone();
        let stop_flag_for_keyboard = self.stop_flag.clone();
        let keyboard_handle = std::thread::spawn(move || {
            Self::keyboard_monitor(db_for_keyboard, stop_flag_for_keyboard);
        });
        self.keyboard_thread_handle = Some(keyboard_handle);

        if !self.disable_clipboard_monitoring {
            tracing::warn!("⚠️  Clipboard monitoring is ENABLED. Clipboard content previews will be stored in the learning database.");
            tracing::warn!("   To disable for privacy, set SUPERCTRL_DISABLE_CLIPBOARD_MONITORING=true");
            let db_for_clipboard = self.database.clone();
            let stop_flag_for_clipboard = self.stop_flag.clone();
            std::thread::spawn(move || {
                Self::clipboard_monitor(db_for_clipboard, stop_flag_for_clipboard);
            });
        } else {
            tracing::info!("Clipboard monitoring is disabled for privacy");
        }

        Ok(())
    }

    fn keyboard_monitor(database: Arc<Mutex<LearningDatabase>>, stop_flag: Arc<AtomicBool>) {
        let modifiers = Arc::new(Mutex::new(ModifierState::default()));

        let modifiers_for_callback = modifiers.clone();
        let database_for_callback = database.clone();
        let stop_flag_for_callback = stop_flag.clone();

        let callback = move |event: rdev::Event| {
            if stop_flag_for_callback.load(Ordering::Acquire) {
                return;
            }

            match event.event_type {
                rdev::EventType::KeyPress(key) => {
                    if let Ok(mut mods) = modifiers_for_callback.lock() {
                        mods.update_key_down(key);

                        if let Some(key_combo) = mods.get_combination(&key) {
                            let event = Event::KeyPress {
                                key: key_combo,
                                timestamp: Local::now(),
                                app_name: None,
                            };

                            if let Ok(mut db) = database_for_callback.lock() {
                                if let Err(e) = db.insert_event(&event) {
                                    tracing::error!("Failed to insert keyboard event: {}", e);
                                }
                            }
                        }
                    }
                }
                rdev::EventType::KeyRelease(key) => {
                    if let Ok(mut mods) = modifiers_for_callback.lock() {
                        mods.update_key_up(key);
                    }
                }
                _ => {}
            }
        };

        if let Err(e) = rdev::listen(callback) {
            tracing::error!("Keyboard monitoring error: {:?}", e);
        }
        tracing::warn!("Keyboard monitor thread exited (rdev::listen() terminated)");
    }

    fn clipboard_monitor(database: Arc<Mutex<LearningDatabase>>, stop_flag: Arc<AtomicBool>) {
        let mut ctx = match arboard::Clipboard::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                tracing::error!("Failed to initialize clipboard context: {}", e);
                return;
            }
        };

        let mut last_content = String::new();

        while !stop_flag.load(Ordering::Acquire) {
            match ctx.get_text() {
                Ok(content) => {
                    if content != last_content && !content.is_empty() {
                        let content_type = "text";
                        let char_count = content.chars().count();
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        content.hash(&mut hasher);
                        let hash = hasher.finish();

                        let content_preview = format!("[REDACTED] ({} chars, hash: {:x})", char_count, hash);

                        let event = Event::ClipboardChange {
                            content_type: content_type.to_string(),
                            content_preview,
                            timestamp: Local::now(),
                            source_app: None,
                        };

                        if let Ok(mut db) = database.lock() {
                            if let Err(e) = db.insert_event(&event) {
                                tracing::error!("Failed to insert clipboard event: {}", e);
                            }
                        }

                        last_content = content;
                    }
                }
                Err(arboard::Error::ContentNotAvailable) => {
                }
                Err(e) => {
                    tracing::debug!("Clipboard read error (may be non-text): {:?}", e);
                }
            }

            std::thread::sleep(Duration::from_secs(2));
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        if !self.state.is_active() {
            anyhow::bail!("Learning collector is not active");
        }

        self.stop_flag.store(true, Ordering::Release);
        self.state = LearningState::Stopped;

        if let Some(handle) = self.keyboard_thread_handle.take() {
            if let Err(e) = handle.join() {
                tracing::error!("Error joining keyboard monitor thread: {:?}", e);
            }
        }

        Ok(())
    }

    pub fn state(&self) -> LearningState {
        self.state
    }

    pub fn database(&self) -> Arc<Mutex<LearningDatabase>> {
        self.database.clone()
    }

    pub fn is_stopped(&self) -> bool {
        self.stop_flag.load(Ordering::Acquire)
    }

    pub async fn generate_system_prompt(&self, api_key: &str, system_prompt_path: PathBuf) -> Result<String> {
        let summary = {
            let db = self.database.lock().unwrap();
            db.aggregate_data()?
        };
        
        let prompt_text = format!(
            "Analyze this workflow data and create a system prompt (max 2000 words) describing this user's working style, applications, patterns, and habits. Format as a system prompt for an AI assistant.\n\n{}",
            summary
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
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
            .header("x-api-key", api_key)
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

        if let Some(parent) = system_prompt_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create system prompt directory")?;
        }

        std::fs::write(&system_prompt_path, generated_text)
            .with_context(|| format!("Failed to write system prompt to {:?}", system_prompt_path))?;

        tracing::info!("System prompt saved to {:?}", system_prompt_path);

        Ok(generated_text.to_string())
    }

    pub fn clear_database(&mut self) -> Result<()> {
        let mut db = self.database.lock().unwrap();
        let conn = db.connection_mut();
        
        conn.execute("DELETE FROM events", [])?;
        conn.execute("DELETE FROM sessions", [])?;
        conn.execute("DELETE FROM app_usage", [])?;
        conn.execute("DELETE FROM key_patterns", [])?;
        
        tracing::info!("Learning database cleared");
        
        Ok(())
    }
}

#[derive(Default)]
struct ModifierState {
    ctrl: bool,
    alt: bool,
    meta: bool,
    shift: bool,
}

impl ModifierState {
    fn update_key_down(&mut self, key: rdev::Key) {
        match key {
            rdev::Key::ControlLeft | rdev::Key::ControlRight => self.ctrl = true,
            rdev::Key::Alt | rdev::Key::AltGr => self.alt = true,
            rdev::Key::MetaLeft | rdev::Key::MetaRight => self.meta = true,
            rdev::Key::ShiftLeft | rdev::Key::ShiftRight => self.shift = true,
            _ => {}
        }
    }

    fn update_key_up(&mut self, key: rdev::Key) {
        match key {
            rdev::Key::ControlLeft | rdev::Key::ControlRight => self.ctrl = false,
            rdev::Key::Alt | rdev::Key::AltGr => self.alt = false,
            rdev::Key::MetaLeft | rdev::Key::MetaRight => self.meta = false,
            rdev::Key::ShiftLeft | rdev::Key::ShiftRight => self.shift = false,
            _ => {}
        }
    }

    fn get_combination(&self, key: &rdev::Key) -> Option<String> {
        if !self.ctrl && !self.alt && !self.meta {
            return None;
        }

        let mut parts = Vec::new();

        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.meta {
            parts.push("Cmd");
        }
        if self.shift {
            parts.push("Shift");
        }

        let key_name = format!("{:?}", key);
        parts.push(&key_name);

        Some(parts.join("+"))
    }
}
