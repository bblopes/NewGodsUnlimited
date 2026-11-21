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
├── assets/presets/           presets embutidos no programa
├── scripts/                  compilação local (reserva do GitHub Actions)
└── target/                   saída do compilador (recriável, fora do Git)
```

## Rodar

```powershell
cargo run --release                                   # programa principal
cargo run --release --bin laboratorio_experimentos    # laboratório
cargo run --release --bin benchmark_simulacao -- --cenario realista
```

O benchmark lê as runs de `../4 - Dados/runs/` e grava relatórios em
`../4 - Dados/benchmarks/`.

## Compilar uma versão

O caminho padrão é o GitHub Actions: criar a tag `v1.2.0` (ou
`v1.2.0-teste.1`) compila no Windows e publica a Release. A reserva local é:

```powershell
scripts\COMPILAR_VERSAO.bat 1.2.0-teste.1
```

que gera `New Gods Unlimited V1.2.0-teste.1.exe` direto em
`2 - Versões de teste/` (ou em `1 - Versão oficial/`, se a versão não tiver
`-teste`). Detalhes em
[`VERSOES_E_DISTRIBUICAO.md`](../3%20-%20Documentos/Documentação/VERSOES_E_DISTRIBUICAO.md).
