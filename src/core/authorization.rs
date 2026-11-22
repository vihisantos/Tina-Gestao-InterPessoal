//! Políticas de autoridade organizacional: negam por padrão.
//!
//! `ActorContext` deve ser construído exclusivamente por um resolvedor
//! autenticado no backend. IDs enviados pelo frontend são seletores, nunca
//! prova de identidade. Este módulo NÃO implementa autenticação.
use crate::error::{Result, TinaError};

/// Identidade verificada pelo backend, nunca desserializada do IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    /// Proprietário autenticado da instalação.
    Owner,
    /// Funcionário autenticado, vinculado a uma empresa.
    Employee {
        employee_id: String,
        company_id: String,
    },
}

/// Contexto de autorização obtido de uma fonte confiável no backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorContext {
    installation_id: String,
    actor: Actor,
}

impl ActorContext {
    /// Cria contexto apenas após validação externa de identidade.
    ///
    /// Esta API não autentica o chamador. Nunca a invoque com valores do IPC.
    pub(crate) fn verified(installation_id: String, actor: Actor) -> Self {
        Self {
            installation_id,
            actor,
        }
    }

    /// Identificador da instalação de origem.
    pub fn installation_id(&self) -> &str {
        &self.installation_id
    }

    /// Ator já resolvido pelo backend.
    pub fn actor(&self) -> &Actor {
        &self.actor
    }
}

/// Recurso protegido por escopo de instalação e, quando aplicável, empresa.
#[derive(Debug, Clone, Copy)]
pub struct ResourceScope<'a> {
    /// Instalação proprietária do recurso.
    pub installation_id: &'a str,
    /// Empresa proprietária, caso exista.
    pub company_id: Option<&'a str>,
}

/// Operações administrativas sujeitas a política explícita.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Consultar recurso.
    Read,
    /// Criar ou alterar recurso.
    Write,
    /// Administrar hierarquia.
    ManageHierarchy,
    /// Alterar permissões.
    ManagePermissions,
    /// Alterar identidade do proprietário.
    ChangeOwner,
}

/// Avaliador de permissões, sem privilégios implícitos por cargo/modelo.
pub struct AuthorizationPolicy;

impl AuthorizationPolicy {
    /// Autoriza o proprietário na instalação e funcionários apenas para
    /// leitura na própria empresa. Escritas de funcionários exigem concessões
    /// explícitas, ainda não implementadas, e portanto são negadas.
    pub fn authorize(
        context: &ActorContext,
        operation: Operation,
        resource: ResourceScope<'_>,
    ) -> Result<()> {
        if context.installation_id.is_empty()
            || resource.installation_id.is_empty()
            || context.installation_id != resource.installation_id
        {
            return Err(TinaError::Unauthorized("instalação não autorizada".into()));
        }

        match (&context.actor, operation) {
            (Actor::Owner, Operation::ChangeOwner) => Err(TinaError::PermissionDenied(
                "troca de proprietário requer fluxo de recuperação dedicado".into(),
            )),
            (Actor::Owner, _) => Ok(()),
            (Actor::Employee { company_id, .. }, Operation::Read)
                if resource.company_id == Some(company_id.as_str()) =>
            {
                Ok(())
            }
            (Actor::Employee { .. }, _) => Err(TinaError::PermissionDenied(
                "operação não concedida ao funcionário".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> ActorContext {
        ActorContext::verified("installation-a".into(), Actor::Owner)
    }

    fn employee() -> ActorContext {
        ActorContext::verified(
            "installation-a".into(),
            Actor::Employee {
                employee_id: "employee-a".into(),
                company_id: "company-a".into(),
            },
        )
    }

    fn scope<'a>(installation: &'a str, company: &'a str) -> ResourceScope<'a> {
        ResourceScope {
            installation_id: installation,
            company_id: Some(company),
        }
    }

    #[test]
    fn owner_can_manage_multiple_companies() {
        for company in ["company-a", "company-b"] {
            assert!(AuthorizationPolicy::authorize(
                &owner(),
                Operation::ManageHierarchy,
                scope("installation-a", company)
            )
            .is_ok());
        }
    }

    #[test]
    fn owner_cannot_cross_installations() {
        assert!(AuthorizationPolicy::authorize(
            &owner(),
            Operation::Read,
            scope("installation-b", "company-a")
        )
        .is_err());
    }

    #[test]
    fn employee_reads_only_own_company() {
        assert!(AuthorizationPolicy::authorize(
            &employee(),
            Operation::Read,
            scope("installation-a", "company-a")
        )
        .is_ok());
        assert!(AuthorizationPolicy::authorize(
            &employee(),
            Operation::Read,
            scope("installation-a", "company-b")
        )
        .is_err());
        assert!(AuthorizationPolicy::authorize(
            &employee(),
            Operation::Read,
            scope("installation-b", "company-a")
        )
        .is_err());
    }

    #[test]
    fn employee_cannot_escalate_permissions() {
        for operation in [
            Operation::Write,
            Operation::ManageHierarchy,
            Operation::ManagePermissions,
            Operation::ChangeOwner,
        ] {
            assert!(AuthorizationPolicy::authorize(
                &employee(),
                operation,
                scope("installation-a", "company-a")
            )
            .is_err());
        }
    }

    #[test]
    fn no_global_access_for_employee() {
        assert!(AuthorizationPolicy::authorize(
            &employee(),
            Operation::Read,
            ResourceScope {
                installation_id: "installation-a",
                company_id: None,
            }
        )
        .is_err());
    }

    #[test]
    fn owner_change_requires_dedicated_flow() {
        assert!(AuthorizationPolicy::authorize(
            &owner(),
            Operation::ChangeOwner,
            scope("installation-a", "company-a")
        )
        .is_err());
    }
}
