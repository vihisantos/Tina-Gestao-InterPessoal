//! Ponto de entrada definitivo da Tina: aplicação desktop Tauri 2.
//!
//! Todo o backend reside neste binário mais o crate `tina` (`src/`).
//! Não existe servidor HTTP/WebSocket: a comunicação usa comandos e eventos
//! nativos do Tauri.

use serde::Serialize;
use tauri::{Emitter, Manager, State};
use tina::TinaApp;

/// Informações reais do bootstrap, lidas do estado local da aplicação.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapInfo {
    version: String,
    database_path: String,
    database_ready: bool,
    message: String,
}

/// Retorna o estado real da instalação Tina.
#[tauri::command]
fn bootstrap(app: State<'_, TinaApp>) -> Result<BootstrapInfo, String> {
    Ok(BootstrapInfo {
        version: tina::VERSION.to_string(),
        database_path: app.config().database_path.to_string_lossy().into_owned(),
        database_ready: true,
        message: "Tina inicializada localmente".to_string(),
    })
}

/// Comando mínimo para verificar a ponte frontend ↔ Rust.
#[tauri::command]
fn ping() -> String {
    "pong".to_string()
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let tina_app = tauri::async_runtime::block_on(TinaApp::initialize())
                .map_err(|error| error.to_string())?;

            app.manage(tina_app);

            if let Some(window) = app.get_webview_window("main") {
                window.emit("tina://ready", ())?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![bootstrap, ping])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar a aplicação desktop Tina");
}
