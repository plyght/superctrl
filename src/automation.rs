use anyhow::{Context, Result};
use enigo::{
    Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings as EnigoSettings,
};
use std::thread;
use std::time::Duration;

pub struct MacAutomation {
    enigo: Enigo,
}

#[derive(Debug, Clone)]
pub enum Action {
    Click {
        x: i32,
        y: i32,
        button: MouseButton,
    },
    Type {
        text: String,
    },
    Keypress {
        keys: Vec<String>,
    },
    Scroll {
        x: i32,
        y: i32,
        scroll_x: i32,
        scroll_y: i32,
    },
    Wait {
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MacAutomation {
    pub fn new() -> Result<Self> {
        let enigo = Enigo::new(&EnigoSettings::default()).context("Failed to initialize Enigo")?;
        Ok(Self { enigo })
    }

    pub fn execute_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Click { x, y, button } => self.click(x, y, button),
            Action::Type { text } => self.type_text(&text),
            Action::Keypress { keys } => self.keypress(&keys),
            Action::Scroll {
                x,
                y,
                scroll_x,
                scroll_y,
            } => self.scroll(x, y, scroll_x, scroll_y),
            Action::Wait { duration_ms } => self.wait(duration_ms),
        }
    }

    fn click(&mut self, x: i32, y: i32, button: MouseButton) -> Result<()> {
        self.enigo
            .move_mouse(x, y, Coordinate::Abs)
            .context("Failed to move mouse")?;

        thread::sleep(Duration::from_millis(50));

        let enigo_button = match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        };

        self.enigo
            .button(enigo_button, Direction::Click)
            .context("Failed to click mouse")?;

        Ok(())
    }

    fn type_text(&mut self, text: &str) -> Result<()> {
        self.enigo.text(text).context("Failed to type text")?;
        Ok(())
    }

    fn keypress(&mut self, keys: &[String]) -> Result<()> {
        use enigo::Key;

        if keys.is_empty() {
            return Ok(());
        }

        let mut modifier_keys = Vec::new();
        let mut regular_keys = Vec::new();

        for key_str in keys {
            let key = self.parse_key(key_str)?;
            match key {
                Key::Shift | Key::Control | Key::Alt | Key::Meta => {
                    modifier_keys.push(key);
                }
                _ => {
                    regular_keys.push(key);
                }
            }
        }

        for modifier in &modifier_keys {
            self.enigo
                .key(*modifier, Direction::Press)
                .context("Failed to press modifier key")?;
        }

        thread::sleep(Duration::from_millis(50));

        for regular_key in &regular_keys {
            self.enigo
                .key(*regular_key, Direction::Click)
                .context("Failed to press regular key")?;
            thread::sleep(Duration::from_millis(50));
        }

        for modifier in modifier_keys.iter().rev() {
            self.enigo
                .key(*modifier, Direction::Release)
                .context("Failed to release modifier key")?;
        }

        Ok(())
    }

    fn scroll(&mut self, x: i32, y: i32, scroll_x: i32, scroll_y: i32) -> Result<()> {
        self.enigo
            .move_mouse(x, y, Coordinate::Abs)
            .context("Failed to move mouse")?;

        thread::sleep(Duration::from_millis(50));

        if scroll_x != 0 {
            self.enigo
                .scroll(scroll_x, enigo::Axis::Horizontal)
                .context("Failed to scroll horizontally")?;
        }

        if scroll_y != 0 {
            self.enigo
                .scroll(scroll_y, enigo::Axis::Vertical)
                .context("Failed to scroll vertically")?;
        }

        Ok(())
    }

    fn wait(&self, duration_ms: u64) -> Result<()> {
        thread::sleep(Duration::from_millis(duration_ms));
        Ok(())
    }

    fn parse_key(&self, key_str: &str) -> Result<Key> {
        let key = match key_str.to_lowercase().as_str() {
            "return" | "enter" => Key::Return,
            "tab" => Key::Tab,
            "space" => Key::Space,
            "backspace" => Key::Backspace,
            "delete" => Key::Delete,
            "escape" | "esc" => Key::Escape,
            "up" | "uparrow" => Key::UpArrow,
            "down" | "downarrow" => Key::DownArrow,
            "left" | "leftarrow" => Key::LeftArrow,
            "right" | "rightarrow" => Key::RightArrow,
            "home" => Key::Home,
            "end" => Key::End,
            "pageup" => Key::PageUp,
            "pagedown" => Key::PageDown,
            "shift" => Key::Shift,
            "control" | "ctrl" => Key::Control,
            "alt" | "option" => Key::Alt,
            "meta" | "command" | "cmd" => Key::Meta,
            "capslock" => Key::CapsLock,
            "f1" => Key::F1,
            "f2" => Key::F2,
            "f3" => Key::F3,
            "f4" => Key::F4,
            "f5" => Key::F5,
            "f6" => Key::F6,
            "f7" => Key::F7,
            "f8" => Key::F8,
            "f9" => Key::F9,
            "f10" => Key::F10,
            "f11" => Key::F11,
            "f12" => Key::F12,
            s if s.len() == 1 => Key::Unicode(s.chars().next().unwrap()),
            _ => anyhow::bail!("Unknown key: {}", key_str),
        };
        Ok(key)
    }
}

impl Default for MacAutomation {
    fn default() -> Self {
        Self::new().expect("Failed to initialize MacAutomation")
    }
}
