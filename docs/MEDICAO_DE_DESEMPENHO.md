# Medição de desempenho

Este documento define uma linha de base antes de qualquer otimização. O
objetivo é saber **onde** o tempo é gasto e garantir que uma mudança mais
rápida não altere o estado biológico da simulação.

## Ferramenta

O binário headless `benchmark_simulacao` cria uma população inicial usando:

- os genomas da população final da run salva mais avançada em `data/runs/`;
- a configuração neural `ValidatedV2` sobre esses fundadores;
- seed fixa, mapa padrão e corrente Clássica em intensidade 1.

Assim, a carga não mede uma população aleatória: nesta primeira linha de
base, ela usou 87 genomas da run `run-4643592898594514058.ecosrun` (tick
9.780.201), com a fiação Val 2 garantida para navegação e porta de doação.
O benchmark entende os formatos de run V1 a V15 para poder reutilizar a
história já existente do projeto.

Há duas passagens independentes por repetição:

1. `World::tick()` mede ticks por segundo sem relógios internos.
2. `World::tick_profiled()` mede as 13 etapas do mesmo tick.

Ao fim, o estado completo é serializado e resumido com um hash. Os hashes das
duas passagens precisam coincidir. Isso verifica que a instrumentação não
alterou a ordem nem as regras do mundo.

## Executar

No diretório raiz:

```powershell
cargo run --release --bin benchmark_simulacao -- --cenario realista
```

Opções úteis:

```text
--cenario base|realista|estresse
--populacao N --ticks N --aquecimento N --repeticoes N --seed N
--run-validado CAMINHO
--sem-arquivo
```

O relatório bruto em JSON é criado em `data/benchmarks/`. Ele é deliberadamente
ignorado pelo Git: contém a medição específica da máquina. A linha de base
interpretada fica neste documento e pode ser atualizada quando houver uma
mudança intencional de hardware ou de cenário.

## Modo headless no executável

Para comparar a influência da visualização durante uma sessão real, abra a aba
**Sessão** e clique em **Modo headless · medir teto de cálculo**. O modal
explica a troca e, ao confirmar, mantém somente um painel mínimo com tick,
população e ticks/s. Nesse modo:

- a arena, organismos, alimento, mapa e painéis detalhados não são desenhados;
- o worker não clona nem publica `World` para a interface;
- a simulação ignora o limitador de velocidade da interface e avança no máximo
  que o cálculo permitir;
- **Voltar à visualização** (ou `Esc`) restaura a tela e publica uma fotografia
  atual do mundo imediatamente.

Para uma comparação justa, observe primeiro a taxa visual com a velocidade da
sessão em `1.000.000×`; nessa configuração a interface também deixa de impor
um teto artificial. Depois ative o modo headless e compare as duas leituras.
O modo não altera regras biológicas, seed ou parâmetros da run.

## Cenários

| Cenário | Intenção | Configuração principal |
|---|---|---|
| `base` | custo estrutural, sem comida nem reprodução | 100 células, emissões e ganho alimentar zerados |
| `realista` | sessão equivalente aos valores usuais da interface | 100 células, emissão 0,12/0,24, alimento 50, limite 5.000 |
| `estresse` | pressão de população e colisões | 300 células, emissão 0,48/0,96, anti-cramming ligado |

`base` não é uma experiência biológica: as células morrem sem alimento. Ele
serve somente como contraste de custo computacional.

## Linha de base — 17 de outubro de 2026

Máquina: Windows 11 Pro 64 bits, AMD Ryzen 7 5800XT (8 núcleos / 16 lógicos).
Build: `cargo run --release`. Todas as medições abaixo usaram seed
`4634958140201280650`, três repetições e verificaram hashes iguais entre o
tick normal e o perfilado.

| Cenário | Carga medida | Mediana | População final | Principal custo |
|---|---:|---:|---:|---|
| base | 100 células, 1.000 ticks, 200 aquecimento | 1.044,6 ticks/s | 0 | feromônios 68,7%; sensores 26,6% |
| realista | 100 células, 1.000 ticks, 200 aquecimento | 805,5 ticks/s | 48 | feromônios 65,5%; sensores 26,9% |
| estresse | 300 células, 500 ticks, 100 aquecimento | 201,5 ticks/s | 300 | feromônios 60,6%; sensores 24,9%; interações 10,4% |

Os percentuais vêm da passagem perfilada e, portanto, indicam distribuição de
custo; a taxa de ticks/s da tabela vem da passagem sem instrumentação. No
cenário realista, as médias por tick foram: feromônios 808,7 µs, sensores
332,5 µs, alimentação 32,3 µs e interações 27,9 µs. No estresse, esses dois
primeiros custos passaram para 3.043,1 µs e 1.249,8 µs, respectivamente.

## Leitura e próximo passo

O primeiro alvo técnico é `PheromoneField::rebuild()`: ele reconstrói o campo
químico inteiro e deposita a contribuição de cada organismo. O segundo é
`World::update_sensors()`, que consulta alimento, organismos, linha de visão e
o campo químico. Antes de alterar qualquer um deles, devemos criar uma medição
mais fina dentro dessas duas funções e comparar a mesma tabela acima após cada
mudança.
