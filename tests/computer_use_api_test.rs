use anyhow::Result;
use std::sync::{
    atomic::AtomicBool,
    Arc,
};
use superctrl::computer_use::ComputerUseAgent;

#[test]
fn test_computer_use_agent_creation() -> Result<()> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    
    let api_key = "test-key-12345".to_string();
    let _agent = ComputerUseAgent::new(api_key, stop_flag)?;
    
    Ok(())
}

#[test]
fn test_coordinate_scaling() {
    use superctrl::computer_use::calculate_scale_factor;
    
    let scale_1920x1080 = calculate_scale_factor(1920, 1080);
    assert!(scale_1920x1080 <= 1.0, "Scale should be <= 1.0");
    assert!(scale_1920x1080 > 0.0, "Scale should be > 0.0");
    
    let scale_800x600 = calculate_scale_factor(800, 600);
    assert!(scale_800x600 <= 1.0, "Scale should be <= 1.0");
    assert!(scale_800x600 > 0.0, "Scale should be > 0.0");
    
    let scale_small = calculate_scale_factor(640, 480);
    assert_eq!(scale_small, 1.0, "Small images should not be scaled");
}

#[test]
fn test_api_request_structure() {
    use serde_json::json;
    
    let tool = json!({
        "type": "computer_20250124",
        "name": "computer",
        "display_width_px": 1920,
        "display_height_px": 1080,
        "display_number": 1
    });
    
    assert_eq!(tool["type"], "computer_20250124");
    assert_eq!(tool["name"], "computer");
    assert_eq!(tool["display_width_px"], 1920);
    assert_eq!(tool["display_height_px"], 1080);
}

#[test]
fn test_tool_result_format() {
    use serde_json::json;
    
    let screenshot_data = "base64encodedimagedata";
    let tool_result = json!([{
        "type": "image",
        "source": {
            "type": "base64",
            "media_type": "image/jpeg",
            "data": screenshot_data
        }
    }]);
    
    assert_eq!(tool_result[0]["type"], "image");
    assert_eq!(tool_result[0]["source"]["type"], "base64");
    assert_eq!(tool_result[0]["source"]["media_type"], "image/jpeg");
    assert_eq!(tool_result[0]["source"]["data"], screenshot_data);
}
