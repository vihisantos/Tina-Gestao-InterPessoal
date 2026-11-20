//! Modelo da hierarquia organizacional entre funcionários.
//!
//! A relação **atual** é representada por `employees.supervisor_id` (uma
//! autorreferência), que é a única fonte de verdade do presente. O histórico
//! de alterações é um log append-only separado, em
//! `employee_supervisor_history`.
//!
//! Cargo (`position_id`) e superioridade (`supervisor_id`) são conceitos
//! distintos: o cargo descreve a posição organizacional; o superior descreve
//! a quem o funcionário responde.

use crate::core::employee::Employee;

/// Tipo de alteração registrada no histórico hierárquico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierarchyOperation {
    /// Primeira atribuição de superior direto.
    Assigned,
    /// Troca de superior direto (o anterior existia).
    Changed,
    /// Remoção da relação (funcionário passa a não ter superior).
    Removed,
}

impl HierarchyOperation {
    /// Representação textual persistida no banco.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assigned => "assigned",
            Self::Changed => "changed",
            Self::Removed => "removed",
        }
    }

    /// Converte o valor persistido para o tipo de domínio.
    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "assigned" => Some(Self::Assigned),
            "changed" => Some(Self::Changed),
            "removed" => Some(Self::Removed),
            _ => None,
        }
    }
}

impl std::fmt::Display for HierarchyOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Evento imutável de alteração da relação hierárquica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorHistoryEntry {
    /// ID do evento.
    pub id: String,
    /// Funcionário cuja relação foi alterada.
    pub employee_id: String,
    /// Superior anterior, quando havia um.
    pub previous_supervisor_id: Option<String>,
    /// Superior novo, quando a relação foi atribuída ou trocada.
    pub new_supervisor_id: Option<String>,
    /// Tipo da operação.
    pub operation: HierarchyOperation,
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

/// Resultado de uma tentativa de alteração de superior direto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorChange {
    /// Funcionário alterado.
    pub employee_id: String,
    /// Superior anterior, se existia.
    pub previous_supervisor_id: Option<String>,
    /// Superior resultante (novo ou mantido).
    pub new_supervisor_id: Option<String>,
    /// Operação registrada; `None` quando a relação já era a solicitada.
    pub operation: Option<HierarchyOperation>,
}

/// Nó da árvore organizacional de uma empresa.
#[derive(Debug, Clone)]
pub struct HierarchyNode {
    /// Funcionário do nó.
    pub employee: Employee,
    /// Profundidade a partir de uma raiz (sem superior).
    ///
    /// `None` indica que o funcionário não é alcançável a partir de uma raiz —
    /// por exemplo, dados legados com ciclo. Nunca deveria ocorrer em dados
    /// criados pelas operações validadas desta missão.
    pub depth: Option<i64>,
}
