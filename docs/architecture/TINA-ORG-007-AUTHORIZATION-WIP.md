# TINA-ORG-007 — revisão do primeiro acesso e segurança

## Implementado na branch
- Conta local de proprietário por instalação (nome e senha mestra).
- Hash Argon2id com salt exclusivo para a senha de login, persistido no SQLite.
- Chave independente e aleatória de 256 bits no Windows Credential Manager.
- Backup da chave cifrado por AES-256-GCM, com chave derivada por Argon2id da senha mestra, associado ao ID da instalação (migration 008).
- Login pode restaurar a chave ao cofre se a entrada estiver **ausente** e a senha mestre for válida. Falhas de permissão e entradas adulteradas não são sobrescritas.
- Migração do backup para contas que existiam antes da migration 008 ao autenticar com o cofre disponível.
- Tela SvelteKit para primeiro acesso, login e bloqueio.
- Sessão efêmera em Rust com expiração; bloqueio temporário de tentativas inválidas persistido em SQLite.
- 25 comandos administrativos Tauri passam por require_owner; os IDs enviados pelo frontend não constituem identidade.
- Cadastro concorrente serializado por transação SQLite antes da escrita no cofre.
- CSP Tauri habilitada em release, com regras de HMR apenas no dev.
- CI usa Cargo.lock sincronizado, formatação, frontend, cargo check/test --locked e smoke específico do vault Windows.

## Pendências antes de concluir a TINA-ORG-007
1. Integrar autorizações contextuais também aos serviços e repositórios Rust; hoje a barreira operacional está nos comandos Tauri.
2. Criar concessões e revogações granulares persistidas para agentes de IA, que ainda não podem executar ações privilegiadas.
3. Auditoria de operações sensíveis e de negativas (issue #8).
4. Cobertura de integração IPC negativa e isolamento de múltiplas instalações.
5. Smoke manual na instalação Windows real, especialmente CSP dev/release, backup e recuperação.
6. Futuro: exportação/importação portátil guiada da instalação e do banco; o backup local cifra a chave mas não é um arquivo de exportação.

## Limites importantes
- O SQLite **ainda não é criptografado** e os seus dados podem ser lidos por quem possuir acesso ao arquivo.
- O backup cifrado da chave no SQLite permite reconstruir o segredo com o banco, ID da instalação e a senha mestre; uma senha fraca aumenta o risco de tentativa de força bruta offline.
- Sem o arquivo SQLite de backup, o segredo guardado apenas no Windows Credential Manager não pode ser recuperado em outro perfil.
- Acesso malicioso como o usuário do Windows ou como administrador do SO não está integralmente contido.
- Não fazer merge antes de completar os bloqueios acima.
