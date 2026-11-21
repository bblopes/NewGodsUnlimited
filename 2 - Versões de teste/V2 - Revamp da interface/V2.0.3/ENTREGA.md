# V2.0.3 — Demografia corrigida e coleta configurável

**Data:** 21/11/2026 · **Versão maior:** V2 — Revamp da interface

## Pedido

- A Demografia não mostrava dado nenhum: corrigir.
- Ela deve mostrar informação o tempo todo, com diversidade genética, idade
  média e energia (histograma e gráfico).
- Coleta padrão a cada 500 ticks, com um controle para baixar até 250 ou subir.

## Entregue

- **Bug corrigido:** na primeira leitura a tela pedia a telemetria com o valor
  `-1` ("ainda sem amostras"), que o motor não aceita (só números sem sinal).
  O pedido era recusado e nenhuma amostra chegava. Agora o valor inicial é `0`,
  e um erro de leitura passa a aparecer no cabeçalho da Demografia em vez de
  ficar invisível.
- **Coleta da demografia:** seletor 250 · 500 · 1.000 · 2.000 · 5.000 ticks
  (padrão 500), na aba e na tela cheia.
- Cartão de células vivas lê o valor ao vivo.
- Gráficos: população, energia média (com o mínimo de reprodução), idade
  média, **diversidade genética**, **gerações** (máxima e média), natalidade,
  mortalidade e o histograma de energia atual.
- Testes: o motor simulado usado nos testes agora recusa tipos errados como o
  motor real, para esse tipo de erro não passar de novo.

## Como testar

Abra `New Gods Unlimited V2.0.3.exe`, vá em Demografia e espere duas coletas;
troque a coleta para 250 e veja as amostras chegarem mais rápido.

## Ficou de fora

Medir o custo da coleta na velocidade (entra no estudo de desempenho da V3).
