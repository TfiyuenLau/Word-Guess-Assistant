pub mod codec;
pub mod discovery;
pub mod game_engine;
pub mod network;

use discovery::discover_rooms_command;
use game_engine::{
    get_game_state_command, next_phase_command, restart_game_command, start_game_command,
    submit_blank_guess_command, submit_night_action_command, submit_vote_command,
    GameEngineState,
};
use network::{
    close_game_room_command, create_game_room_command, get_room_players_command,
    join_game_room_command, leave_game_room_command, send_chat_command, NetworkState,
};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(NetworkState::new())
        .manage(GameEngineState::new())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            create_game_room_command,
            close_game_room_command,
            join_game_room_command,
            leave_game_room_command,
            send_chat_command,
            get_room_players_command,
            start_game_command,
            next_phase_command,
            submit_vote_command,
            submit_night_action_command,
            submit_blank_guess_command,
            restart_game_command,
            get_game_state_command,
            discover_rooms_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
