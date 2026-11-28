//! Motor central de orquestração de ferramentas da Tina.
//!
//! O motor separa quatro responsabilidades que hoje estão misturadas no chat:
//! acumulação indexada de chamadas do modelo, planejamento determinístico com
//! dependências, autorização vinculada a uma versão imutável do plano e
//! execução sequencial com retomada e histórico compatível com NIM.
//!
//! Nenhuma escrita organizacional acontece durante a inferência. Planos são
//! persistidos antes de qualquer execução e só avançam após decisão explícita
//! do proprietário.

pub mod calls;
/// Dependências determinísticas e referências simbólicas.
pub mod dependencies;
/// Execução sequencial com retomada e recuperação.
pub mod executor;
/// Histórico compatível com `assistant.tool_calls` e `role: tool`.
pub mod history;
/// Modelos imutáveis de plano e etapas.
pub mod plan;
/// Persistência transacional de planos, etapas e auditoria.
pub mod store;
