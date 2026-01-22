use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::automation::{Action, MacAutomation, MouseButton};
use crate::screenshot::ScreenCapture;

pub fn calculate_scale_factor(width: u32, height: u32) -> f64 {
    let long_edge = width.max(height) as f64;
    let total_pixels = (width * height) as f64;

    let long_edge_scale = 1568.0 / long_edge;
    let total_pixels_scale = (1_150_000.0 / total_pixels).sqrt();

    long_edge_scale.min(total_pixels_scale).min(1.0)
}

const MODEL: &str = "claude-sonnet-4-5";
const MAX_ITERATIONS: usize = 50;
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const TOOL_VERSION: &str = "computer_20250124";
const BETA_FLAG: &str = "computer-use-2025-01-24";

pub struct ComputerUseAgent {
    api_key: String,
    automation: MacAutomation,
    screenshot: ScreenCapture,
    stop_flag: Arc<AtomicBool>,
    full_trust_mode: bool,
    client: reqwest::Client,
    actual_screen_width: u32,
    actual_screen_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: Value,
}

#[derive(Debug, Clone, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    tools: Vec<Value>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    stop_reason: String,
    #[serde(default)]
    stop_sequence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolResult {
    #[serde(rename = "type")]
    result_type: String,
    tool_use_id: String,
    content: Value,
}

impl ComputerUseAgent {
    pub fn new(api_key: String, stop_flag: Arc<AtomicBool>) -> Result<Self> {
        let automation = MacAutomation::new()?;
        let client = reqwest::Client::new();

        let (actual_width, actual_height) = Self::get_actual_screen_size()?;
        let screenshot = ScreenCapture::new(actual_width, actual_height);

        Ok(Self {
            api_key,
            automation,
            screenshot,
            stop_flag,
            full_trust_mode: true,
            client,
            actual_screen_width: actual_width,
            actual_screen_height: actual_height,
        })
    }

    fn get_actual_screen_size() -> Result<(u32, u32)> {
        use xcap::Monitor;
        let monitors = Monitor::all().context("Failed to get monitors")?;
        let primary = monitors
            .into_iter()
            .find(|m| m.is_primary())
            .context("No primary monitor found")?;
        Ok((primary.width(), primary.height()))
    }

    pub fn with_display_size(mut self, width: u32, height: u32) -> Self {
        self.screenshot = ScreenCapture::new(width, height);
        self
    }

    pub fn with_full_trust_mode(mut self, enabled: bool) -> Self {
        self.full_trust_mode = enabled;
        self
    }

