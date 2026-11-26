# Reparo auditável de checksums de migrations

## Quando usar

Use este procedimento **somente** quando a inicialização falhar com:

```text
migration N was previously applied but has been modified
```

e o diagnóstico mostrar que a diferença é **exclusivamente** de quebra de linha
(`LF` ↔ `CRLF`) nos arquivos de `migration/*.sql`. O SQLx 0.8.6 calcula o
checksum com `SHA-384` sobre os bytes exatos do SQL; por isso o mesmo arquivo
com `CRLF` e com `LF` produz hashes diferentes.

O banco ativo da instalação atual (`%APPDATA%\tina\tina.db`) está consistente
com os arquivos embutidos no executável e **não precisa ser reparado**. Não
execute a ferramenta “por precaução”.

## O que a inicialização faz

A inicialização normal (`TinaApp::initialize()` → `Database::open()`) é
rigorosa:

- não repara nada automaticamente;
- não cria backup automaticamente;
- não há variável de ambiente para autorizar reparo no startup;
- ao detectar divergência apenas de `LF/CRLF`, aborta e informa o comando da
  ferramenta explícita;
- ao detectar qualquer outra divergência, aborta sem tocar no banco.

## Como reparar

Execute no raiz do repositório, com a Tina **fechada**:

```powershell
cargo run --example repair_migration_checksums -- "$env:APPDATA\tina\tina.db"
```

Para outro banco:

```powershell
cargo run --example repair_migration_checksums -- "C:\caminho\para\tina.db"
```

A ferramenta:

1. lê `_sqlx_migrations` e os SQLs embutidos em `migration/`;
2. aceita somente checksums que são o SHA-384 do mesmo SQL nas variantes
   exatas, `LF`, `CRLF` ou a forma já embutida;
3. rejeita qualquer SQL realmente alterado, sem criar backup e sem gravar;
4. cria `<pasta-do-banco>\backups\migration-checksum-<nanos>.db` com
   `VACUUM INTO`, validando `PRAGMA integrity_check`, contadores de todas as
   tabelas e o próprio journal de migrations;
5. abre `BEGIN IMMEDIATE`, confirma que `PRAGMA data_version`, os contadores e
   o journal não mudaram desde o backup e atualiza somente
   `_sqlx_migrations.checksum` em uma transação atômica;
6. valida novamente todos os checksums aplicados e confirma que o SQLx aceita
   as migrations.

Saída esperada quando há reparo:

```text
Versões reparadas: [2, 3, 4, 5, 6]
Backup verificado: C:\Users\...\tina\backups\migration-checksum-....db
Inicialização validada: SQLx aceitou todas as migrations.
```

Códigos de saída:

- `0`: reparo concluído (ou nada a reparar) e validação final OK;
- `2`: divergência não reparável ou escrita concorrente — nenhuma alteração;
- `3`: reparo aplicado, mas a validação final falhou — investigar o backup.

## O que nunca é reparado

- SQL com conteúdo diferente de uma troca `LF`/`CRLF`;
- migration marcada como falha (`success = 0`) no journal;
- versões presentes no banco que não existem no binário atual;
- banco modificado por outro processo entre o backup e a transação.

Nesses casos, o backup não é apagado e a decisão continua sendo manual. Um
exemplo real é o backup legado `001_initial.sql` com checksum `131f65b5…`:
isso não é variante de quebra de linha do arquivo atual e, portanto, é
rejeitado pela ferramenta.

## Prevenção

`.gitattributes` força `/migration/*.sql` a serem checkoutados com `LF`:

```gitattributes
/migration/*.sql text eol=lf
```

Isso impede novo drift em checkouts do Windows. Para bancos já gravados com
`CRLF`, a ferramenta continua sendo necessária.

## Banco e migrations usados pelo executável final

- O caminho vem de `database_path` em `%APPDATA%\tina\tina.toml`.
  Na instalação atual: `C:\Users\MedRoom3\AppData\Roaming\tina\tina.db`.
- Se `database_path` ausente, o padrão é `%APPDATA%\tina\tina.db`.
- As migrations são incorporadas em tempo de compilação por
  `sqlx::migrate!("./migration")`; o executável não lê `.sql` do disco.

Migrations embutidas na versão final:

| Versão | Arquivo |
|---:|---|
| 1 | `001_initial.sql` |
| 2 | `002_employee_deactivated_at.sql` |
| 3 | `003_employee_status_column.sql` |
| 4 | `004_employee_status_sync.sql` |
| 5 | `005_employee_supervisor_history.sql` |
| 6 | `006_employee_assignment_history.sql` |
| 7 | `007_owner_account.sql` |
| 8 | `008_owner_key_recovery.sql` |
| 9 | `009_organizational_audit_events.sql` |
| 10 | `010_chat_sessions.sql` |
| 11 | `011_audit_owner_context.sql` |

## Verificação rápida

```powershell
python -c "import os,sqlite3; p=os.path.join(os.environ['APPDATA'],'tina','tina.db'); c=sqlite3.connect('file:'+p.replace('\\\\','/')+'?mode=ro',uri=True); print(c.execute('PRAGMA integrity_check').fetchone()[0]); print(c.execute('SELECT version FROM _sqlx_migrations ORDER BY version').fetchall())"
```

Se retornar `ok` e as versões `1..11`, o banco está consistente e não deve ser
modificado.
