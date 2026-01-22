use anyhow::Result;
use notify_rust::{Notification, Timeout};

pub fn notify_command_received(command: &str) -> Result<()> {
    Notification::new()
        .summary("superctrl")
        .body(&format!("Command received: {}", truncate(command, 60)))
        .icon("computer")
        .timeout(Timeout::Milliseconds(3000))
        .show()?;
    Ok(())
}

pub fn notify_command_completed(command: &str) -> Result<()> {
    Notification::new()
        .summary("superctrl")
        .body(&format!("✓ Completed: {}", truncate(command, 60)))
        .icon("checkbox")
        .timeout(Timeout::Milliseconds(3000))
        .show()?;
    Ok(())
}

pub fn notify_command_failed(command: &str, error: &str) -> Result<()> {
    Notification::new()
        .summary("superctrl")
        .body(&format!(
            "✗ Failed: {}\n{}",
            truncate(command, 40),
            truncate(error, 60)
        ))
        .icon("error")
        .timeout(Timeout::Milliseconds(5000))
        .show()?;
    Ok(())
}

pub fn notify_emergency_stop() -> Result<()> {
    Notification::new()
        .summary("superctrl")
        .body("🛑 Emergency stop triggered")
        .icon("stop")
        .timeout(Timeout::Milliseconds(2000))
        .show()?;
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
