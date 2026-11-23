# TINA-ORG-007 — autenticação local do proprietário (em andamento)

## Esta branch implementa
- Cadastro inicial de uma conta local de proprietário por instalação.
- Hash Argon2id com salt criptograficamente aleatório para a senha de login.
- Geração de chave independente de 256 bits, guardada no Windows Credential Manager.
- Sessão em memória com expiração e bloqueio automático por tentativas inválidas.
- Tela inicial de cadastro, login e encerramento de sessão em SvelteKit.
- 25 comandos IPC organizacionais bloqueados até login do proprietário.
- Identidade do proprietário validada no backend (não aceitar UUIDs da UI).
- Os dados ficam no dispositivo; não existe serviço externo de login.

## Ainda não concluído
1. Permissões granulares de funcionários de IA, concessão e revogação persistidas.
2. Política de autorização integrada também nos serviços e repositórios.
3. Auditoria transacional das operações sensíveis (issue #8).
4. Testes automatizados de IPC e integração com Windows Credential Manager real.
5. Política de recuperação e exportação de chave e revisão de segurança.

## Limites de segurança
- O SQLite AINDA NÃO É CRIPTOGRAFADO. A chave do vault é uma reserva para futura feature de criptografia do banco.
- O cofre pertence ao perfil do Windows. Perder perfil/cofre pode perder a chave; precisa haver recuperação antes de uso real com banco criptografado.
- Uma pessoa com acesso à sessão do Windows antes do primeiro cadastro pode registrar a conta inicial.
- Ameaças do próprio perfil Windows e alterações diretas ao arquivo SQLite ainda estão fora das garantias desta implementação.
- Não fazer merge da PR enquanto a autorização completa e os testes ainda estiverem pendentes.
