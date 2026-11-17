# TECHNOLOGY_DECISIONS.md

**Versão:** 1.0.0 (2026-11-17)  
**Status:** Decisões técnicas para Fase 0  

---

## 1. Decisão: Linguagem de Programação

### Opção Considerada

| Linguagem | Prós | Contras |
|-----------|------|---------|
| **Rust** | Zero-cost abstractions, memory safety, single binary | Curva de aprendizado íngreme |
| Python | Ecossistema AI maduro, simplicidade | Runtime, performance |
| TypeScript/Node.js | Frontend compatível, ecossistema | Runtime, single-threaded |
| Go | Simples, concorrente, bom para web | Menos opções de AI |

### Decisão: **Rust**

### Justificativa

1. **Local-first:** Rust compila para binário único, sem dependência de runtime
2. **Segurança:** Contrato de segurança sem GC, ideal para dados sensíveis
3. **Performance:** Zero-cost abstractions para latência baixa
4. **Cross-platform:** Windows, Linux, macOS com mesmo código
5. **Alinhamento com requisitos:** Local-first, sem backend obrigatório

---

## 2. Decisão: Banco de Dados

### Opção Considerada

| Banco | Prós | Contras |
|-------|------|---------|
| **SQLite** | Zero dependência, local, embedido | Escala limitada, sem vector search nativo |
| PostgreSQL | ACID, pgvector, robustez | Requer instalação separada |
| MySQL | Amplamente usado | Não ideal para AI/vetores |

### Decisão: **SQLite**

### Justificativa

1. **Local-first absoluto:** Nenhuma instalação separada necessária
2. **Zero custo operacional:** Arquivo único em `tina.db`
3. **Transações ACID:** SQLx com SQLite suporta transações
4. **Pgvector equivalente:** Implementaremos busca vetorial em Rust com `rust-bert` ou `tch`
5. **Backup simples:** Copia do arquivo `tina.db`

### Estratégia Vector Search

```rust
// Implementação alternativa para SQLite
pub struct VectorStore {
    db: SqlitePool,
    embedding_model: EmbeddingModel, // BGE-small ou similar
}

impl VectorStore {
    pub async fn search(&self, query: &str, limit: usize) -> Vec<Memory> {
        let embedding = self.embedding_model.embed(query).await?;
        // Busca porSimilaridade coseno usando Código personalizado
        // ou via tabela de embeddings pré-calculados
    }
}
```

---

## 3. Decisão: Web Framework

### Opção Considerada

| Framework | Prós | Contras |
|-----------|------|---------|
| **Actix Web** | Performance, WebSocket nativo | Complexidade |
| Axum | Moderno, Tokio integrado | Menos documentação |
| Warp | Filtros funcionais | Menos usado |
| Tide | Simples | Menos maturidade |

### Decisão: **Actix Web 4**

### Justificativa

1. **Performance líder:** Benchmark mais rápido que concorrentes
2. **WebSocket nativo:** WebSocket + HTTP em mesmo servidor
3. **Middleware robusto:** Auth, CORS, logging
4. **Estabilidade:** Framework maduro desde 2018
5. **Excelente para Rust:** Compatibilidade total com ecosystem

---

## 4. Decisão: Frontend Framework

### Opção Considerada

| Framework | Prós | Contras |
|-----------|------|---------|
| **Tauri + SvelteKit** | Binário pequeno, sem Node.js | Curva de aprendizado |
| Tauri + React | Ampla documentação | Binário maior |
| Electron | Ecossistema maior | Binário enorme, Node.js |

### Decisão: **Tauri 2 + SvelteKit**

### Justificativa

1. **Tamanho do binário:** ~15MB vs ~150MB do Electron
2. **Sem Node.js runtime:** Alinha com local-first
3. **WebView otimizado:** Usa motor do sistema
4. **Plugin system:** Rust plugins para performance
5. **Segurança:** Contexto limitado, sem acesso direto ao filesystem via JS

---

## 5. Decisão: Armazenamento de Embeddings

### Estratégia: **Cache híbrido**

**Problema:** pgvector não está disponível em SQLite no Windows sem instalação separada.

### Solução Proposta

```rust
// 1. Embeddings calculados e armazenados como JSON no SQLite
// 2. Índice de similaridade usando FAISS-lite em Rust ou custom
// 3. Cache em disco para embeddings frequentemente usados

pub struct Embedder {
    model: SentenceTransformer, // BGE-small ou similar
    cache: DiskCache,           // LRU no disco
}

pub struct VectorIndex {
    // Implementação HNSW ou IVF em memória ou em arquivo
    // Para escala futura
}
```

### Alternativas Consideradas

1. **Usar `duckdb` com extensions** - Menos maduro no Windows
2. **Implementar B-tree para embeddings** - Mais controle total
3. **Cache em Redis (opcional)** - Quando necessário escalar

---

## 6. Decisão: Armazenamento Seguro de Credenciais

### Opção Considerada

| Método | Prós | Contras |
|--------|------|---------|
| Windows Credential Manager | Integrado ao SO | Windows-only |
| Linux Secret Service | Padrão Linux | Não Windows |
| Arquivo criptografado | Cross-platform | Chave de master |
| Keyring crate | Abstraction layer | Depende do SO |

### Decisão: **Keyring (cross-platform)**

### Implementação

