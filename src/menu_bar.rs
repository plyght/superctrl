use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};

use crate::gui::{AppState, SharedGuiState};
use crate::preferences;

pub struct MenuBar {
    _tray_icon: TrayIcon,
    status_item: MenuItem,
    recent_actions_items: Vec<MenuItem>,
    stop_item: MenuItem,
    learning_toggle_item: MenuItem,
    generate_prompt_item: MenuItem,
    preferences_item: MenuItem,
    quit_item: MenuItem,
    state: SharedGuiState,
}

impl MenuBar {
    pub fn new(state: SharedGuiState) -> Result<Self> {
        let menu = Menu::new();

        let status_item = MenuItem::new("Status: Idle", false, None);
        menu.append(&status_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let actions_label = MenuItem::new("Recent Actions:", false, None);
        menu.append(&actions_label)?;

        let mut recent_actions_items = Vec::new();
        for i in 0..5 {
            let item = MenuItem::new(format!("  [{}] No actions yet", i + 1), false, None);
            menu.append(&item)?;
            recent_actions_items.push(item);
        }

        menu.append(&PredefinedMenuItem::separator())?;

        let stop_item = MenuItem::new("Stop Current Task", true, None);
        stop_item.set_enabled(false);
        menu.append(&stop_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let learning_toggle_item = MenuItem::new("Start Learning", true, None);
        menu.append(&learning_toggle_item)?;

        let generate_prompt_item = MenuItem::new("Generate System Prompt", true, None);
        menu.append(&generate_prompt_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let preferences_item = MenuItem::new("Preferences...", true, None);
        menu.append(&preferences_item)?;

        let quit_item = MenuItem::new("Quit", true, None);
        menu.append(&quit_item)?;

        let icon_data = Self::create_icon_data(&AppState::Idle);

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_tooltip("superctrl")
            .with_icon(icon_data)
            .build()?;

        Ok(Self {
            _tray_icon: tray_icon,
            status_item,
            recent_actions_items,
            stop_item,
            learning_toggle_item,
            generate_prompt_item,
            preferences_item,
            quit_item,
            state,
        })
    }

    fn create_icon_data(state: &AppState) -> tray_icon::Icon {
        let (r, g, b) = match state {
            AppState::Idle => (128, 128, 128),
            AppState::Working(_) => (0, 122, 255),
            AppState::Error(_) => (255, 59, 48),
        };

        let size = 32;
        let mut rgba = Vec::with_capacity(size * size * 4);

        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - size as f32 / 2.0;
                let dy = y as f32 - size as f32 / 2.0;
                let distance = (dx * dx + dy * dy).sqrt();
                let radius = size as f32 / 2.0 - 4.0;

                let alpha = if distance <= radius {
                    255
                } else if distance <= radius + 2.0 {
                    ((radius + 2.0 - distance) * 127.5) as u8
                } else {
                    0
                };

                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(alpha);
            }
        }

        tray_icon::Icon::from_rgba(rgba, size as u32, size as u32).unwrap()
    }

    pub fn update(&mut self) -> Result<()> {
        let state = self.state.lock().unwrap();

        let status_text = format!(
            "{} {}",
            state.app_state.icon_symbol(),
            state.app_state.status_text()
        );
        self.status_item.set_text(status_text);

        let recent_actions = state.get_recent_actions();
        for (i, item) in self.recent_actions_items.iter().enumerate() {
            if i < recent_actions.len() {
                item.set_text(format!("  {}", recent_actions[i]));
            } else {
                item.set_text(format!("  [{}] No action", i + 1));
            }
        }

        match &state.app_state {
            AppState::Working(_) => {
                self.stop_item.set_enabled(true);
            }
            _ => {
                self.stop_item.set_enabled(false);
            }
        }

        let learning_enabled = state.is_learning_enabled();
        if learning_enabled {
            self.learning_toggle_item.set_text("Stop Learning");
        } else {
            self.learning_toggle_item.set_text("Start Learning");
        }

        Ok(())
    }

    pub fn handle_events(&self) -> Option<MenuBarEvent> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.stop_item.id() {
                return Some(MenuBarEvent::StopTask);
            } else if event.id == self.learning_toggle_item.id() {
                let state = self.state.lock().unwrap();
                let learning_enabled = state.is_learning_enabled();
                drop(state);
                if learning_enabled {
                    return Some(MenuBarEvent::LearnStop);
                } else {
                    return Some(MenuBarEvent::LearnStart);
                }
            } else if event.id == self.generate_prompt_item.id() {
                return Some(MenuBarEvent::LearnGenerate);
            } else if event.id == self.preferences_item.id() {
                return Some(MenuBarEvent::OpenPreferences);
            } else if event.id == self.quit_item.id() {
                return Some(MenuBarEvent::Quit);
            }
        }
        None
    }

    pub fn update_icon(&mut self, state: &AppState) -> Result<()> {
        let icon_data = Self::create_icon_data(state);
        self._tray_icon.set_icon(Some(icon_data))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum MenuBarEvent {
    StopTask,
    LearnStart,
    LearnStop,
    LearnGenerate,
    OpenPreferences,
    Quit,
}

pub fn run_menu_bar_loop(state: SharedGuiState) -> Result<()> {
    let mut menu_bar = MenuBar::new(state.clone())?;
    let rt_handle = tokio::runtime::Handle::try_current()
        .unwrap_or_else(|_| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.handle().clone()
        });

    loop {
        if let Some(event) = menu_bar.handle_events() {
            match event {
                MenuBarEvent::StopTask => {
                    tracing::info!("Stop task requested from menu bar");
                    let gui_state = state.lock().unwrap();
                    gui_state.trigger_stop();
                    drop(gui_state);

                    let mut gui_state = state.lock().unwrap();
                    gui_state.update_status(AppState::Idle);
                }
                MenuBarEvent::LearnStart => {
                    tracing::info!("Start learning requested from menu bar");
                    if let Err(e) = rt_handle.block_on(crate::ipc::send_learn_start_command()) {
                        tracing::error!("Failed to send learn start command: {}", e);
                    } else {
                        let mut gui_state = state.lock().unwrap();
                        gui_state.set_learning_enabled(true);
                    }
                }
                MenuBarEvent::LearnStop => {
                    tracing::info!("Stop learning requested from menu bar");
                    if let Err(e) = rt_handle.block_on(crate::ipc::send_learn_stop_command()) {
                        tracing::error!("Failed to send learn stop command: {}", e);
                    } else {
                        let mut gui_state = state.lock().unwrap();
                        gui_state.set_learning_enabled(false);
                    }
                }
                MenuBarEvent::LearnGenerate => {
                    tracing::info!("Generate system prompt requested from menu bar");
                    if let Err(e) = rt_handle.block_on(crate::ipc::send_learn_finish_command()) {
                        tracing::error!("Failed to send learn finish command: {}", e);
                    }
                }
                MenuBarEvent::OpenPreferences => {
                    tracing::info!("Open preferences requested from menu bar");
                    preferences::open_preferences_window(state.clone());
                }
                MenuBarEvent::Quit => {
                    tracing::info!("Quit requested from menu bar");
                    std::process::exit(0);
                }
            }
        }

        if let Err(e) = menu_bar.update() {
            tracing::error!("Menu bar update error: {}", e);
        }

        let current_state = state.lock().unwrap().app_state.clone();
        if let Err(e) = menu_bar.update_icon(&current_state) {
            tracing::error!("Icon update error: {}", e);
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
