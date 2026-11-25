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

#[tauri::command]
async fn chat_create(
    app: State<'_, TinaApp>,
    title: String,
    request_id: String,
) -> Result<tina::infra::chat::ChatSession, String> {
    app.owner_create_chat_with_id(&request_id, &title)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn chat_list(app: State<'_, TinaApp>) -> Result<Vec<tina::infra::chat::ChatSession>, String> {
    app.owner_list_chats().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn chat_rename(
    app: State<'_, TinaApp>,
    session_id: String,
    title: String,
) -> Result<tina::infra::chat::ChatSession, String> {
    app.owner_rename_chat(&session_id, &title)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn chat_history(
    app: State<'_, TinaApp>,
    session_id: String,
) -> Result<Vec<tina::infra::chat::ChatMessage>, String> {
    app.owner_chat_messages(&session_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn chat_append(
    app: State<'_, TinaApp>,
    session_id: String,
    content: String,
    request_id: String,
) -> Result<tina::infra::chat::ChatMessage, String> {
    app.owner_append_chat_with_id(&session_id, &request_id, &content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_set_key(app: State<'_, TinaApp>, key: String) -> Result<(), String> {
    app.owner_set_nvidia_key(&key).map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_has_key(app: State<'_, TinaApp>) -> Result<bool, String> {
    app.owner_has_nvidia_key().map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_delete_key(app: State<'_, TinaApp>) -> Result<(), String> {
    app.owner_delete_nvidia_key().map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_limits(app: State<'_, TinaApp>) -> Result<tina::infra::chat::ChatLimits, String> {
    app.require_owner(tina::core::authorization::Operation::Read)
        .map_err(|e| e.to_string())?;
    Ok(app.chat_limits())
}

struct ActiveRun {
    id: String,
    cancel: tokio::sync::watch::Sender<bool>,
    receiver: Option<tokio::sync::watch::Receiver<bool>>,
}

#[derive(Default)]
struct ChatRun(std::sync::Mutex<Option<ActiveRun>>);

impl ChatRun {
    fn prepare(&self, id: String) -> Result<(), String> {
        let mut active = self.0.lock().map_err(|_| "estado de chat indisponível")?;
        if active.is_some() {
            return Err("já existe uma geração ativa".into());
        }
        let (cancel, receiver) = tokio::sync::watch::channel(false);
        *active = Some(ActiveRun {
            id,
            cancel,
            receiver: Some(receiver),
        });
        Ok(())
    }

    fn claim(&self, id: &str) -> Result<tokio::sync::watch::Receiver<bool>, String> {
        let mut active = self.0.lock().map_err(|_| "estado de chat indisponível")?;
        active
            .as_mut()
            .filter(|entry| entry.id == id)
            .and_then(|entry| entry.receiver.take())
            .ok_or_else(|| "geração inexistente ou já iniciada".into())
    }

    fn cancel(&self, id: Option<&str>) -> Result<(), String> {
        let mut active = self.0.lock().map_err(|_| "estado de chat indisponível")?;
        if let Some(entry) = active.as_ref() {
            if id.is_none_or(|id| entry.id == id) {
                let _ = entry.cancel.send(true);
                if entry.receiver.is_some() {
                    *active = None;
                }
            }
        }
        Ok(())
    }
}

struct ChatRunGuard<'a>(&'a ChatRun, &'a str);
impl Drop for ChatRunGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0 .0.lock() {
            if active.as_ref().is_some_and(|entry| entry.id == self.1) {
                *active = None;
            }
        }
    }
}

#[tauri::command]
fn chat_prepare(
    app: State<'_, TinaApp>,
    run: State<'_, ChatRun>,
    run_id: String,
) -> Result<(), String> {
    app.require_owner(tina::core::authorization::Operation::Write)
        .map_err(|e| e.to_string())?;
    run.prepare(run_id)
}

#[tauri::command]
async fn chat_generate(
    app: State<'_, TinaApp>,
    run: State<'_, ChatRun>,
    session_id: String,
    run_id: String,
    on_delta: tauri::ipc::Channel<String>,
) -> Result<tina::infra::chat::ChatMessage, String> {
    let rx = run.claim(&run_id)?;
    let _guard = ChatRunGuard(run.inner(), &run_id);
    app.require_owner(tina::core::authorization::Operation::Write)
        .map_err(|e| e.to_string())?;
    app.owner_generate_chat(&session_id, rx, |delta| {
        on_delta
            .send(delta.to_owned())
            .map_err(|_| tina::TinaError::Other("canal de chat encerrado".into()))
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_cancel(
    app: State<'_, TinaApp>,
    run: State<'_, ChatRun>,
    run_id: String,
) -> Result<(), String> {
    app.require_owner(tina::core::authorization::Operation::Write)
        .map_err(|e| e.to_string())?;
    run.cancel(Some(&run_id))
}

#[cfg(test)]
mod chat_run_tests {
    use super::*;

    #[test]
    fn cancellation_before_claim_and_stale_cancellation_are_safe() {
        let run = ChatRun::default();
        run.prepare("first".into()).unwrap();
        run.cancel(Some("first")).unwrap();
        assert!(run.claim("first").is_err());
        run.prepare("second".into()).unwrap();
        run.cancel(Some("first")).unwrap();
        let receiver = run.claim("second").unwrap();
        assert!(!*receiver.borrow());
        assert!(run.claim("second").is_err());
        run.cancel(None).unwrap();
        assert!(*receiver.borrow());
        drop(ChatRunGuard(&run, "second"));
        assert!(run.prepare("third".into()).is_ok());
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let tina_app = tauri::async_runtime::block_on(TinaApp::initialize())
                .map_err(|error| error.to_string())?;

            app.manage(tina_app);
            app.manage(ChatRun::default());

            if let Some(window) = app.get_webview_window("main") {
                window.emit("tina://ready", ())?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            ping,
            chat_create,
            chat_list,
            chat_history,
            chat_rename,
            chat_append,
            chat_prepare,
            chat_generate,
            chat_cancel,
            chat_limits,
            chat_set_key,
            chat_has_key,
            chat_delete_key,
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
