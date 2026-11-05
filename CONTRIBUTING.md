# Guia de Contribuição — NServo

Obrigado por seu interesse em contribuir com o **NServo**! Nosso objetivo é construir um navegador moderno, minimalista, centrado no teclado, escrito em Rust e extensível em Lua, projetado para ser compreendido e mantido por seres humanos sem dependência de ferramentas de IA.

---

## 1. Princípios Inegociáveis

Todo código enviado ao repositório do NServo deve respeitar estritamente os seguintes princípios:

1. **Clean Code & SOLID**:
   - **S**ingle Responsibility: cada módulo, struct e função faz apenas uma coisa bem feita.
   - **O**pen/Closed: novas capacidades devem ser adicionadas por extensão (ex: traits de `BrowserEngine`, plugins Lua, regras de adblock), sem modificar o núcleo desnecessariamente.
   - **L**iskov Substitution: implementações da trait `BrowserEngine` (como `ServoEngine` e `MockEngine`) devem ser intercambiáveis.
   - **I**nterface Segregation: traits enxutas e focadas.
   - **D**ependency Inversion: desacoplamento através de traits e injeção de dependências.

2. **Composition over Inheritance**:
   - Estruturas de dados compostas, sem hierarquias artificiais.

3. **KISS & DRY (Sem exageros)**:
   - Clareza tem prioridade sobre "engenhosidade".
   - Evite macros obscuras ou abstrações precoces.

---

## 2. Regra dos 200 Linhas por Arquivo (Hard Limit)

> **ATENÇÃO:** Nenhum arquivo fonte `.rs` pode ultrapassar **200 linhas de código**.

Essa regra existe para garantir:
- Fácil navegação em editores de texto como Neovim e Vim.
- Manutenção manual descomplicada.
- Revisão de Pull Requests rápida e confiável.
- Foco absoluto na responsabilidade única.

Se um arquivo estiver crescendo próximo a 180 linhas, refatore-o em um submódulo:
- Exemplo: em vez de um `app.rs` gigante, divida em `src/app/state.rs`, `src/app/event_handler.rs`, etc.
- O teste automatizado `tests/line_count_test.rs` falhará o build se algum arquivo violar essa regra.

---

## 3. Estrutura de Módulos

Novas funcionalidades devem ser adicionadas no diretório apropriado:

```text
src/
├── browser/        # Backend do motor Servo / WebRender / wgpu e traits
├── command/        # Parser, autocompletion e executores de comandos (:open, etc.)
├── config/         # Inicialização de diretórios, options e keybindings
├── plugins/        # Descoberta, carregamento e plugins padrão (pass, readability)
├── greasemonkey/   # Parse de metadados, match de URLs e injeção de scripts
├── adblock/        # Parser ABP/EasyList, filtros cosméticos e bloqueio de rede
├── ui/             # Status bar, command line, dialogs e renderizador TUI
├── tabs/           # Gerenciamento de abas e ciclo de vida
├── history/        # Persistência em SQLite do histórico e ranking de frequência
├── bookmarks/      # Persistência em SQLite de favoritos e tags
├── modes/          # Máquina de estados dos modos (Normal, Insert, Passthrough, Command)
├── session/        # Serialização e restauração de sessões (:wq, session.json)
├── security/       # Assinatura de plugins e prompt de confiança
├── lua/            # Runtime Lua (mlua), bindings nservo.* e helpers globais
└── platform/       # Abstrações de sistema (Linux, Windows, macOS, Android)
```

---

## 4. Fluxo de Trabalho e Testes

Antes de submeter um Pull Request:

1. **Formatação e Lints**:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   ```

2. **Executar a Suíte de Testes**:
   ```bash
   cargo test
   ```
   Certifique-se de que todos os testes passem, incluindo `test_no_file_exceeds_200_lines`.

3. **Mensagens de Commit**:
   - Siga convenções semânticas claras:
     - `feat: adiciona suporte a filtros cosméticos por domínio`
     - `fix: corrige matching de wildcard em scripts greasemonkey`
     - `refactor: divide módulo de renderização para respeitar limite de 200 linhas`
     - `docs: atualiza guia de compilação para Android`
