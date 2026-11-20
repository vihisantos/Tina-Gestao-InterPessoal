//! Modelo do histórico de movimentações de cargo e departamento.
//!
//! A posição **atual** continua em `employees.position_id`; o departamento é
//! derivado da cadeia `position → department`. O histórico é um log
//! append-only em `employee_assignment_history` e registra a posição e o
//! departamento anteriores e novos de cada mudança.

/// Evento imutável de mudança de cargo/departamento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentHistoryEntry {
    /// ID do evento.
    pub id: String,
    /// Funcionário movimentado.
    pub employee_id: String,
    /// Cargo anterior à mudança.
    pub previous_position_id: String,
    /// Cargo resultante da mudança.
    pub new_position_id: String,
    /// Departamento derivado do cargo anterior.
    pub previous_department_id: String,
    /// Departamento derivado do novo cargo.
    pub new_department_id: String,
    /// Motivo informado pelo chamador, quando houver.
    pub reason: Option<String>,
    /// Autor da operação, quando houver identidade confiável.
    ///
    /// Hoje é sempre `None`: a instalação é local e não há autenticação por
    /// operação. O campo não é preenchido com o proprietário da instalação.
    pub author_id: Option<String>,
    /// Instante da operação (UTC, formato SQLite).
    pub created_at: String,
}

/// Resultado de uma tentativa de mudança de cargo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionChange {
    /// Funcionário movimentado.
    pub employee_id: String,
    /// Cargo anterior.
    pub previous_position_id: String,
    /// Cargo resultante (novo ou mantido).
    pub new_position_id: String,
    /// Departamento anterior.
    pub previous_department_id: String,
    /// Departamento resultante.
    pub new_department_id: String,
    /// `true` quando houve mudança e o evento foi gravado; `false` quando o
    /// funcionário já ocupava o cargo solicitado.
    pub recorded: bool,
}
