pub mod data;
pub mod engine;
pub mod mcp;
pub mod model;
pub mod tools;

use engine::Engine;
use serde_json::Value;
use std::sync::Mutex;

struct AppState(Mutex<Engine>);

/// The UI goes through the same tool dispatcher as the MCP server.
#[tauri::command]
fn call_tool(state: tauri::State<AppState>, name: String, args: Option<Value>) -> Result<Value, String> {
    let mut engine = state.0.lock().map_err(|e| e.to_string())?;
    tools::dispatch(&mut engine, &name, &args.unwrap_or(Value::Object(Default::default())))
}

#[tauri::command]
fn list_tools() -> Vec<Value> {
    tools::definitions()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState(Mutex::new(Engine::new())))
        .invoke_handler(tauri::generate_handler![call_tool, list_tools])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