    pub async fn execute_command(&mut self, command: &str) -> Result<String> {
        let (display_width, display_height) = self.screenshot.get_display_size();

        let system_prompt = format!(
            "You are an automation assistant for macOS with screen resolution {}x{}. \
             You have been granted access to the computer use tool for legitimate desktop automation.\n\n\
             System context:\n\
             - macOS desktop environment\n\
             - Uses Raycast (not Spotlight) for app launching via Cmd+Space\n\
             - Applications open in windows that appear on screen\n\
             - After launching an app, it will appear as a window - take a screenshot to verify\n\n\
             Your role: Translate user requests into specific computer actions using the tool.\n\n\
             Available actions:\n\
             - screenshot: Capture the current display (use frequently to see current state)\n\
             - left_click: Click at coordinates [x, y] (use ONLY when keyboard shortcuts won't work)\n\
             - type: Type text string (use this to enter text into input fields)\n\
             - key: Press key or key combination (e.g., \"cmd+space\" for Spotlight/Raycast, \"return\" for Enter)\n\
             - mouse_move: Move cursor to coordinates\n\
             - scroll: Scroll in any direction with amount control\n\
             - left_click_drag: Click and drag between coordinates\n\
             - right_click, middle_click: Additional mouse buttons\n\
             - double_click, triple_click: Multiple clicks\n\
             - wait: DO NOT USE - actions have built-in delays, wait is unnecessary\n\n\
             CRITICAL macOS patterns:\n\
             - To open applications: Press Cmd+Space (opens Raycast), type app name with 'type' action, then press Return/Enter key - DO NOT CLICK\n\
             - ALWAYS use keyboard shortcuts when possible - prefer Return/Enter over mouse clicks\n\
             - After typing text, press Return/Enter to submit - don't click buttons\n\
             - Use mouse clicks ONLY when keyboard shortcuts are impossible\n\
             - Navigate with keyboard: arrows, tab, return - avoid mouse when possible\n\n\
             Speed and efficiency:\n\
             - DO NOT use wait actions - the system has built-in delays after each action\n\
             - Work quickly - actions execute fast on macOS\n\
             - Take screenshots after major actions to verify state\n\
             - Prefer keyboard over mouse for speed\n\
             - After typing, immediately press Return/Enter - don't wait or click\n\n\
             Process:\n\
             1. Take a screenshot to see current state\n\
             2. Execute actions rapidly using keyboard shortcuts\n\
             3. After typing, press Return/Enter immediately\n\
             4. CRITICAL: After pressing Return/Enter to launch an app, ALWAYS take a screenshot to verify it opened\n\
             5. Use screenshots to confirm actions succeeded before continuing\n\
             6. Avoid wait actions - they're unnecessary\n\n\
             Verification:\n\
             - After launching an app (Cmd+Space → type → Return), take a screenshot\n\
             - Look for the app window in the screenshot to confirm it opened\n\
             - Only proceed with next actions after verifying success in screenshot",
            display_width, display_height
        );

        let computer_tool = json!({
            "type": TOOL_VERSION,
            "name": "computer",
            "display_width_px": display_width,
            "display_height_px": display_height,
            "display_number": 1
        });

        let mut messages: Vec<AnthropicMessage> = vec![AnthropicMessage {
            role: "user".to_string(),
            content: json!([{
                "type": "text",
                "text": command
            }]),
        }];

        let mut iteration = 0;
        let mut final_response = String::new();

        while iteration < MAX_ITERATIONS {
            if self.stop_flag.load(Ordering::Relaxed) {
                anyhow::bail!("Execution stopped by user");
            }

            iteration += 1;

            let request = AnthropicRequest {
                model: MODEL.to_string(),
                max_tokens: 4096,
                tools: vec![computer_tool.clone()],
                messages: messages.clone(),
                system: Some(system_prompt.clone()),
            };

            let response = self
                .client
                .post(ANTHROPIC_API_URL)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("anthropic-beta", BETA_FLAG)
                .header("content-type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!("Anthropic API error: {:?}", e);
                    anyhow::anyhow!("Failed to call Anthropic API: {}", e)
                })?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                tracing::error!("Anthropic API error: {} - {}", status, error_text);
                anyhow::bail!("Anthropic API returned error: {} - {}", status, error_text);
            }

            let api_response: AnthropicResponse = response
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to parse Anthropic response: {}", e))?;

            let mut tool_results = Vec::new();
            let mut assistant_content = Vec::new();

            for block in api_response.content {
                match block {
                    ContentBlock::Text { text } => {
                        final_response = text.clone();
                        assistant_content.push(json!({
                            "type": "text",
                            "text": text
                        }));
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        if name == "computer" {
                            let id_clone = id.clone();
                            let result = match self.execute_computer_action(&input).await {
                                Ok(r) => r,
                                Err(e) => {
                                    tracing::error!("Failed to execute computer action: {}", e);
                                    json!([{
                                        "type": "text",
                                        "text": format!("Error executing action: {}", e)
                                    }])
                                }
                            };
                            tool_results.push(ToolResult {
                                result_type: "tool_result".to_string(),
                                tool_use_id: id_clone.clone(),
                                content: json!(result),
                            });

                            assistant_content.push(json!({
                                "type": "tool_use",
                                "id": id_clone,
                                "name": name,
                                "input": input
                            }));
                        }
                    }
                }
            }

            messages.push(AnthropicMessage {
                role: "assistant".to_string(),
                content: json!(assistant_content),
            });