```rust
use keyring::Entry;

pub struct CredentialStore {
    entry: Entry,
}

impl CredentialStore {
    pub fn new(service: &str, account: &str) -> Result<Self> {
        Ok(Self {
            entry: Entry::new(service, account)?,
        })
    }

    pub fn store(&self, secret: &str) -> Result<()> {
        self.entry.set_secret(secret)?;
        Ok(())
    }

    pub fn retrieve(&self) -> Result<String> {
        self.entry.get_secret()
    }
}
```

### Fallback: **Arquivo criptografado com Argon2id**

Se o Keyring falhar, usaremos:
- Arquivo `tina.secrets` criptografado
- Derivation key de senha do usuário
- Algoritmo: Argon2id + AES-256-GCM

---

## 7. Decisão: Tratamento de Erros

### Opção Considerada

| Crate | Prós | Contras |
|-------|------|---------|
| **anyhow + eyre** | Simplicidade, pretty errors | Menos tipos |
| thiserror | Tipos fortes | Mais boilerplate |
| snafu | Guia completo | Adicional |

### Decisão: **anyhow + thiserror**

### Justificativa

1. **anyhow:** Para erros de runtime, transparência fácil
2. **thiserror:** Para erros de domínio, tipos fortes
3. **eyre:** Alternative leve, pode ser usado como fallback

---

## 8. Decisão: Logging e Observability

### Opção Considerada

| Crate | Prós | Contras |
|-------|------|---------|
| **tracing** | Async, hierarchical, structured | Curva de aprendizado |
| log | Simple | Sincrono, menos informação |
| env_logger | Simples | Limitado |

### Decisão: **tracing + tracing-subscriber**

### Justificativa

1. **Structured logging:** JSON para análise
2. **Async support:** Não bloqueia aplicação
3. **Level filtering:** Desenvolvimento vs Produção
4. **OpenTelemetry compatible:** Futuro evolutivo

---

## 9. Decisão: Serialização de Estado

### Opção Considerada

| Formato | Prós | Contras |
|---------|------|---------|
| **JSON** | Universal, simples | Verboso |
| **MessagePack** | Compacto | Menos legível |
| **BinCode** | Muito rápido | Não human-friendly |
| **YAML** | Legível | Mais lento |

### Decisão: **JSON + Serde**

### Justificativa

1. **Interoperabilidade:** Facilidade de debug
2. **Serde:** Padrão Rust
3. **JSON Schema:** Validação de entrada
4. **Compacto o suficiente:** Para configurações e mensagens

---

## 10. Decisão: Concurrency Model

### Opção Considerada

| Modelo | Prós | Contras |
|--------|------|---------|
| **Tokio** | Padrão da indústria, excelente docs | Mais complexo |
| async-std | Simples | Menos usado |
| smol | Pequeno | Menos recursos |

### Decisão: **Tokio 1.x**

### Justificativa

1. **Maturidade:** 10+ anos, amplamente testado
2. **Performance:** Melhor throughput que alternativas
3. **Ecosistema:** Todos os frameworks principais suportam
4. **Windows support:** Compatível com ambiente alvo

---

## 11. Decisão: Integração NVIDIA API

### Estratégia

1. **HTTP client:** reqwest com TLS
2. **Streaming:** Server-Sent Events via reqwest
3. **Model selection:** Dinâmico baseado em cargo/tarefa
4. **Error handling:** Circuit breaker pattern
5. **Rate limiting:** Token bucket

### Implementação Proposta

```rust
pub struct NvidiaNimClient {
    api_key: String,
    base_url: Url,
    client: ReqwestClient,
    rate_limiter: RateLimiter,
}

impl NvidiaNimClient {
    pub async fn chat_completion(
        &self,
        model: &str,
        messages: &[ChatMessage],
        stream: bool,
    ) -> Result<ChatResponse> {
        // Implementação OpenAI-compatible
    }
}
```

---

## 12. Roadmap de Decisões Futuras

| Decisão | Status | Quando decidir |
|---------|--------|----------------|
| Sistema de jobs/delegações | A definir | Fase 2 |
| Plano de backup automático | A definir | Fase 3 |
| Sistema de plugins | A definir | Fase 4 |
| Suporte a múltiplos provedores | Fora do escopo Fase 1 | Pós-fase 1 |

---

## Anexos

### 12.1 Dependências Principais (Cargo.toml extract)

```toml
# Core
tokio = { version = "1", features = ["full"] }
actix-web = { version = "4", features = ["rustls", "tokio"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "chrono", "json"] }

# Security
argon2 = "0.5"
jsonwebtoken = "9.3"
keyring = "3.6"

# NVIDIA API
reqwest = { version = "0.12", features = ["json", "rustls-tls", "timeout"] }

# MCP
rmcp = { version = "3.1", features = ["server"] }
```

### 12.2 Compatibilidade Windows

Todos os crates selecionados têm suporte comprovado ao Windows:

- ✅ tokio
- ✅ actix-web
- ✅ sqlx-sqlite
- ✅ reqwest
- ✅ rmcp

### 12.3 Tamanho Estimado do Binário

Com `opt-level = "z"` e `lto = true`:
- **Estimated binary size:** 15-25 MB
- **Memory footprint:** 50-100 MB em runtime normal

---

*Este documento será atualizado conforme novas decisões forem tomadas durante o desenvolvimento.*