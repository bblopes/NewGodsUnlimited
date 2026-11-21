# Avaliação de desempenho — abertura da V3

**Data:** 21/11/2026 · **Base:** código da V2.0.3 · **Meta:** 200 células a
1.000 ticks/s, ou seja, cerca de **1 ms por tick**.

Este documento responde a três perguntas: onde o tempo do tick é gasto hoje,
quão otimizado está cada processo, e se a ideia de dividir o mapa em
quadrantes e rodar na GPU é o caminho certo.

## 1. Como foi medido

- Binário `benchmark_simulacao` (ver `MEDICAO_DE_DESEMPENHO.md`), agora com
  perfil mais fino: feromônios separados em **células** e **PF**, e sensores
  separados em **grades e contato**, **visão** e **químicos**.
- Novo cenário `sessao`, com os padrões da interface V2: emissão 0,12,
  limite 5.000 e anti-cramming ligado. Ele usa 15.000 ticks de aquecimento
  para o fitoplâncton crescer até o regime da sessão.
- Roda no GitHub pelo workflow **Medir desempenho** (Windows, 4 vCPU), com a
  população de referência versionada em `5 - Código-fonte/dados-de-teste/`.
- Em todas as execuções, o tick normal e o perfilado terminaram com o mesmo
  hash de estado. A instrumentação não alterou a simulação.

## 2. Resultado: o gargalo não são as células

| Caso | Células | Fitoplâncton | Ticks/s | Maior custo |
|---|---:|---:|---:|---|
| realista (curto) | 100 → 3 | 188 | 1.349 | feromônios das células 35%, PF 29%, químicos 22% |
| estresse | 300 → 58 | 268 | 243 | feromônios das células 43%, químicos 26%, interações 11% |
| **sessão** | 100 → 0 | **5.419** | **112** | **PF 88% (7,8 ms/tick)**, movimento do alimento 7,5% |
| **sessão** | 200 → 0 | **4.993** | **115** | **PF 88% (7,7 ms/tick)**, movimento do alimento 7,1% |

As duas linhas de sessão terminaram **sem nenhuma célula viva** e, mesmo
assim, rodaram a 112–115 ticks/s. É exatamente a velocidade que aparece na
interface com cerca de 100 células. A lentidão atual vem do fitoplâncton: com
cerca de 5.000 partículas dentro do oásis, espalhar o feromônio de alimento
(PF) custa 7,8 ms por tick, quase todo o orçamento.

Por que custa tanto: o fitoplâncton fica dentro do oásis, que é cercado por
parede. Para cada uma das até 9.216 células da malha química com alimento, o
código:

1. procura parede num raio de 500 unidades, varrendo cerca de 650 pixels do
   mapa;
2. como quase sempre há parede, traça uma linha de visão para cada uma das
   ~121 amostras do raio, com até 25 passos cada.

Esse trabalho é refeito do zero a cada tick, embora a geometria (mapa e
malha) não mude entre ticks.

## 3. Avaliação de cada processo

Números de µs/tick do cenário de sessão (cerca de 5.000 fitoplânctons) e do
estresse (300 células), máquina do GitHub.