            if tool_results.is_empty() {
                break;
            }

            let tool_result_content: Vec<Value> = tool_results
                .into_iter()
                .map(|tr| {
                    json!({
                        "type": tr.result_type,
                        "tool_use_id": tr.tool_use_id,
                        "content": tr.content
                    })
                })
                .collect();

            messages.push(AnthropicMessage {
                role: "user".to_string(),
                content: json!(tool_result_content),
            });
        }

        if iteration >= MAX_ITERATIONS {
            anyhow::bail!("Maximum iterations reached");
        }

        Ok(final_response)
    }

    async fn execute_computer_action(&mut self, input: &Value) -> Result<Value> {
        let action = input["action"]
            .as_str()
            .context("Missing action field")?;

        tracing::info!("Executing action: {} with input: {}", action, serde_json::to_string_pretty(input).unwrap_or_default());

        let (display_width, display_height) = self.screenshot.get_display_size();
        let scale = calculate_scale_factor(display_width, display_height);
        let scale_back = 1.0 / scale;

        let result = match action {
            "screenshot" => {
                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "left_click" => {
                let coord = input["coordinate"]
                    .as_array()
                    .context("Missing coordinate array")?;
                let x = (coord[0].as_f64().context("Invalid x coordinate")? * scale_back) as i32;
                let y = (coord[1].as_f64().context("Invalid y coordinate")? * scale_back) as i32;

                tracing::info!("Clicking at ({}, {})", x, y);
                self.automation
                    .execute_action(Action::Click {
                        x,
                        y,
                        button: MouseButton::Left,
                    })?;
                
                std::thread::sleep(std::time::Duration::from_millis(150));

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "right_click" => {
                let coord = input["coordinate"]
                    .as_array()
                    .context("Missing coordinate array")?;
                let x = (coord[0].as_f64().context("Invalid x coordinate")? * scale_back) as i32;
                let y = (coord[1].as_f64().context("Invalid y coordinate")? * scale_back) as i32;

                self.automation
                    .execute_action(Action::Click {
                        x,
                        y,
                        button: MouseButton::Right,
                    })?;

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "middle_click" => {
                let coord = input["coordinate"]
                    .as_array()
                    .context("Missing coordinate array")?;
                let x = (coord[0].as_f64().context("Invalid x coordinate")? * scale_back) as i32;
                let y = (coord[1].as_f64().context("Invalid y coordinate")? * scale_back) as i32;

                self.automation
                    .execute_action(Action::Click {
                        x,
                        y,
                        button: MouseButton::Middle,
                    })?;

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "double_click" => {
                let coord = input["coordinate"]
                    .as_array()
                    .context("Missing coordinate array")?;
                let x = (coord[0].as_f64().context("Invalid x coordinate")? * scale_back) as i32;
                let y = (coord[1].as_f64().context("Invalid y coordinate")? * scale_back) as i32;

                for _ in 0..2 {
                    self.automation.execute_action(Action::Click {
                        x,
                        y,
                        button: MouseButton::Left,
                    })?;
                }

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "triple_click" => {
                let coord = input["coordinate"]
                    .as_array()
                    .context("Missing coordinate array")?;
                let x = (coord[0].as_f64().context("Invalid x coordinate")? * scale_back) as i32;
                let y = (coord[1].as_f64().context("Invalid y coordinate")? * scale_back) as i32;

                for _ in 0..3 {
                    self.automation.execute_action(Action::Click {
                        x,
                        y,
                        button: MouseButton::Left,
                    })?;
                }

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!({
                    "success": true,
                    "screenshot": screenshot_base64
                })
            }
            "type" => {
                let text = input["text"]
                    .as_str()
                    .context("Missing text field")?
                    .to_string();

                tracing::info!("Typing: {}", text);
                self.automation.execute_action(Action::Type { text })?;
                
                std::thread::sleep(std::time::Duration::from_millis(100));

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "key" => {
                let key_str = if let Some(key) = input["key"].as_str() {
                    key.to_string()
                } else if let Some(text) = input["text"].as_str() {
                    text.to_string()
                } else if let Some(keys_array) = input["keys"].as_array() {
                    keys_array
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join("+")
                } else {
                    tracing::error!("Key action input: {}", serde_json::to_string_pretty(input).unwrap_or_default());
                    anyhow::bail!("Missing 'key', 'text', or 'keys' field in key action input");
                };

                let keys = self.parse_key_combination(&key_str)?;
                let is_return_or_enter = keys.iter().any(|k| k.to_lowercase() == "return" || k.to_lowercase() == "enter");
                tracing::info!("Pressing keys: {:?}", keys);
                self.automation.execute_action(Action::Keypress { keys })?;
                
                let delay_ms = if is_return_or_enter { 500 } else { 100 };
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "mouse_move" => {
                let coord = input["coordinate"]
                    .as_array()
                    .context("Missing coordinate array")?;
                let x = (coord[0].as_f64().context("Invalid x coordinate")? * scale_back) as i32;
                let y = (coord[1].as_f64().context("Invalid y coordinate")? * scale_back) as i32;

                self.automation.execute_action(Action::Click {
                    x,
                    y,
                    button: MouseButton::Left,
                })?;

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "scroll" => {
                let coord = input["coordinate"]
                    .as_array()
                    .context("Missing coordinate array")?;
                let x = (coord[0].as_f64().context("Invalid x coordinate")? * scale_back) as i32;
                let y = (coord[1].as_f64().context("Invalid y coordinate")? * scale_back) as i32;

                let scroll_x = input["scroll_x"].as_i64().unwrap_or(0) as i32;
                let scroll_y = input["scroll_y"].as_i64().unwrap_or(0) as i32;

                self.automation.execute_action(Action::Scroll {
                    x,
                    y,
                    scroll_x,
                    scroll_y,
                })?;

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "left_click_drag" => {
                let start_coord = input["start_coordinate"]
                    .as_array()
                    .context("Missing start_coordinate array")?;
                let end_coord = input["end_coordinate"]
                    .as_array()
                    .context("Missing end_coordinate array")?;
                let start_x = (start_coord[0].as_f64().context("Invalid start x")? * scale_back) as i32;
                let start_y = (start_coord[1].as_f64().context("Invalid start y")? * scale_back) as i32;
                let end_x = (end_coord[0].as_f64().context("Invalid end x")? * scale_back) as i32;
                let end_y = (end_coord[1].as_f64().context("Invalid end y")? * scale_back) as i32;

                self.automation.execute_action(Action::Click {
                    x: start_x,
                    y: start_y,
                    button: MouseButton::Left,
                })?;

                std::thread::sleep(std::time::Duration::from_millis(100));

                self.automation.execute_action(Action::Click {
                    x: end_x,
                    y: end_y,
                    button: MouseButton::Left,
                })?;

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            "wait" => {
                let duration_secs = input["duration_seconds"]
                    .as_f64()
                    .or_else(|| input["duration"].as_f64())
                    .unwrap_or(0.2);
                let duration_ms = (duration_secs * 1000.0) as u64;

                tracing::warn!("Wait action used ({}ms) - this is usually unnecessary", duration_ms);
                self.automation.execute_action(Action::Wait { duration_ms })?;

                let screenshot_base64 = self.screenshot.capture_screenshot()?;
                json!([{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": screenshot_base64
                    }
                }])
            }
            _ => {
                anyhow::bail!("Unknown action: {}", action);
            }
        };

        Ok(result)
    }

    fn parse_key_combination(&self, key_str: &str) -> Result<Vec<String>> {
        let parts: Vec<&str> = key_str.split('+').map(|s| s.trim()).collect();
        let mut keys = Vec::new();

        for part in parts {
            let normalized = match part.to_lowercase().as_str() {
                "ctrl" | "control" => "control",
                "cmd" | "command" | "meta" => "meta",
                "alt" | "option" => "alt",
                "shift" => "shift",
                _ => part,
            };
            keys.push(normalized.to_string());
        }

        Ok(keys)
    }
}
