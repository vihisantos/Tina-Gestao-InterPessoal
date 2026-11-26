# Política de segurança

A Tina tem código publicamente visível e permanece em desenvolvimento. Dados reais, segredos e detalhes de incidentes não pertencem ao repositório público.

## Reportando uma vulnerabilidade

**Nunca** abra issue, discussão ou PR público com detalhes de segurança.

Decisões de canal:

- Preferencialmente, use **GitHub Security Advisories** privados do repositório:
  `https://github.com/vihisantos/Tina-Gestao-InterPessoal/security/advisories/new`
- Se o Advisory não estiver disponível para a sua conta, contate o proprietário pelo canal
  privado da equipe. Não divulgue o problema por nenhum outro meio.

Inclua: o que foi encontrado, onde (caminho do arquivo, commit), impacto estimado, e como
reproduzir. Não inclua exploits prontos em material público depois de corrigido.

## O que está dentro do escopo agora

- Vazamento de segredos no histórico, no CI ou nos fluxos de contribuição.
- Violação da política de integridade: material de origem não aprovada no repositório.
- Qualquer código futuro com comportamento malicioso ou imprevisto introduzido por dependência.

## Superfícies da aplicação

- Autenticação, autorização, armazenamento local, migrações SQLite, cofre de credenciais, WebView, comandos Tauri e chamadas a provedores externos.

## Controles de publicação

- Usar somente executores hospedados para CI público; nunca executar contribuições públicas em máquinas pessoais. Os testes nativos são locais, em ambiente isolado e com dados sintéticos.
- Proteger a branch principal contra push direto, force push e merge sem revisão do proprietário.
- Habilitar secret scanning, push protection, relatórios privados de vulnerabilidades e alertas de dependências nas configurações do GitHub. A política não implica que essas opções já estejam ativas.
- Auditar todas as refs e o histórico, PRs, issues, logs e artefatos publicados; revogar ou rotacionar credenciais expostas. Remover arquivos do último commit não elimina cópias anteriores.
- Não incluir bancos, configurações reais, conversas, credenciais ou logs de produção em testes, PRs ou artefatos.

## Compromissos permanentes

- **Segredos nunca entram no repositório.** CI falha fechado em padrões de chave privada.
- **Dados locais permanecem locais.** Nenhuma telemetria é coletada sem decisão explícita do
  proprietário, com consentimento visível para o usuário final.
- **Integridade de origem é controle de segurança.** Um vazamento de código de origem não aprovada
  é tratado como incidente, não como "detalhe jurídico".

## Prazos e expectativas

Este é um projeto de um mantenedor, em desenvolvimento. Não há SLA contratual. Os relatórios são
analisados assim que possível, e você receberá confirmação de recebimento pelo mesmo canal privado.

## Reconhecimento

Agradecemos relatórios responsáveis. Com a sua autorização, o reconhecimento é feito no release que
incorporou a correção; nunca antes.

## Fora de escopo desta política

Questões de licença de terceiros entram em [`CONTRIBUTING.md`](CONTRIBUTING.md) e no registro de
proveniência — o canal de segurança trata de risco técnico e de acesso, e permanece estritamente
privado.
