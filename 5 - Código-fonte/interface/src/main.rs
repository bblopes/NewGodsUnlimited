// Sem janela de console no executável de produção.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;

use engine::{Engine, EngineInfo, Frame, Genetics, MapData, TelemetryBatch, UiCommand};
use tauri::State;

#[tauri::command]
fn info() -> EngineInfo {
    Engine::info()
}

#[tauri::command]
async fn frame(engine: State<'_, Engine>) -> Result<Frame, String> {
    Ok(engine.frame())
}

#[tauri::command]
async fn telemetry(
    engine: State<'_, Engine>,
    from: usize,
    epoch: u64,
) -> Result<TelemetryBatch, String> {
    Ok(engine.telemetry(from, epoch))
}

#[tauri::command]
async fn map_data(engine: State<'_, Engine>) -> Result<MapData, String> {
    Ok(engine.map_data())
}

#[tauri::command]
async fn flow_vectors(
    engine: State<'_, Engine>,
    columns: usize,
    rows: usize,
) -> Result<Vec<f32>, String> {
    Ok(engine.flow_vectors(columns, rows))
}

#[tauri::command]
async fn genetics(engine: State<'_, Engine>) -> Result<Option<Genetics>, String> {
    Ok(engine.genetics())
}

#[tauri::command]
async fn command(engine: State<'_, Engine>, cmd: UiCommand) -> Result<(), String> {
    engine.send(cmd);
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(Engine::start())
        .invoke_handler(tauri::generate_handler![
            info,
            frame,
            telemetry,
            map_data,
            flow_vectors,
            genetics,
            command
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o New Gods Unlimited");
}