| # | Etapa | Custo medido | Estado | O que fazer |
|---|---|---|---|---|
| 1 | Tempo | ~0 | ótimo | nada |
| 2 | Emissão de alimento | ~1 µs | ótimo | nada |
| 3a | Feromônios · células | 17 µs (sessão), 1.765 µs (300 cél.) | **ruim** | cachear assinatura PG e canais por organismo (hoje: 2 × 1.256 hashes por célula, duas vezes por tick); pré-calcular "há parede no raio" por célula da malha |
| 3b | Feromônios · PF | **7.817 µs** | **muito ruim** | pré-calcular, por célula da malha, a lista (destino, peso) já filtrada por paredes; refazer só quando o mapa mudar. Depósito vira ~150 mil multiplicações-somas |
| 4a | Sensores · grades e contato | 195 µs | médio | trocar `HashMap` reconstruído a cada tick por grade plana (21 × 12 células de 600 u); não indexar alimento quando não há células |
| 4b | Sensores · visão | 89 µs (300 cél.) | bom | pular a linha de visão quando não há parede no raio (mesmo cache de 3a); paralelizável |
| 4c | Sensores · químicos | 1.067 µs (300 cél.) | **ruim** | quase todo o custo é recalcular a assinatura PG; com cache cai para poucos µs |
| 5 | Cérebro | 36 µs (300 cél.) | ótimo | nada agora; paralelizável trivialmente |
| 6 | Movimento do alimento | 668 µs | médio | corrente e paredes por partícula. Paralelizar (cada partícula é independente) e evitar o `map.clone()` por tick em `reproduce_sunlit_phytoplankton` |
| 7 | Movimento das células | 20 µs | ótimo | nada |
| 8 | Interações | 457 µs (300 cél.) | médio | até 6 passadas de anti-cramming, cada uma reconstruindo `HashMap`; grade plana reaproveitada |
| 9 | Energia | 11 µs | ótimo | nada |
| 10 | Mortes | ~0 | ótimo | nada |
| 11 | Alimentação | 147 µs | médio | grade plana de células em vez de `HashMap`; paralelizável (cada partícula escolhe a vencedora só lendo) |
| 12 | Limpeza de alimento | 68 µs | bom | nada agora |
| 13 | Reprodução | ~0 | ótimo | nada |

Em resumo, três processos são ruins e explicam quase tudo: PF, a assinatura
PG recalculada e as estruturas `HashMap` refeitas a cada tick. O resto está
bem implementado ou é barato demais para importar.

## 4. Projeção

Com os itens 3a, 3b e 4c (todos exatos, sem mudar o resultado bit a bit):

- **Sessão com ~5.000 fitoplânctons:** de ~8,9 ms para cerca de 1,2 ms por
  tick (~800 ticks/s), dominado por movimento e grades do alimento.
- **Com 200 células vivas:** some-se a parte das células (depósito de PG,
  visão, contato, interações), estimada em 0,6–1,0 ms. Total de 1,8–2,2 ms,
  ou ~450–550 ticks/s em uma thread.
- **Para chegar a 1 ms:** paralelizar as passadas que só leem o retrato
  (movimento do alimento, visão, química, cérebro, alimentação) nos 8 núcleos
  do Ryzen. É aí que paralelismo entra, depois do conserto algorítmico.

Isso é estimativa; cada passo será medido no workflow antes do próximo.

## 5. Posição sobre quadrantes e GPU

**GPU: não agora, e provavelmente não para o tick inteiro.**

- O gargalo atual é trabalho repetido, não falta de poder de cálculo. Um
  cache resolve 88% do tempo da sessão; a GPU só faria o mesmo desperdício
  mais rápido.
- A 1.000 ticks/s o orçamento é 1 ms. Cada ida e volta CPU↔GPU (enviar
  posições, ler motores e campos de volta) custa de 0,1 a 0,5 ms por tick
  antes de calcular qualquer coisa.
- Os cérebros são pequenos e com muitos desvios (fiação diferente por
  célula). Esse é o pior caso para GPU, e com 200 células não há volume para
  ocupá-la.
- Somas em paralelo na GPU (atomics) mudam a ordem das somas de ponto
  flutuante. O resultado deixa de ser idêntico entre execuções, e isso quebra
  a garantia de sessão reproduzível (mesma seed → mesmo hash).
- Seria reescrever o motor em shaders, um custo grande para a fase atual.
  Se um dia houver milhares de células ou campos químicos com difusão e
  memória, a GPU volta a fazer sentido para os campos (malhas), não para os
  cérebros.

**Quadrantes: a intuição de paralelizar está certa; dividir o espaço em
quadrantes não é a melhor forma aqui.**

