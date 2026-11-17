//! Position management

/// Predefined position templates for common roles.
pub const POSITION_TEMPLATES: &[PositionTemplate] = &[
    PositionTemplate {
        code: "agent_base",
        name: "Assistente Base",
        level: 1,
        authority: 1,
        description: "Agente base com capacidades gerais",
        standard_models: &["nvidia/nemotron-3.5-bolt-e4b"],
    },
    PositionTemplate {
        code: "team_lead",
        name: "Líder de Team",
        level: 5,
        authority: 5,
        description: "Líder de time, pode delegar para assistentes base",
        standard_models: &["nvidia/nemotron-3.5-lightning-30b-a3b"],
    },
    PositionTemplate {
        code: "dept_head",
        name: "Chefe de Departamento",
        level: 10,
        authority: 10,
        description: "Chefe de departamento, supervisiona múltiplos líderes",
        standard_models: &["nvidia/nemotron-3.5-lightning-30b-a3b", "nvidia/nemotron-3.5-super-vl-120b-a12b"],
    },
    PositionTemplate {
        code: "executive",
        name: "Executivo",
        level: 20,
        authority: 20,
        description: "Executivo, coordenador de alta direção",
        standard_models: &["nvidia/nemotron-3-ultra-550b-a55b"],
    },
    PositionTemplate {
        code: "specialist",
        name: "Especialista",
        level: 3,
        authority: 3,
        description: "Especialista em área específica",
        standard_models: &["nvidia/nemotron-3.5-bolt-e4b"],
    },
    PositionTemplate {
        code: "analyst",
        name: "Analista",
        level: 2,
        authority: 2,
        description: "Analista de dados ou processos",
        standard_models: &["nvidia/nemotron-3.5-bolt-e4b"],
    },
    PositionTemplate {
        code: "intern",
        name: "Estagiário",
        level: 1,
        authority: 1,
        description: "Estagiário com supervisão obrigatória",
        standard_models: &["nvidia/nemotron-3-nano-30b-a3b"],
    },
];

/// Position template for creating positions.
pub struct PositionTemplate {
    /// Unique code
    pub code: &'static str,
    /// Display name
    pub name: &'static str,
    /// Hierarchy level
    pub level: u32,
    /// Authority level
    pub authority: u32,
    /// Description
    pub description: &'static str,
    /// Standard models for this position
    pub standard_models: &'static [&'static str],
}