# Estrutura do repositório

Este documento descreve a função de cada pasta no nível principal do projeto
**New Gods Unlimited**. A regra central é simples: fonte, documentação e
referências aprovadas entram no Git; saídas de compilação, dados de uso e
distribuições ficam locais e são recriáveis ou arquiváveis.

| Pasta | Para que serve | Git |
|---|---|---|
| `.cargo/` | Configuração específica do Cargo/Rust usada por este projeto. | Versionada |
| `.git/` | Banco de dados local do Git: commits, branches e configuração do repositório. Não editar manualmente. | Gerenciada pelo Git |
| `archive/` | Material histórico. `archive/releases/` preserva executáveis e pacotes antigos antes de substituições. | Só o guia é versionado; binários são ignorados |
| `assets/` | Recursos de referência usados pelo programa. Hoje contém presets distribuídos junto com o código. | Versionada |
| `data/` | Dados produzidos durante uso e experimentos: sessões, campanhas e presets pessoais. | Ignorada, exceto o guia |
| `docs/` | Arquitetura, regras biológicas, especificações e propostas do projeto. | Versionada |
| `releases/` | Entrega portátil atual para usuários. A compilação publica em `releases/windows/current/`. | Ignorada |
| `scripts/` | Automação de tarefas locais. `COMPILAR_PORTATIL.bat` e `.ps1` geram a entrega Windows. | Versionada |
| `src/` | Código-fonte Rust do aplicativo. | Versionada |
| `target/` | Saída padrão do Cargo para testes e desenvolvimento. Pode ser apagada e será recriada. | Ignorada |
| `target_publicacao/` | Saída de compilação dedicada à publicação portátil, para não misturar com builds normais. Pode ser recriada. | Ignorada |

## Dentro de `src/`

| Caminho | Responsabilidade |
|---|---|
| `main.rs` | Ponto de entrada e configuração da janela. |
| `lib.rs` | Expõe os módulos reutilizados pelos binários auxiliares. |
| `app.rs` | Interface principal, controles de sessão, desenho e persistência. |
| `config.rs` | Constantes e parâmetros globais do mundo. |
| `lab.rs` | Laboratório de experiências, campanhas e análises. |
| `bin/` | Executáveis auxiliares. Atualmente contém o laboratório de experimentos. |
| `environment/` | Mapa e correntes ambientais. |
| `simulation/` | Regras de simulação: mundo, cérebro, genoma e feromônios. |

## Arquivos principais da raiz

| Arquivo | Para que serve |
|---|---|
| `Cargo.toml` | Identidade do pacote Rust e dependências. |
| `Cargo.lock` | Versões exatas das dependências, garantindo builds repetíveis. |
| `README.md` | Porta de entrada do projeto e instruções de uso. |
| `.gitignore` | Define o que não entra em commits, como builds, dados e releases locais. |

## Onde colocar coisas novas

- Código que executa no produto: `src/`.
- Testes novos: `tests/` quando forem testes de integração, ou ao lado do módulo quando forem unitários.
- Decisão, explicação ou especificação: `docs/`.
- Preset aprovado como referência: `assets/presets/`.
- Resultado de um experimento: `data/runs/` ou `data/campaigns/`.
- Entrega compilada para uso: gerada pelo script em `releases/windows/current/`.
