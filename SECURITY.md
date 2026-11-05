# Política e Estratégia de Segurança — NServo

A segurança é um dos pilares centrais do navegador **NServo**. Como um navegador que combina navegação na web com execução de scripts locais em Lua e ferramentas de linha de comando externas (como `pass`), um modelo de ameaças rigoroso é essencial.

---

## 1. Modelo de Confiança de Plugins

Plugins Lua possuem acesso total a APIs de sistema, leitura/gravação de disco e execução de subprocessos. Por esse motivo, o NServo adota um modelo de **Confiança Explícita (Zero Trust by Default)**:

1. **Detecção de Novo Plugin**:
   Ao iniciar ou ao executar um comando de plugin, o NServo verifica se o diretório do plugin (`plugins/<nome>/init.lua`) contém um hash registrado no arquivo `trusted_plugins.json`.

2. **Prompt de Segurança Bloqueante**:
   Se o plugin for desconhecido ou tiver seu conteúdo alterado, a execução é estritamente impedida e o usuário é confrontado com a mensagem:

   ```text
   Foi detectado um novo plugin: '<NOME>'.

   Plugins Lua possuem acesso ao sistema local e podem executar código arbitrário.

   Deseja confiar neste plugin?

   [y] Sim
   [n] Não
   ```

3. **Verificação Criptográfica de Integridade**:
   - O arquivo `trusted_plugins.json` armazena o caminho, timestamp de aprovação e o hash determinístico do script Lua.
   - Qualquer modificação no arquivo `init.lua` de um plugin altera seu hash, invalidando a confiança imediatamente e forçando uma nova revalidação pelo usuário.

4. **Revogação de Confiança**:
   A confiança pode ser revogada editando `trusted_plugins.json` ou via API interna `PluginTrustStore::revoke("<nome>")`.

---

## 2. Isolamento de Scripts Greasemonkey

- Scripts Greasemonkey operam exclusivamente no contexto de renderização do DOM.
- Por padrão, o escopo `grant: none` impede que scripts de terceiros acessem dados locais do sistema ou APIs sensíveis de controle de abas.
- O código de injeção é envelopado em IIFEs (Immediately Invoked Function Expressions) isoladas com tratamento de exceções global para evitar que falhas em scripts quebrem a execução do navegador.

---

## 3. Bloqueador de Anúncios e Proteção de Privacidade

- O motor de Adblock integrado processa listas baseadas na sintaxe ABP / EasyList (`filters/ads.txt`).
- As regras são aplicadas tanto a requisições de rede (scripts, iframes, imagens, XMLHttpRequests) quanto a filtros cosméticos (element hiding) antes de qualquer carregamento.
- Proteção contra trackers de terceiros via opção `$third-party`.

---

## 4. Persistência Segura em SQLite

- Os bancos de dados `history.db` e `bookmarks.db` utilizam SQLite local com transações atômicas e queries parametrizadas (`params![...]`), prevenindo qualquer vulnerabilidade de SQL Injection.
- O comando `:clean` permite ao usuário expurgar instantaneamente todo o histórico e favoritos sob confirmação explícita.

---

## 5. Práticas de Desenvolvimento em Rust

- **100% Safe Rust**: O código de negócio do NServo evita blocos `unsafe`.
- Tratamento de erros robusto com `Result<T>` e `thiserror::Error`, prevenindo panics inesperados em tempo de execução.
- Nenhuma dependência não auditada ou de repositórios não confiáveis.