- Quase todas as etapas pesadas só **leem** um retrato do início do tick e
  escrevem no próprio organismo ou partícula: química, visão, cérebro,
  movimento do alimento, escolha da vencedora do alimento. Elas paralelizam
  **por entidade**, sem nenhuma borda: cada thread pega um pedaço da lista e
  o resultado é idêntico ao sequencial. Em Rust isso é o `rayon`.
- Quadrantes fixos sofrem com desbalanceamento. A vida se concentra no
  oásis e em cardumes, então um quadrante teria quase todo o trabalho e os
  outros ficariam ociosos.
- O problema das bordas existe de verdade, mas só nas etapas em que duas
  células escrevem uma na outra: anti-cramming, fagocitose, doação e a
  disputa pela mesma comida. Hoje essas etapas pesam 1–11% do tick. Para
  elas há duas soluções conhecidas:
  - **Duas fases:** calcular em paralelo as propostas (empurrões, capturas,
    doações) e aplicá-las em sequência numa ordem fixa. Simples e
    determinístico.
  - **Tabuleiro (xadrez):** dividir em ladrilhos com margem maior que o
    maior alcance de interação. A visão chega a 600 unidades e o PG a 500,
    então a margem precisa ser de pelo menos 600. Processar primeiro os
    ladrilhos "brancos" e depois os "pretos", de modo que dois ladrilhos
    vizinhos nunca rodem ao mesmo tempo. É a forma correta de fazer
    quadrantes, mas só compensa com milhares de células.
- Recomendação: rayon por entidade nas etapas de leitura e duas fases nas de
  escrita. Quadrantes em tabuleiro ficam guardados para quando a população
  justificar.

## 6. Roteiro da V3

Cada passo é uma entrega própria (V3.0.x). Cada uma é medida no workflow, e
exige hash igual ao do código anterior quando a mudança não altera regras.

1. **V3.0.0 — PF pré-calculado.** Tabela de depósito por célula da malha,
   refeita só quando o mapa muda. Teste comparando o campo com a
   implementação atual. Ganho esperado: ~7× na sessão.
2. **V3.0.1 — Assinatura e canais em cache** por organismo, mais o cache
   "há parede no raio" por célula da malha (serve a feromônios e visão).
3. **V3.0.2 — Grades planas** no lugar dos `HashMap` reconstruídos (visão,
   contato, alimentação, anti-cramming e densidade do fitoplâncton), e fim
   do `map.clone()` por tick.
4. **V3.0.3 — Paralelismo com rayon** nas etapas de leitura, com hash igual
   ao sequencial. A interface ganha a opção de número de threads.
5. **V3.0.4 — Duas fases nas interações**, se a medição com 200+ células
   mostrar que elas viraram gargalo.

Medições ainda por fazer:

- Um cenário que mantenha 200 células vivas no regime da sessão. Os
  fundadores de referência morrem no cenário `sessao`: falta um modo que
  reponha a população ou use fundadores adaptados ao oásis.
- O custo da própria interface: publicar o frame e, a cada 500 ticks, a
  telemetria. O modo headless já permite comparar.

## 7. Acompanhamento

| Entrega | Sessão (~5.400 fito) | Estresse (300 cél.) | Estado igual ao anterior |
|---|---:|---:|---|
| V2.0.3 (linha de base) | 111,8 ticks/s | 243 ticks/s | — |
| V3.0.0 — PF pré-calculado | 793,1 ticks/s | 265 ticks/s | sim (4 cenários) |
| V3.0.1 — assinatura em cache + distância às paredes | 1.412 ticks/s* | 1.137 ticks/s | sim (4 cenários) |

\* Parte do ganho na sessão é variação entre máquinas do GitHub: o movimento do fitoplâncton, que não mudou, saiu ~40% mais rápido.

Desde a V3.0.2 a medição compara a versão anterior e a nova na mesma
máquina (as máquinas do GitHub variam até 1,7× entre execuções):

| Entrega | Sessão | Estresse | Realista | Estado igual |
|---|---:|---:|---:|---|
| V3.0.2 × V3.0.1 — grades leves e fitoplâncton | 2,08× | 1,44× | 1,61× | sim |
