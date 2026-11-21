# Nova interface

**Estado:** estrutura de navegação aprovada (21/11/2026); próximo passo é o
acabamento visual.

- Especificação completa (documento colaborativo): <https://claude.ai/artifact/N6jV2iVzmqULwtnbQRvp7Q>
- Protótipo navegável: [`../Apresentação/prototipo-nova-interface.html`](../Apresentação/prototipo-nova-interface.html)

## Decisão

Trocar a interface desenhada à mão em Macroquad (`app.rs`, ~7.400 linhas) por
**Tauri 2 + React + TypeScript + Tailwind**, com a arena em **PixiJS**. O motor
Rust não muda: continua nativo na thread worker, e a interface só envia
comandos e recebe estado.

## Estrutura da tela

- **Barra superior:** run, tick, células, ticks/s, Iniciar/Pausar, velocidade,
  Aplicar/Reverter com contador de alterações pendentes, Salvar, troca entre
  Sessão e Laboratório.
- **Arena:** mundo, camadas, clique para selecionar célula, zoom, seguir,
  inserir célula e editar mapa.
- **Lateral direita com seis abas:** Sessão (a run), Ambiente (regras do
  mundo), Mapa (editor), Células (indivíduo e genética), Demografia (gráficos,
  expansível para tela cheia) e Camadas (só visualização).
- **Barra de status:** publicação, FPS, avisos e mensagens.
- **Laboratório** como segundo modo do mesmo app, com replay somente leitura.

## Etapas

1. Acabamento visual: 2 ou 3 variações de estilo em HTML.
2. Separar o motor num crate próprio (`ngu-core`), validado pelo hash do benchmark.
3. Esqueleto Tauri: ponte, comandos e arena com o mundo real.
4. Portar as abas, uma por vez.
5. Laboratório dentro do app.
6. Checklist de paridade e entrega do `.exe` (vira a `V2.0.0`).

O executável Macroquad atual continua funcionando até a paridade.

## Contrato front ↔ motor (resumo)

- **Comandos:** pausar, velocidade, headless, manter ativo, `apply_settings(diff)`,
  reiniciar, mapa, inserir célula, salvar/carregar run, presets, selecionar
  célula, relatório genético, telemetria, campanhas.
- **Eventos:** `metrics` (~4 Hz), `frame` (30–60 Hz, binário compacto),
  `cell_detail`, `telemetry_sample`, `run_event`, `status`.
- O front nunca recebe o `World` inteiro, só o que a aba aberta precisa.
