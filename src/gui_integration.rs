use crate::gui::{ActionRecord, AppState, SharedGuiState};

pub fn update_status(state: &SharedGuiState, new_state: AppState) {
    let mut gui_state = state.lock().unwrap();
    gui_state.update_status(new_state);
}

pub fn add_action(state: &SharedGuiState, command: String, description: String) {
    let mut gui_state = state.lock().unwrap();
    let action = ActionRecord::new(command, description);
    gui_state.add_action(action);
}

pub fn trigger_stop(state: &SharedGuiState) {
    let gui_state = state.lock().unwrap();
    gui_state.trigger_stop();
}

pub fn reset_stop(state: &SharedGuiState) {
    let gui_state = state.lock().unwrap();
    gui_state.reset_stop();
}

pub fn is_stopped(state: &SharedGuiState) -> bool {
    let gui_state = state.lock().unwrap();
    let stop_flag = gui_state.get_stop_flag();
    stop_flag.load(std::sync::atomic::Ordering::Acquire)
}
