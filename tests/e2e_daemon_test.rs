use anyhow::Result;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use superctrl::computer_use::ComputerUseAgent;
use tokio::time::timeout;

#[tokio::test]
async fn test_ipc_server_lifecycle() -> Result<()> {
    use superctrl::ipc::{IpcCommand, IpcResponse};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let socket_path = "/tmp/superctrl_test.sock";
    
    if std::path::Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }

    let listener = tokio::net::UnixListener::bind(socket_path)?;

    let server_task = tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buffer = vec![0u8; 4096];
            if let Ok(n) = stream.read(&mut buffer).await {
                let request = String::from_utf8_lossy(&buffer[..n]);
                if let Ok(cmd) = serde_json::from_str::<IpcCommand>(&request) {
                    let response = match cmd {
                        IpcCommand::Status => IpcResponse {
                            success: true,
                            message: "Daemon is running".to_string(),
                        },
                        _ => IpcResponse {
                            success: false,
                            message: "Unsupported".to_string(),
                        },
                    };
                    let resp_json = serde_json::to_string(&response).unwrap();
                    let _ = stream.write_all(resp_json.as_bytes()).await;
                }
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut stream = UnixStream::connect(socket_path).await?;
    let cmd = IpcCommand::Status;
    let cmd_json = serde_json::to_string(&cmd)?;
    stream.write_all(cmd_json.as_bytes()).await?;
    stream.flush().await?;

    let mut buffer = vec![0u8; 4096];
    let n = stream.read(&mut buffer).await?;
    let response: IpcResponse = serde_json::from_slice(&buffer[..n])?;

    assert!(response.success);
    assert_eq!(response.message, "Daemon is running");

    let _ = server_task.await;
    std::fs::remove_file(socket_path)?;

    Ok(())
}

#[tokio::test]
async fn test_stop_flag_interrupt() -> Result<()> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    
    let stop_flag_clone = stop_flag.clone();
    let task = tokio::spawn(async move {
        let mut iterations = 0;
        while iterations < 100 {
            if stop_flag_clone.load(Ordering::Relaxed) {
                return iterations;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            iterations += 1;
        }
        iterations
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    stop_flag.store(true, Ordering::Relaxed);

    let result = timeout(Duration::from_secs(2), task).await??;
    
    assert!(result < 100, "Task should have stopped early");
    assert!(result > 0, "Task should have run some iterations");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_real_api_call() -> Result<()> {
    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            println!("Skipping real API test: ANTHROPIC_API_KEY not set");
            return Ok(());
        }
    };

    let stop_flag = Arc::new(AtomicBool::new(false));
    let mut agent = ComputerUseAgent::new(api_key, stop_flag)?;

    let result = timeout(
        Duration::from_secs(30),
        agent.execute_command("What can you see on screen? Just describe briefly.")
    )
    .await??;

    assert!(!result.is_empty(), "Should receive a response from API");
    println!("API Response: {}", result);

    Ok(())
}

#[tokio::test]
async fn test_automation_sequence() -> Result<()> {
    use superctrl::{Action, MacAutomation};

    let mut automation = MacAutomation::new()?;

    let actions = vec![
        Action::Wait { duration_ms: 100 },
        Action::MouseMove { x: 500, y: 500 },
        Action::Wait { duration_ms: 50 },
        Action::Scroll {
            x: 500,
            y: 500,
            scroll_x: 0,
            scroll_y: 1,
        },
    ];

    for action in actions {
        automation.execute_action(action)?;
    }

    Ok(())
}

#[tokio::test]
async fn test_screenshot_and_scale() -> Result<()> {
    use superctrl::{computer_use::calculate_scale_factor, ScreenCapture};

    let capture = ScreenCapture::default();
    let screenshot = capture.capture_screenshot()?;

    assert!(!screenshot.is_empty());
    assert!(screenshot.starts_with("iVBOR") || screenshot.starts_with("/9j/"));

    let scale = calculate_scale_factor(1920, 1080);
    assert!(scale > 0.0 && scale <= 1.0);

    Ok(())
}
