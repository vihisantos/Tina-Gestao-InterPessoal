//! IPC para a conta local do proprietário, sem expor o segredo do cofre.

use serde::Serialize;
use tauri::State;
use tina::TinaApp;

/// Estado público da conta local e da sessão.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerStatusDto {
    configured: bool,
    unlocked: bool,
    display_name: Option<String>,
}

/// Consulta estado inicial da conta e validade da sessão.
#[tauri::command]
pub async fn owner_status(state: State<'_, TinaApp>) -> Result<OwnerStatusDto, String> {
    let status = state.owner_status().await.map_err(|e| e.to_string())?;
    Ok(OwnerStatusDto {
        configured: status.configured,
        unlocked: status.unlocked,
        display_name: status.display_name,
    })
}

/// Primeiro acesso: cadastra nome, senha e gera chave segura no vault.
#[tauri::command]
pub async fn create_owner_account(
    state: State<'_, TinaApp>,
    display_name: String,
    password: String,
) -> Result<(), String> {
    state
        .create_owner_account(&display_name, &password)
        .await
        .map_err(|e| e.to_string())
}

/// Autentica senha sem confiar em UUIDs fornecidos pela interface.
#[tauri::command]
pub async fn sign_in_owner(state: State<'_, TinaApp>, password: String) -> Result<(), String> {
    state
        .sign_in_owner(&password)
        .await
        .map_err(|e| e.to_string())
}

/// Bloqueia a sessão sem eliminar a conta ou a chave.
#[tauri::command]
pub fn lock_owner(state: State<'_, TinaApp>, run: State<'_, crate::ChatRun>) -> Result<(), String> {
    state.lock_owner().map_err(|e| e.to_string())?;
    run.cancel(None)
}
