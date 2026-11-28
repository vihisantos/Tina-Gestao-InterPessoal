# Motor central de orquestração de ferramentas — relatório técnico

Branch: `feat/R-5-rh-agent-hiring` (PR #74, draft). Nenhum merge foi feito em
`main`; a PR #73 permanece aberta e draft. O banco principal em
`%APPDATA%\tina\tina.db` e a instância isolada de validação não foram alterados
por este trabalho.

## 1. Escopo entregue

- Parser multi-call com acumulação por `index`, validação de tipo/nome/JSON e
  limites de 8 chamadas por rodada, 256 B de `id`, 100 B de nome, 16 KB de
  argumentos por chamada e 64 KB de argumentos por rodada.
- Separação entre **planejamento**, **autorização**, **execução** e
  **histórico**, em `src/orchestration/` (`calls`, `dependencies`, `plan`,
  `store`, `history`, `executor`), reutilizável por chat, funcionários IA e
  futuros apps.
- Plano imutável por `(installation_id, session_id)`, com fingerprint
  autorizável. Uma nova mensagem com ferramentas falha explicitamente enquanto
  existe plano ativo; a substituição é uma operação explícita e auditada.
- Validação determinística no backend: ferramentas catalogadas, justificativa
  obrigatória, alvos organizacionais inequívocos, ordem topológica e
  dependências fixas organização → departamento → cargo → minuta.
- Referências simbólicas tipadas `@id.campo` (`id`, `name`, `company_id`,
  `code`, `status`) resolvidas somente após a etapa de origem ser executada e
  autorizada.
- Execução sequencial com retomada: etapas `executed`, `failed`, `skipped` e
  `cancelled` nunca são repetidas. A continuação textual usa
  `enable_tools = false`; qualquer ampliação do plano exige nova autorização.
- Cancelamento por sinal `watch` mais persistência, com revalidação da sessão
  do proprietário antes de cada etapa e antes da retomada.
- Histórico compatível com NIM: pedido gravado como `assistant.tool_calls` e
  resultados convertidos para `role:"tool"` no payload, mantendo o SQLite com
  `role IN ('user','assistant')`.
- Cartão de plano na UI com ações, justificativas/efeitos, dependências e
  limites; aprovação vinculada ao fingerprint. Mensagens internas de protocolo
  são rotuladas na UI, sem exibir JSON bruto.
- Migration `021_tool_orchestration.sql`, aditiva e forward-only; nenhuma
  migration publicada `001`–`020` foi editada.

## 2. Decisões Relevantes

- `lifecycle_status` continua sendo a fonte autoritativa das propostas; o campo
  `status` permanece como espelho legado.
- Chamada única continua no caminho legado de proposta; múltiplas chamadas
  exigem plano autorizado.
- Propostas avulsas pendentes são marcadas como `superseded_by = plan_id` quando
  um plano é criado, com evento de auditoria.
- Falha parcial preserva efeitos já gravados, interrompe dependentes e nunca
  afirma rollback.
- `ExecutionSummary` não cruza a fronteira Tauri; o DTO `PlanDecision` usa
  `camelCase`.
- O ajuste local `TINA_TEST_DATA_DIR` em `src/config.rs` não foi publicado e
  permanece fora do índice; ele só permitiu abrir a instância isolada sem
  tocar no banco principal.

## 3. Evidências de validação

Executado no clone isolado da branch, após todas as alterações:

| Comando | Resultado |
|---|---|
| `cargo fmt --all` | limpo |
| `cargo check --all-targets` | ok (apenas warnings pré-existentes) |
| `cargo test --all-targets` | 185 passed, 0 failed, 1 ignored |
| `npm run frontend:check` | 0 erros, 0 avisos |
| `npm run frontend:test` | 9 arquivos, 50 testes passed |
| `npm run frontend:build` | build estático concluído |
| `cargo build -p tina-desktop` | ok |
| CI remoto `45902133556` | `completed` / `success` |

Testes específicos do orquestrador cobrem: fingerprint imutável e plano ativo
único; execução em ordem com referência simbólica; falha parcial com efeitos
preservados e dependentes bloqueados; cancelamento explícito e por sinal;
retomada sem repetir etapas; isolamento por instalação; e compatibilidade do
histórico com o protocolo NIM. O teste de upgrade `v20 → v21` preserva conversas
antigas e valida `integrity_check = ok`.

Estado das PRs verificado após a implementação: #74 `OPEN`, draft,
`mergeStateStatus CLEAN`; #73 `OPEN`, draft.

## 4. Limitações

- A validação de múltiplas chamadas **reais** contra a NVIDIA depende da chave
  configurada na instância isolada. O cofre isolado ainda não possui a chave; o
  valor não foi lido, exibido ou copiado em nenhum momento. O proprietário deve
  salvar a chave pela janela Tauri (`Salvar no cofre Windows`) e então repetir
  o cenário multi-call de ponta a ponta.
- O critério de `nvidia_evaluation::tool_passed()` permanece rígido para uma
  chamada por rodada; a avaliação sintética não certifica o runtime do
  orquestrador. Os testes de runtime cobrem o fluxo com chamadas simuladas.
- A divergência histórica da `001_initial.sql` entre `d42d7e1` e `e5c070c`
  continua adiada. Distribuir para bancos legados exige estratégia auditável
  com backup e confirmação explícita; nenhum banco foi apagado ou reparado
  automaticamente.
- `chat_generation` é um mutex global com `try_lock`; plano e `chat_generate`
  simultâneos devem ainda passar por verificação manual na instância isolada.
- O fingerprint usa `DefaultHasher`: é estável dentro da versão compilada e
  suficiente para vincular a decisão ao plano apresentado, mas não é uma
  assinatura criptográfica nem garantia de estabilidade entre versões.

## 5. Riscos e mitigções

| Risco | Mitigação implementada |
|---|---|
| Modelo emitir chamadas fora do catálogo | Validação contra `definitions()` antes de criar o plano |
| Dependências forjadas pelo modelo | Ordem e dependências de negócio derivadas no backend; topological sort |
| Substituição silenciosa de plano pendente | Índice único de plano ativo e erro explícito no novo turno |
| Reexecução após interrupção | Estado persistido por etapa é a fonte da verdade; `claim_step` é idempotente |
| Aprovação de plano alterado | Fingerprint recalculado e confrontado na autorização, início e retomada |
| Sessão expirada durante execução | Marcador revalidado antes de cada etapa e na retomada |
| Exposição de credenciais | Chave apenas via cofre `Tina.NVIDIA` na janela Tauri; nunca persistida no histórico |
| Afirmação indevida de rollback | Mensagens de status dizem explicitamente que efeitos gravados não são desfeitos |

## 6. Próximos passos operacionais

1. Salvar a chave NVIDIA na instância isolada pela janela Tauri e validar um
   pedido com duas ou mais chamadas, incluindo aprovação, cancelamento e
   retomada.
2. Observar o cartão de plano, confirmando que nenhum JSON de protocolo aparece
   no histórico.
3. Publicar os commits restantes na branch sem merge e sem force push.
4. Antes de distribuir a usuários com bancos legados, resolver a divergência da
   migration `001` com reparo forward-only auditável e backup explícito.
