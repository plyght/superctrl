use anyhow::Result;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use superctrl::{Action, MacAutomation, MouseButton, ScreenCapture};

#[test]
fn test_screenshot_capture() -> Result<()> {
    let capture = ScreenCapture::default();
    let screenshot = capture.capture_screenshot()?;

    assert!(!screenshot.is_empty(), "Screenshot should not be empty");
    assert!(
        screenshot.len() > 100,
        "Screenshot should be base64 encoded"
    );

    Ok(())
}

#[test]
fn test_screenshot_custom_size() -> Result<()> {
    let capture = ScreenCapture::new(800, 600);
    let (width, height) = capture.get_display_size();

    assert_eq!(width, 800);
    assert_eq!(height, 600);

    Ok(())
}

#[test]
fn test_automation_wait() -> Result<()> {
    let mut automation = MacAutomation::new()?;

    let start = std::time::Instant::now();
    automation.execute_action(Action::Wait { duration_ms: 100 })?;
    let elapsed = start.elapsed();

    assert!(elapsed.as_millis() >= 100, "Wait should be at least 100ms");

    Ok(())
}

#[test]
fn test_automation_actions() -> Result<()> {
    let mut automation = MacAutomation::new()?;

    let actions = vec![
        Action::Wait { duration_ms: 50 },
        Action::Scroll {
            x: 500,
            y: 500,
            scroll_x: 0,
            scroll_y: 1,
        },
        Action::Wait { duration_ms: 50 },
    ];

    for action in actions {
        automation.execute_action(action)?;
    }

    Ok(())
}

#[test]
fn test_stop_flag() {
    let stop_flag = Arc::new(AtomicBool::new(false));

    assert!(!stop_flag.load(Ordering::Relaxed));

    stop_flag.store(true, Ordering::Relaxed);
    assert!(stop_flag.load(Ordering::Relaxed));
}

#[test]
fn test_mouse_button_types() {
    let buttons = vec![MouseButton::Left, MouseButton::Right, MouseButton::Middle];

    for button in buttons {
        let _action = Action::Click {
            x: 100,
            y: 100,
            button,
        };
    }
}
