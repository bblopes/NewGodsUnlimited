# V5.0.1 — Gravador de sessão (telemetria da simulação)

**Data:** 22/11/2026 · **Versão maior:** V5 — Funções novas

## Pedido

- Poder ligar uma gravação, deixar a simulação rodando a noite toda e não
  perder nada se a janela fechar ou o computador desligar.
- Só a simulação: telemetria e as mudanças feitas pelo usuário que afetam o
  simulador. Sem comandos, mouse ou teclado por enquanto.
- Aproveitar o gravador que já tinha sido começado (conversa "Pendências").

## Entregue

- Botão **Gravar** na barra de cima, ao lado do Pausar. Durante a gravação
  aparecem o cronômetro, o botão **Parar** e a bandeira de **marcar momento**
  (atalho F8).
- Cada gravação vira uma pasta em `4 - Dados\Gravações\` (ou na pasta de
  dados do app, se o `.exe` estiver fora do projeto), com:
  - `telemetria.jsonl`: uma amostra da Demografia por linha;
  - `eventos_eco.jsonl`: o que mudou a simulação (parâmetros aplicados com
    antes/depois, reinício, mapa aplicado, célula inserida);
  - `mapas/`: o mapa no início e a cada "Aplicar mapa";
  - `marcas.jsonl`, `manifesto.json` e, ao parar, `RESUMO.md`.
- **Cada linha vai direto para o disco.** Se o app fechar ou o computador
  desligar, fica salvo tudo até a última amostra; só o `RESUMO.md` depende de
  apertar Parar.

**O que muda para você:** nada na simulação. É um registro ao lado dela.

## Medição

Sem medição de desempenho: o motor não mudou (só a interface). As gravações
escrevem uma linha por amostra da Demografia (a cada 500 ticks por padrão).

## Como testar

1. Clique em **Gravar › Ecossistema** e deixe rodar.
2. Aplique uma mudança em Biologia e marque um momento com F8.
3. Feche o app sem parar a gravação e abra a pasta `4 - Dados\Gravações\`:
   `telemetria.jsonl` e `eventos_eco.jsonl` estão lá até o último momento.

## Ficou de fora

Salvar e carregar a run (estado do mundo) para continuar depois; o modo
"ecossistema + sistema" (comandos, mouse e teclado) fica escondido.
