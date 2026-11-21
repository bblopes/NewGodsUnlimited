# Código-fonte

Todo o projeto Rust fica nesta pasta. Os comandos abaixo rodam **daqui de
dentro** (`5 - Código-fonte/`).

```text
5 - Código-fonte/
├── Cargo.toml / Cargo.lock   projeto e dependências (macroquad)
├── .cargo/config.toml        faz o .exe não depender do Visual C++ Redistributable
├── src/
│   ├── main.rs, app.rs       interface atual (Macroquad)
│   ├── lab.rs                Laboratório de experimentos
│   ├── simulation/           World, genoma, cérebro, feromônios
│   ├── environment/          corrente e mapa
│   └── bin/                  benchmark_simulacao, laboratorio_experimentos
├── interface/                V2 — Revamp da interface (Tauri): src/engine.rs + ui/
├── assets/presets/           presets embutidos no programa
├── scripts/                  compilação local (reserva do GitHub Actions)
└── target/                   saída do compilador (recriável, fora do Git)
```

## Rodar

```powershell
cargo run --release                                   # programa V1 (Macroquad)
cd interface; cargo run --release; cd ..              # interface nova (V2)
cargo run --release --bin laboratorio_experimentos    # laboratório
cargo run --release --bin benchmark_simulacao -- --cenario realista
```

O benchmark lê as runs de `../4 - Dados/runs/` e grava relatórios em
`../4 - Dados/benchmarks/`.

## Interface nova

`interface/` é um projeto Rust separado que usa este mesmo motor (a biblioteca
`new_gods_unlimited`). `src/engine.rs` roda a simulação numa thread e expõe
comandos para a tela; `ui/` tem o HTML, o CSS e o JavaScript da interface,
embutidos no `.exe`. A versão dela fica em `interface/Cargo.toml` e as notas
em `interface/NOTAS_DA_VERSAO.md`.

## Compilar uma versão

O caminho padrão é o GitHub Actions: mudar o campo `version` do `Cargo.toml`
(por exemplo `1.2.0`) e levar a mudança para a `main` compila no
Windows e publica a Release. A reserva local é:

```powershell
scripts\COMPILAR_VERSAO.bat 1.2.0
```

que gera `New Gods Unlimited V1.2.0.exe` direto em
`2 - Versões de teste/V1 - Interface original/`. Detalhes em
[`VERSOES_E_DISTRIBUICAO.md`](../3%20-%20Documentos/Documentação/VERSOES_E_DISTRIBUICAO.md).
