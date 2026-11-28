# Referências de Produto — Tina

**Status:** catálogo inicial de pesquisa  
**Objetivo:** registrar produtos usados como referência de experiência, capacidades e padrões arquiteturais para orientar decisões próprias da Tina.  
**Regra:** referência não significa autorização para copiar código, assets, textos, marcas, documentação proprietária ou desenho protegido. A Tina mantém implementação *clean-room*, conforme `CONTRIBUTING.md` e `docs/provenance/POLICY.md`.

## Como usar este catálogo

Cada referência deve registrar:
- **Fonte primária:** URL oficial e, quando possível, arquivo/documentação específica.
- **O que foi observado:** somente capacidades confirmadas pela fonte.
- **O que interessa à Tina:** hipótese de produto, não compromisso de implementação.
- **Adaptação Tina:** como a capacidade se encaixaria na arquitetura e nos princípios locais.
- **Licença e limites:** licença vigente na versão consultada e restrições relevantes.
- **Estado:** `a pesquisar`, `pesquisado`, `proposta`, `aprovado`, `implementado` ou `descartado`.
- **Data da verificação:** a licença e o produto podem mudar; revisar antes de qualquer incorporação.

Nenhuma referência autoriza reutilização de código. Qualquer dependência, código de terceiro ou asset exige análise de licença, aprovação e registro próprio em `docs/provenance/REGISTER.md`.

---

## 1. OpenBot — Nightly Labs

- **Projeto:** OpenBot
- **Fonte principal:** https://github.com/nightly-labs/openbot
- **Site:** https://openbot.run/
- **README consultado:** https://github.com/nightly-labs/openbot/blob/main/README.md
- **Estado:** pesquisado — referência prioritária
- **Data da verificação:** 2026-11-28

### O que a fonte confirma

O repositório descreve o OpenBot como um workspace desktop *local-first* para colegas/agentes de IA persistentes. A descrição do projeto destaca workspaces dedicados, filas de tarefas, compartilhamento de arquivos, controle de navegador e colaboração entre agentes.

O README também documenta, entre outros pontos:
- uma conversa local estável por agente ao alternar entre provedores, mantendo identificadores nativos de sessão do provedor como detalhe interno;
- workspaces e dados locais associados aos agentes;
- compartilhamento/transferência de arquivos e entregas;
- canais e colaboração entre agentes;
- browser embutido em sessão isolada;
- opção de acesso remoto com serviço de conta configurável, transporte WebRTC e API local autenticada;
- rotinas/webhooks e detalhes operacionais de distribuição.

Esses itens descrevem o produto OpenBot; não são afirmações de que a Tina já os implemente.

### O que interessa à Tina

1. **Identidade persistente por funcionário:** um funcionário deve continuar sendo a mesma entidade ao longo de várias sessões de chat.
2. **Workspace individual:** separar arquivos, estado operacional e ferramentas de cada funcionário.
3. **Fila de mensagens/tarefas:** evitar concorrência desordenada quando um agente recebe novas instruções durante uma execução.
4. **Colaboração entre funcionários:** comunicação e transferência de trabalho com remetente, destinatário, escopo e trilha de auditoria.
5. **Compartilhamento de arquivos:** transferência explícita e autorizada, sem transformar pastas de agentes em um espaço comum irrestrito.
6. **Navegador isolado:** sessão e permissões de navegação separadas por agente ou workspace.
7. **Conversa independente do provedor:** histórico de produto da Tina não deve depender do identificador de sessão de um fornecedor de modelo.
8. **Execução local-first:** preservar controle local dos dados e deixar claro quando uma capacidade exige serviço remoto.

### Adaptação proposta para a Tina (ainda não aprovada)

