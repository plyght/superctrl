use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{Context, Result};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};

pub struct EmergencyStop {
    stop_flag: Arc<AtomicBool>,
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
}

impl EmergencyStop {
    pub fn new() -> Result<Self> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let manager = GlobalHotKeyManager::new().context(
            "Failed to create GlobalHotKeyManager. \
             On macOS, this requires Accessibility permissions. \
             Go to System Settings > Privacy & Security > Accessibility \
             and add superctrl to the allowed apps.",
        )?;

        let hotkey = HotKey::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Escape);

        Ok(Self {
            stop_flag,
            manager,
            hotkey,
        })
    }

    pub fn register_hotkey(&self) -> Result<()> {
        self.manager
            .register(self.hotkey)
            .context("Failed to register global hotkey (⌘⇧⎋)")?;

        eprintln!("✓ Emergency stop hotkey registered: ⌘⇧⎋ (Command+Shift+Escape)");

        Ok(())
    }

    pub fn unregister_hotkey(&self) -> Result<()> {
        self.manager
            .unregister(self.hotkey)
            .context("Failed to unregister hotkey")?;
        Ok(())
    }

    pub fn get_stop_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop_flag)
    }

    pub fn start_listener(stop_flag: Arc<AtomicBool>) {
        tokio::spawn(async move {
            let receiver = GlobalHotKeyEvent::receiver();

            loop {
                if let Ok(event) = receiver.try_recv() {
                    if event.state == global_hotkey::HotKeyState::Pressed {
                        stop_flag.store(true, Ordering::Release);
                        eprintln!("🛑 EMERGENCY STOP ACTIVATED (⌘⇧⎋)");
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });
    }
}

impl Default for EmergencyStop {
    fn default() -> Self {
        Self::new().expect("Failed to create EmergencyStop")
    }
}

impl Drop for EmergencyStop {
    fn drop(&mut self) {
        let _ = self.unregister_hotkey();
    }
}
