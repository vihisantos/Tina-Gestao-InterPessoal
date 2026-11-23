//! Ponto de entrada definitivo da Tina: aplicação desktop Tauri 2.
//!
//! Todo o backend reside neste binário mais o crate `tina` (`src/`).
//! Não existe servidor HTTP/WebSocket: a comunicação usa comandos e eventos
//! nativos do Tauri.

mod commands;

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
    installation_id: String,
}

/// Retorna o estado real da instalação Tina.
#[tauri::command]
fn bootstrap(app: State<'_, TinaApp>) -> Result<BootstrapInfo, String> {
    let authenticated = app.owner_unlocked().map_err(|e| e.to_string())?;
    Ok(BootstrapInfo {
        version: tina::VERSION.to_string(),
        database_path: if authenticated {
            app.config().database_path.to_string_lossy().into_owned()
        } else {
            "Protegido até o login".into()
        },
        database_ready: true,
        message: "Tina inicializada localmente".to_string(),
        installation_id: if authenticated {
            app.config().installation_id.clone()
        } else {
            String::new()
        },
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
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            ping,
            commands::owner_status,
            commands::create_owner_account,
            commands::sign_in_owner,
            commands::lock_owner,
            commands::list_companies,
            commands::get_company,
            commands::create_company,
            commands::update_company,
            commands::archive_company,
            commands::list_departments,
            commands::get_department,
            commands::create_department,
            commands::update_department,
            commands::archive_department,
            commands::list_positions,
            commands::get_position,
            commands::create_position,
            commands::update_position,
            commands::archive_position,
            commands::create_employee,
            commands::get_employee,
            commands::list_employees_by_company,
            commands::list_employees_by_department,
            commands::update_employee,
            commands::update_employee_responsibilities,
            commands::change_employee_position,
            commands::assign_employee_supervisor,
            commands::deactivate_employee,
            commands::reactivate_employee
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar a aplicação desktop Tina");
}