- O proprietário navega pela árvore **Organizações → Departamentos → Funcionários**; a conversa e o histórico ficam dentro da identidade do funcionário, não na navegação global.
- Cada funcionário possui sessões próprias, memória própria, workspace e estado de execução próprios.
- Tina Central mantém um chat próprio e pode acessar funcionários por autoridade de proprietário, inclusive fora da cadeia hierárquica.
- Filas devem distinguir mensagens conversacionais, ordens de trabalho e eventos de sistema; definir política de interrupção, cancelamento, retomada e idempotência.
- Delegações continuam vinculadas à hierarquia e à empresa; colaboração não pode furar isolamento entre empresas/instalações.
- Compartilhamento de arquivos exige concessão explícita, registro de origem e auditoria.
- Contexto da sessão, memória permanente e histórico bruto devem ser camadas distintas. Resumos/compactações precisam manter referências às mensagens e artefatos de origem.
- O navegador e ferramentas devem respeitar o modelo de permissões da Tina e não herdar privilégios do proprietário automaticamente.

### Licença e limites

O README atual declara **PolyForm Noncommercial License 1.0.0** para a versão corrente e informa que versões até e incluindo **0.1.11** permanecem sob **Apache-2.0**. O projeto informa que uso comercial requer licença separada do titular. A Tina não deve incorporar código da versão atual sem autorização/licença comercial apropriada. A licença deve ser revalidada diretamente no arquivo LICENSE antes de qualquer uso de código.

**Decisão atual:** usar somente como referência de pesquisa de produto e arquitetura; não copiar código, documentação extensa, assets ou interface.

---

## 2. Grok Bot — xAI

- **Projeto:** Grok Bot / AI teammates
- **Fonte inicial:** https://x.ai/
- **Estado:** a pesquisar
- **Data da verificação:** não verificado nesta edição

### Foco de pesquisa

- experiência de conversar com colegas/agentes de IA;
- identidade persistente e presença dos agentes;
- delegação e acompanhamento de trabalho;
- integração com ferramentas e ambiente de execução;
- apresentação de atividade e estado do agente.

### Perguntas para a próxima pesquisa

- Como a navegação entre equipes e agentes é organizada?
- Como o produto separa chats, tarefas, memória e execução?
- Que controles o proprietário tem sobre ações e agentes?
- O que é realmente persistente e o que é apenas estado de sessão?

**Decisão atual:** nenhuma capacidade específica aprovada; pesquisar fontes oficiais antes de pontuar.

---

## 3. Muse

- **Projeto:** Muse
- **Fonte inicial:** a identificar e confirmar
- **Estado:** a pesquisar
- **Data da verificação:** não verificado nesta edição

### Foco de pesquisa

- agentes que trabalham continuamente ou em segundo plano;
- objetivos, planos e acompanhamento;
- aprovações humanas e controle de ações;
- histórico operacional e observabilidade.

**Decisão atual:** nenhuma capacidade específica aprovada; confirmar qual produto chamado “Muse” é a referência pretendida e localizar documentação oficial.

---

## 4. Dots — OpenAI

- **Projeto:** Dots
- **Fonte inicial:** documentação oficial do produto OpenAI
- **Estado:** a pesquisar
- **Data da verificação:** não verificado nesta edição

### Foco de pesquisa

- agentes pessoais persistentes e proativos;
- continuidade de trabalho entre conversas;
- uso de computador/navegador e aplicativos conectados;
- progresso entre sessões e controles do usuário.

**Decisão atual:** nenhuma capacidade específica aprovada; pesquisar documentação oficial e disponibilidade atual.

---

## 5. Nod

- **Projeto:** Nod
- **Fonte inicial:** a identificar e confirmar
- **Estado:** a pesquisar
- **Data da verificação:** não verificado nesta edição

### Foco de pesquisa

- agente pessoal contínuo;
- memória e proveniência das lembranças;
- computador, navegador e ferramentas;
- permissões, credenciais e aprovações.

**Decisão atual:** nenhuma capacidade específica aprovada; confirmar a identidade exata do projeto e suas fontes primárias.

---

# Auditoria inicial: o que já existe na Tina

Esta seção registra o estado observado no repositório Tina consultado em 2026-11-28. É uma leitura estática de arquivos; não equivale a executar testes nem comprova comportamento em runtime.

## Organização e hierarquia

