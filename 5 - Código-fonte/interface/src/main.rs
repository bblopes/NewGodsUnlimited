// Sem janela de console no executável de produção.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod arquivos;
mod banco;
mod engine;
mod gravador;

use engine::{
    Engine, EngineInfo, Frame, Genetics, IndividualView, MapData, NeuralCatalog, TelemetryBatch,
    UiCommand,
};
use tauri::{Manager, State};

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
async fn neural_catalog(engine: State<'_, Engine>) -> Result<NeuralCatalog, String> {
    Ok(engine.neural_catalog())
}

#[tauri::command]
async fn individual_view(
    engine: State<'_, Engine>,
    key: String,
) -> Result<Option<IndividualView>, String> {
    Ok(engine.individual_view(&key))
}

#[tauri::command]
async fn storage(engine: State<'_, Engine>) -> Result<arquivos::Armazenamento, String> {
    Ok(engine.storage())
}

#[tauri::command]
async fn command(engine: State<'_, Engine>, cmd: UiCommand) -> Result<(), String> {
    engine.send(cmd);
    Ok(())
}

/// Onde ficam as gravações do gravador de sessão.
///
/// Se o executável estiver dentro da pasta do projeto (ex.:
/// `ECOSSYSTEM/2 - Versões de teste/V2 …/X.exe`), usa `4 - Dados/Gravações`,
/// para que tudo fique no HD junto do projeto. Senão, a pasta de dados do app.
fn pasta_de_gravacoes(dados_do_app: Option<std::path::PathBuf>) -> std::path::PathBuf {
    if let Ok(executavel) = std::env::current_exe() {
        for pasta in executavel.ancestors().skip(1).take(5) {
            let dados = pasta.join("4 - Dados");
            if dados.is_dir() {
                return dados.join("Gravações");
            }
        }
    }
    dados_do_app
        .map(|pasta| pasta.join("gravacoes"))
        .unwrap_or_else(|| std::path::PathBuf::from("gravacoes"))
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // O banco de indivíduos fica na pasta de dados do aplicativo, fora
            // das pastas de versão, para sobreviver a qualquer versão nova.
            let dados_do_app = app.path().app_data_dir().ok();
            let caminho = dados_do_app
                .as_ref()
                .map(|pasta| pasta.join("banco_de_individuos.json"));
            app.manage(Engine::start(caminho, pasta_de_gravacoes(dados_do_app)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            info,
            frame,
            telemetry,
            map_data,
            flow_vectors,
            genetics,
            neural_catalog,
            individual_view,
            storage,
            command
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o New Gods Unlimited");
}
