# V2.0.2 — Aba Biologia e código genético explicado

**Data:** 21/11/2026 · **Versão maior:** V2 — Revamp da interface

## Pedido

- Criar a aba **Biologia** (entre Sessão e Ambiente) com células e
  fitoplâncton; **Ambiente** fica só com corrente e densidade.
- Anotar como pendência uma futura aba de **Física** (sol, intensidade do sol).
- No código genético da célula, mostrar todas as categorias de informação e os
  bits de cada trecho; deixar claro com qual substância (C0–C7) P1, P2 e P3
  trabalham, inclusive passando o mouse na rede neural.

## Entregue

- Aba **Biologia**: Células (mínimo e limite de reprodução, custo mental) e
  Fitoplâncton. **Ambiente**: Corrente e Densidade. Cada aba mostra só as suas
  alterações pendentes.
- **Código genético** (Células › Indivíduo e painel do indivíduo salvo): grade
  de bits colorida por trecho e um cartão para cada trecho, com posição, bits,
  valor decodificado e explicação:
  - fiação neural (bits 0–1215, 64 registros), com a lista dos 64 registros
    (origem → destino, peso, se entra no cérebro);
  - canais químicos de P1/P2/P3 (bits 1207–1215);
  - alelo do alcance visual (1216–1220);
  - atividade dos 30 nós (1221–1250);
  - reserva (1251–1255) e longevidade (alelo fora do genoma neural).
- **Rede neural:** P1, P2, P3 e E1, E2, E3 aparecem como `P1·C5` e o tooltip
  diz "trabalha com a substância C5". O cabeçalho da rede mostra o mapa
  `P1=C? · P2=C? · P3=C?` do indivíduo.

## Respostas às perguntas

- **PG:** toda célula emite PG basal o tempo todo; o motor `EG` só reforça.
  Por isso aparece PG em regiões onde nenhuma rede tem `EG` ligado.
- **P1, P2, P3 e C0–C7:** é exatamente isso. P1/P2/P3 são três "grupos" fixos
  da rede (sensor de intensidade, gradientes X/Y e o motor de emissão E1/E2/E3).
  Nove bits do genoma escolhem, para cada grupo, uma das oito substâncias do
  ambiente (C0–C7). Se dois grupos sortearem a mesma, o motor avança para a
  próxima livre, então os três são sempre diferentes. Isso varia de indivíduo
  para indivíduo e é herdado com mutação.
- **Achado:** esses 9 bits ficam dentro do último registro de fiação (#63).
  Uma mutação ali pode mudar ao mesmo tempo uma ligação e um canal químico.
  Fica registrado em Backlog › Pendências para a gente decidir se separa.

## Como testar

1. Abra `New Gods Unlimited V2.0.2.exe`; confira as abas Biologia e Ambiente.
2. Clique numa célula, abra **Código genético** e **Ver os 64 registros**.
3. Na rede neural, passe o mouse em P1 e veja a substância.

## Ficou de fora

A aba de Física (sol) — anotada como pendência para avaliar.
