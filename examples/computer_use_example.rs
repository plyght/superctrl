use anyhow::Result;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use superctrl::computer_use::ComputerUseAgent;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    let stop_flag = Arc::new(AtomicBool::new(false));

    let stop_flag_clone = stop_flag.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\nReceived Ctrl+C, stopping...");
        stop_flag_clone.store(true, Ordering::Relaxed);
    });

    let mut agent = ComputerUseAgent::new(api_key, stop_flag)?
        .with_display_size(1920, 1080)
        .with_full_trust_mode(true);

    let command = "Open Safari and navigate to github.com";
    println!("Executing command: {}", command);

    match agent.execute_command(command).await {
        Ok(response) => {
            println!("Success: {}", response);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}
