use anyhow::{Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestMessageContentPartImageArgs,
        ChatCompletionRequestMessageContentPartTextArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
        ChatCompletionTool, ChatCompletionToolArgs, ChatCompletionToolType,
        CreateChatCompletionRequestArgs, FunctionObjectArgs, ImageDetail, ImageUrlArgs,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::automation::{Action, MacAutomation, MouseButton};
use crate::screenshot::ScreenCapture;

const MODEL: &str = "gpt-4o";
const MAX_ITERATIONS: usize = 50;

pub struct ComputerUseAgent {
    client: Client<OpenAIConfig>,
    automation: MacAutomation,
    screenshot: ScreenCapture,
    stop_flag: Arc<AtomicBool>,
    full_trust_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ComputerCall {
    action: String,
    #[serde(flatten)]
    params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ComputerCallOutput {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    screenshot: String,
}

impl ComputerUseAgent {
    pub fn new(api_key: String, stop_flag: Arc<AtomicBool>) -> Result<Self> {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        let automation = MacAutomation::new()?;
        let screenshot = ScreenCapture::default();

        Ok(Self {
            client,
            automation,
            screenshot,
            stop_flag,
            full_trust_mode: true,
        })
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
        let initial_screenshot = self
            .screenshot
            .capture_screenshot()
            .context("Failed to capture initial screenshot")?;

        let (display_width, display_height) = self.screenshot.get_display_size();

        let system_message = ChatCompletionRequestSystemMessageArgs::default()
            .content(format!(
                "You are a computer use agent running on macOS. The display resolution is {}x{}. \
                 Execute the user's command by using the computer_use_preview tool to perform actions. \
                 Always provide a screenshot with each action. Be precise with coordinates and actions.",
                display_width, display_height
            ))
            .build()?;

        let text_part = ChatCompletionRequestMessageContentPartTextArgs::default()
            .text(command)
            .build()?;

        let image_url = ImageUrlArgs::default()
            .url(format!("data:image/png;base64,{}", initial_screenshot))
            .detail(ImageDetail::High)
            .build()?;

        let image_part = ChatCompletionRequestMessageContentPartImageArgs::default()
            .image_url(image_url)
            .build()?;

        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content(ChatCompletionRequestUserMessageContent::Array(vec![
                text_part.into(),
                image_part.into(),
            ]))
            .build()?;

        let mut messages: Vec<ChatCompletionRequestMessage> =
            vec![system_message.into(), user_message.into()];

        let tool = self.create_computer_use_tool(display_width, display_height)?;

        let mut iteration = 0;
        let mut final_response = String::new();

        while iteration < MAX_ITERATIONS {
            if self.stop_flag.load(Ordering::Relaxed) {
                anyhow::bail!("Execution stopped by user");
            }

            iteration += 1;

            let request = CreateChatCompletionRequestArgs::default()
                .model(MODEL)
                .messages(messages.clone())
                .tools(vec![tool.clone()])
                .build()?;

            let response = self
                .client
                .chat()
                .create(request)
                .await
                .context("Failed to create chat completion")?;

            let choice = response.choices.first().context("No choices in response")?;

            let message = &choice.message;

            if let Some(content) = &message.content {
                final_response = content.clone();
            }

            let assistant_message = ChatCompletionRequestAssistantMessageArgs::default()
                .content(message.content.clone().unwrap_or_default())
                .tool_calls(message.tool_calls.clone().unwrap_or_default())
                .build()?;

            messages.push(ChatCompletionRequestMessage::Assistant(assistant_message));

            if let Some(tool_calls) = &message.tool_calls {
                if tool_calls.is_empty() {
                    break;
                }

                for tool_call in tool_calls {
                    if tool_call.function.name != "computer_use_preview" {
                        continue;
                    }

                    let computer_call: ComputerCall =
                        serde_json::from_str(&tool_call.function.arguments)
                            .context("Failed to parse computer call")?;

                    let output = self.execute_computer_call(&computer_call).await?;

                    let tool_message = json!({
                        "role": "tool",
                        "tool_call_id": tool_call.id,
                        "content": serde_json::to_string(&output)?
                    });

                    messages.push(serde_json::from_value(tool_message)?);
                }
            } else {
                break;
            }
        }

        if iteration >= MAX_ITERATIONS {
            anyhow::bail!("Maximum iterations reached");
        }

        Ok(final_response)
    }

    async fn execute_computer_call(&mut self, call: &ComputerCall) -> Result<ComputerCallOutput> {
        let action_result = match call.action.as_str() {
            "click" => {
                let x = call.params["x"].as_i64().context("Missing x coordinate")? as i32;
                let y = call.params["y"].as_i64().context("Missing y coordinate")? as i32;
                let button_str = call.params["button"].as_str().unwrap_or("left");
                let button = match button_str {
                    "left" => MouseButton::Left,
                    "right" => MouseButton::Right,
                    "middle" => MouseButton::Middle,
                    _ => MouseButton::Left,
                };

                self.automation
                    .execute_action(Action::Click { x, y, button })
            }
            "type" => {
                let text = call.params["text"]
                    .as_str()
                    .context("Missing text")?
                    .to_string();

                self.automation.execute_action(Action::Type { text })
            }
            "keypress" => {
                let keys_value = &call.params["keys"];
                let keys = if let Some(keys_array) = keys_value.as_array() {
                    keys_array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                } else if let Some(key_str) = keys_value.as_str() {
                    vec![key_str.to_string()]
                } else {
                    anyhow::bail!("Invalid keys format");
                };

                self.automation.execute_action(Action::Keypress { keys })
            }
            "scroll" => {
                let x = call.params["x"].as_i64().context("Missing x coordinate")? as i32;
                let y = call.params["y"].as_i64().context("Missing y coordinate")? as i32;
                let scroll_x = call.params["scroll_x"].as_i64().unwrap_or(0) as i32;
                let scroll_y = call.params["scroll_y"].as_i64().unwrap_or(0) as i32;

                self.automation.execute_action(Action::Scroll {
                    x,
                    y,
                    scroll_x,
                    scroll_y,
                })
            }
            "wait" => {
                let duration_ms = call.params["duration_ms"].as_u64().unwrap_or(1000);

                self.automation.execute_action(Action::Wait { duration_ms })
            }
            _ => {
                anyhow::bail!("Unknown action: {}", call.action);
            }
        };

        let screenshot = self
            .screenshot
            .capture_screenshot()
            .context("Failed to capture screenshot")?;

        match action_result {
            Ok(_) => Ok(ComputerCallOutput {
                success: true,
                error: None,
                screenshot,
            }),
            Err(e) => Ok(ComputerCallOutput {
                success: false,
                error: Some(e.to_string()),
                screenshot,
            }),
        }
    }

    fn create_computer_use_tool(
        &self,
        display_width: u32,
        display_height: u32,
    ) -> Result<ChatCompletionTool> {
        let function = FunctionObjectArgs::default()
            .name("computer_use_preview")
            .description(format!(
                "Execute computer actions on macOS. Display size: {}x{}. \
                 Available actions: click, type, keypress, scroll, wait.",
                display_width, display_height
            ))
            .parameters(json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["click", "type", "keypress", "scroll", "wait"],
                        "description": "The action to perform"
                    },
                    "x": {
                        "type": "integer",
                        "description": "X coordinate for click/scroll actions"
                    },
                    "y": {
                        "type": "integer",
                        "description": "Y coordinate for click/scroll actions"
                    },
                    "button": {
                        "type": "string",
                        "enum": ["left", "right", "middle"],
                        "description": "Mouse button for click action"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type for type action"
                    },
                    "keys": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "Keys to press for keypress action"
                    },
                    "scroll_x": {
                        "type": "integer",
                        "description": "Horizontal scroll amount"
                    },
                    "scroll_y": {
                        "type": "integer",
                        "description": "Vertical scroll amount"
                    },
                    "duration_ms": {
                        "type": "integer",
                        "description": "Wait duration in milliseconds"
                    }
                },
                "required": ["action"]
            }))
            .build()?;

        let tool = ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(function)
            .build()?;

        Ok(tool)
    }
}