- O domínio já possui instalações, empresas, departamentos, cargos e funcionários.
- O funcionário está ligado ao cargo; a empresa é obtida pela relação cargo → departamento → empresa.
- `employees.supervisor_id` é a relação atual de superior direto e aceita profundidade variável; há histórico append-only de alterações hierárquicas.
- O isolamento entre empresas deve ser validado no backend Rust; IDs recebidos da interface são seletores, não prova de autorização.
- O modelo documentado determina o proprietário como autoridade máxima, mas os próprios documentos alertam que autenticação/autorização completa do proprietário ainda não está implementada.

Fontes internas:
- `docs/architecture/ORGANIZATIONAL-DOMAIN.md`
- `docs/architecture/COMPANY-ISOLATION.md`
- `docs/architecture/AUTHORITY-MODEL.md`

## Chat e contexto

- A migration `010_chat_sessions.sql` cria `chat_sessions` e `chat_messages`, ambas isoladas por `installation_id`.
- A estrutura atual não associa sessões a `employee_id`; portanto, ainda representa o histórico do chat principal da instalação, não um histórico independente por funcionário.
- `src/infra/chat.rs` oferece criação, renomeação e listagem de sessões, persistência de mensagens e seleção de uma janela cronológica recente limitada por bytes.
- A seleção atual é uma janela de sufixo do histórico: não é compactação semântica, não cria pacote de continuidade e não calcula utilização real de tokens do modelo.
- `src/context/selector.rs`, `distributor.rs` e `consolidator.rs` contêm estruturas iniciais e trechos marcados como TODO; não devem ser tratados como sistema completo de memória/contexto.
- A interface possui componentes de histórico e espaço de funcionário, mas a auditoria desta edição não encontrou evidência suficiente para afirmar que cada funcionário já tenha seu próprio chat persistente.

Fontes internas:
- `migration/010_chat_sessions.sql`
- `src/infra/chat.rs`
- `src/context/selector.rs`
- `src/context/distributor.rs`
- `src/context/consolidator.rs`
- `src-frontend/src/lib/components/ChatHistory.svelte`
- `src-frontend/src/lib/components/EmployeeSpace.svelte`

## Implicações para o desenho da próxima fase

1. Preservar as entidades organizacionais existentes; não duplicar empresa em `employees` sem ADR e justificativa.
2. Evoluir o chat de instalação para chat pertencente a um ator/agente, mantendo isolamento por instalação e escopo empresarial.
3. Tratar Tina Central como identidade de agente distinta dos funcionários, mas subordinada à autoridade do proprietário.
4. Fazer da navegação principal uma árvore de organizações, departamentos e funcionários; o histórico de sessões aparece dentro do agente selecionado.
5. Criar sessões encadeáveis: uma sessão pode continuar outra, com snapshot versionado e referências às mensagens-fonte.
6. Medir contexto com o tokenizer/limites do modelo efetivamente utilizado, reservando espaço para system prompt, ferramentas e resposta; bytes podem ser fallback, não métrica apresentada como tokens.
7. Não confundir sessão, memória permanente, workspace, fila de tarefas e log de auditoria.
8. Planejar filas e execução antes de adicionar colaboração multiagente; mensagens precisam de estados, ordenação, idempotência, cancelamento e política de retomada.
9. Nenhuma mudança de schema deve editar migrations publicadas; adicionar migrations novas e testes de instalação/isolamento/recuperação.
10. Validar todas as mudanças com branch e PR; nunca escrever diretamente em `main`.

## Próximas pesquisas prioritárias

1. OpenBot: filas de mensagens, ciclo de vida de tarefas, contexto/compactação, workspaces e segurança do browser.
2. Grok Bot: experiência de navegação, presença e gestão dos agentes.
3. Muse, Dots e Nod: confirmar produtos exatos e pesquisar fontes primárias antes de registrar funcionalidades.
4. Comparar cada padrão com a arquitetura local-first, Windows-only, SQLite e NVIDIA NIM já documentada para a Tina.

---

## Histórico de decisões

| Data | Decisão | Estado |
|---|---|---|
| 2026-11-28 | Adotar este arquivo como catálogo central de referências de produto; referências não autorizam cópia de implementação | Registrado |
| 2026-11-28 | Corrigir OpenBot para `nightly-labs/openbot` | Confirmado |
| 2026-11-28 | Usar OpenBot como referência prioritária para estudar workspace, filas, colaboração e continuidade | Em pesquisa |
