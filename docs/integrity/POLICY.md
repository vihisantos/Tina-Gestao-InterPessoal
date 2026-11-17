# Política de Integridade de Origem

**Escopo:** todo o repositório, em todos os tempos: código, testes, migrações, schemas,
dependências, manifests, lockfiles, configurações, workflows, scripts, documentação, mensagens de
commit, nomes de branch, títulos e corpos de PR, binários e artefatos.

## 1. O que é a política

A Tina está sendo reconstruída *clean-room*: novo material, escrito a partir de requisitos
funcionais aprovados pelo proprietário, sem nenhuma ligação com a implementação anterior.

São **proibidos** neste repositório, em qualquer formato e por qualquer caminho:

- Código-fonte, testes, scripts, migrações, schemas e configurações da implementação anterior,
  completos ou em trechos, inclusive adaptados ou "reescritos de memória".
- Arquivos de qualquer natureza provenientes daquela implementação: fontes, builds, executáveis,
  pacotes, instaladores, extrações ou dados recuperados.
- Dependências e lockfiles transportados, ou incompatíveis com o regime de distribuição decidido
  pelo proprietário.
- Manifests, workflows e documentação técnica daquela implementação sem reescrita e revisão
  originadas de requisitos aprovados.
- Nomes, marcadores, apelidos e identificadores da identidade técnica anterior — em conteúdo,
  caminhos, nomes de arquivo, branches, mensagens de commit ou textos de PR.

## 2. O que a política não contém — por regra, não por omissão

Este documento não lista marcadores, nomes, padrões ou hashes proibidos literalmente no próprio
repositório: fazê-lo documentaria a mesmíssima coisa que devemos bloquear.

O conjunto de evidências de origem proibida vive **fora deste repositório**, em mecanismo
externo de verificação mantido pelo proprietário. O repositório público contém apenas a política
(este arquivo) e a referência do mecanismo (a verificação de CI o invoca pelo nome público do
mecanismo, nunca pelo conteúdo da lista).

## 3. Como a verificação funciona

1. **Gate obrigatório:** um workflow de CI executa a verificação de integridade em todo push e PR.
   A verificação falhando bloqueia o merge; ela não depende de disciplina manual.
2. **Escopo:** conteúdo textual, nomes e caminhos de arquivos, manifests e lockfiles, scripts,
   configurações e documentação, mensagens de commit, nomes de branch, título e corpo de PR, e
   binários (rejeitados por padrão, salvo allowlist explícita aprovada pelo proprietário).
3. **Conteúdo proibido:** qualquer correspondência — trecho, variante ou adaptação — acusa
   violação e bloqueia o merge. O mecanismo usa impressões de conteúdo e similaridade semântica,
   não apenas busca por texto exato.
4. **Endereços e processamento:** a lista de material proibido, suas impressões e os limiares de
   similaridade ficam fora do repositório, protegidos no mecanismo externo. Este repositório não
   sabe, e não deve saber, o que há na lista.
5. **Sem exceções genéricas.** Um falso positivo é reportado como problema de segurança (canal
   privado de `SECURITY.md`), ajuste é feito no mecanismo externo pelo proprietário, e o repositório
   não ganha isenções no processo.

## 4. Obrigação de reportar

Qualquer um que detectar material de origem não aprovada no repositório reporta privadamente ao
proprietário (ver `SECURITY.md`). Remoção é tratada como incidente: avaliação, limpeza e registro —
nunca "apagar e seguir".

## 5. Estado dos mecanismos

Durante o bootstrap, o gate de integridade existe apenas como placeholder honesto no CI
(declarando o próprio estado). A versão inicial do mecanismo (verificação literal de conteúdo,
caminhos e dependências proibidos) entra no PR seguinte, com falha fechada, antes de qualquer
código de aplicação.
