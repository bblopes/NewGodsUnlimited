# V5.0.3 — Abas Física e Mapa, modo dia, presets de mapa e runs salvas

**Data:** 22/11/2026 · **Versão maior:** V5 — Funções novas

## Pedido

- Preparar o terreno do sol: a aba Ambiente vira **Física** (Física do mapa:
  corrente e sol; Interações: anti-cramming) e a aba Mapa fica só com a
  **Construção** (paredes) e os **presets de mapa**.
- Modo noturno: o escuro atual vira o noturno e entra um tema claro, com
  botão ao lado do Pausar.
- Presets de mapa voltando a funcionar, como na V1, guardando paredes, sol e
  corrente.
- Salvar e carregar a run (a 5.0.4, feita junto).

## Entregue

- **Física**: Corrente, Sol (desenhar região, limpar, ver iluminação,
  aplicar) e Interações (anti-cramming).
- **Mapa › Construção**: só Selecionar, Parede e Borracha.
- **Mapa › Presets de mapa**: salvar o mapa aplicado com nome, carregar (já
  aplica paredes, sol e corrente) e apagar (dois cliques). Os arquivos
  `.ngumapa` ficam em `4 - Dados\Mapas\`; para importar ou exportar, é só
  copiar arquivos para dentro ou para fora dela. O "Oásis original" vem
  pronto.
- **Runs salvas** (Sessão e botão Salvar no topo): salvar com nome e carregar
  para continuar de onde parou, com as configurações e a telemetria. Os
  arquivos `.ngurun` ficam em `4 - Dados\Runs\` e são gravados num
  temporário antes de trocar, para um desligamento no meio não estragar a
  anterior.
- **Modo dia / noite**: botão de sol/lua ao lado do Pausar; a escolha fica
  salva.
- Correção: o tooltip da rede neural (V5.0.0) usava o mesmo elemento do
  tooltip da arena.

**O que muda para você:** o sol se desenha em Física; as runs da V5 usam o
formato novo `.ngurun` (as `.ecosrun` da V1 continuam abrindo só na V1).

## Medição

Sem mudança no motor da simulação; só a interface.

## Como testar

1. Troque para o modo dia no botão ao lado do Pausar e volte.
2. Em Física, desenhe uma região de sol e aplique; em Mapa, salve um preset,
   carregue o "Oásis original" e depois o seu.
3. Em Sessão › Runs salvas, salve, reinicie a run e carregue a salva.

## Ficou de fora

Paredes e sol desenhados na arena ainda usam as mesmas cores nos dois temas.
Mapas embutidos novos ficam para a V5.5.0.
