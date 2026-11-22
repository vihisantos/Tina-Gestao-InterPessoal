# TINA-ORG-007 — autorização: implementação incremental

## Entregue nesta branch
- Política Rust `AuthorizationPolicy` com negação por padrão.
- Contexto `ActorContext` não desserializável, com construtor restrito ao crate.
- Proprietário pode consultar/administrar recursos de qualquer empresa **da mesma instalação**.
- Funcionário só pode consultar recursos de sua própria empresa; escrita e administração negadas até concessões explícitas.
- Alteração do proprietário bloqueada até fluxo dedicado.
- Seis testes unitários de escopo e escalada.

## PENDÊNCIAS — não considerar esta issue concluída
1. Implementar autenticação local real e resolver identidade confiável por operação; UUID do config não é credencial.
2. Integrar o verificador a **todos** os comandos Tauri e serviços/repositórios protegidos; hoje a política é isolada.
3. Criar permissões granulares persistidas e mecanismos de concessão/revogação.
4. Auditar decisões negadas e operações sensíveis (issue #8).
5. Adicionar testes de integração SQLite/IPC e executar CI.

## Segurança
**NÃO** instanciar `ActorContext::verified` a partir de `company_id`, `employee_id` ou `owner_uuid` recebidos do frontend. Não há sessão autenticada nesta etapa. A política é uma fundação, não uma barreira ativa nos comandos existentes.

Nenhum provedor de IA, backend remoto, HTTP server ou instalador foi adicionado.
