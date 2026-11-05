# Estratégia de Testes Automatizados — NServo

O NServo adota uma pirâmide de testes rigorosa para garantir estabilidade, segurança e conformidade estrita com as diretrizes de Clean Code e limites de tamanho de código.

---

## 1. Categorias de Testes

### 1.1 Testes de Arquitetura e Limites de Código
- **Arquivo**: `tests/line_count_test.rs`
- **Objetivo**: Inspeciona recursivamente cada arquivo `.rs` dentro de `src/` e assegura que **nenhum arquivo exceda 200 linhas de código**.
- Garante conformidade contínua com a regra de manutenção humana do projeto.

### 1.2 Testes de Bloqueio de Anúncios (Adblock)
- **Arquivo**: `tests/adblock_test.rs`
- **Escopo**:
  - Parsing de regras ABP de rede: `||domain^$third-party`
  - Parsing de exceções: `@@||domain/whitelist^`
  - Parsing de filtros cosméticos: `domain##.ad-class`
  - Casos de borda: matching de terceiro, combinação de regras e precedência de exceções.

### 1.3 Testes de Persistência SQLite
- **Arquivos**: `tests/history_test.rs`, `tests/bookmarks_test.rs`
- **Escopo**:
  - Criação de esquema e índices em bancos SQLite em memória.
  - Incremento de contador de visitas e timestamps.
  - Ranking de autocomplete ponderado por recência e frequência.
  - Busca de favoritos por URL, título e tags.
  - Expurgos atômicos (`clear()`).

### 1.4 Testes de Comandos e Autocompletion
- **Arquivo**: `tests/command_test.rs`
- **Escopo**:
  - Parser de comandos Vim-like (`:open`, `:Open`, `:tabnew`, `:wq`, `:clean`, `:bookmark`, etc.).
  - Motor de autocompletion de comandos e histórico.

### 1.5 Testes de Greasemonkey
- **Arquivo**: `tests/greasemonkey_test.rs`
- **Escopo**:
  - Extração de blocos `// ==UserScript==`.
  - Parsing de tags `@name`, `@match`, `@run-at`.
  - Algoritmo de matching de URLs com curingas (`*://domain/*`).
  - Geração de código de injeção JavaScript seguro.

### 1.6 Testes da Máquina de Estados de Modos
- **Arquivo**: `tests/modes_test.rs`
- **Escopo**:
  - Transições entre Normal, Insert, Passthrough e Command.
  - Tratamento de teclas de escape (`ESC`, `i`, `:`, `ctrl+v`).

### 1.7 Testes de Sessão
- **Arquivo**: `tests/session_test.rs`
- **Escopo**:
  - Serialização e desserialização de abas, URLs ativas e coordenadas de scroll.
  - Restauração idêntica de sessões ao reiniciar.

### 1.8 Testes do Runtime Lua
- **Arquivo**: `tests/lua_test.rs`
- **Escopo**:
  - Inicialização do runtime Lua (mlua).
  - Funções de configuração globais (`set`, `bind`, `nservo.notify`, `nservo.tabs.new`).

### 1.9 Testes de Segurança de Plugins
- **Arquivo**: `tests/security_test.rs`
- **Escopo**:
  - Armazenamento de registros de confiança em JSON.
  - Detecção imediata de adulteração de código por hash inválido.
  - Concessão e revogação de permissões.

---

## 2. Executando os Testes

Para executar toda a suíte de testes:

```bash
cargo test
```

Para executar um teste específico:

```bash
cargo test --test adblock_test
cargo test --test line_count_test
```

Para verificar warnings e lints:

```bash
cargo check
cargo clippy --all-targets -- -D warnings
```
