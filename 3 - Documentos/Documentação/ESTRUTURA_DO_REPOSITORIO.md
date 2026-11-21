# Estrutura do repositório

A raiz é feita para quem abre a pasta pela primeira vez: versão para rodar,
documentos para ler e, por último, o código. Os números fixam essa ordem no
Explorer do Windows.

| Pasta | Finalidade | No GitHub? |
| --- | --- | --- |
| `LEIA-ME.md` | porta de entrada | sim |
| `1 - Versão oficial/` | última versão aprovada (`.exe`), presets e runs salvas pelo programa | só o `LEIA-ME.md` |
| `2 - Versões de teste/` | builds em validação | só o `LEIA-ME.md` |
| `2 - Versões de teste/_histórico/` | versões substituídas | não |
| `3 - Documentos/` | apresentação, documentação, backlog, propostas e relatórios | sim |
| `4 - Dados/` | runs, campanhas, presets e benchmarks de desenvolvimento | só o `LEIA-ME.md` |
| `5 - Código-fonte/` | projeto Rust completo | sim, exceto `target/` |
| `.github/` | workflow de compilação e README da página do GitHub | sim |

Os `.exe` não entram nos commits: ficam nas Releases do GitHub.

## Onde colocar coisas novas

- Código que roda no produto: `5 - Código-fonte/src/`.
- Testes de integração: `5 - Código-fonte/tests/`; unitários, ao lado do módulo.
- Regra já implementada: `3 - Documentos/Documentação/`.
- Ideia especificada, ainda não feita: `3 - Documentos/Propostas/`.
- Item do que vem a seguir: `3 - Documentos/Backlog/`.
- Medição ou análise: `3 - Documentos/Relatórios/`.
- Material para mostrar a outras pessoas: `3 - Documentos/Apresentação/`.
- Preset de referência embutido no programa: `5 - Código-fonte/assets/presets/`.
- Resultado de experimento: `4 - Dados/runs/` ou `4 - Dados/campanhas/`.
- Versão compilada: gerada pelo GitHub Actions (ver [Versões e distribuição](VERSOES_E_DISTRIBUICAO.md)).

## Por que existe cada coisa no código

- `.cargo/config.toml` — liga `crt-static`, para o `.exe` não exigir o Visual C++ Redistributable.
- `Cargo.lock` — trava as versões exatas das dependências; builds repetíveis.
- `target/` — cache do compilador; pode ser apagado a qualquer momento.
