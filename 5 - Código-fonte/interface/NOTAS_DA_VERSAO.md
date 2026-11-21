## New Gods Unlimited V2.0.0 — primeira versão da interface nova

Interface nova (Tauri) sobre o **mesmo motor de simulação** da V1.1.0: nenhuma regra biológica mudou.

### O que já funciona

- Barra superior com run, tick, células, ticks/s, Iniciar/Pausar, velocidade de 1× a 1.000.000×, Aplicar/Reverter com contador de alterações pendentes e Reiniciar com confirmação.
- Arena com zoom (roda do mouse), arrastar, encaixar, clicar numa célula para abri-la, hover com dados e seguir a célula.
- **Sessão:** condições iniciais (seed, células, configuração neural Clássica/Motor/PF guia, Amadeus), modo headless, manter o Windows ativo, inserir célula no mapa.
- **Ambiente:** reprodução, custo mental, fitoplâncton, corrente e anti-cramming, aplicados no tick exato.
- **Mapa:** parede, borracha, região ensolarada por contorno, limpar sol, desfazer, aplicar e descartar.
- **Células:** indivíduo com energia, idade, filhos, corpo, sensores, rede neural ao vivo e código genético; genética com grupos, circuitos e uso de neurônios.
- **Demografia:** janelas 5k/30k/100k/25%/Tudo, população, energia média com o mínimo de reprodução em vigor, idade, natalidade, mortalidade e histograma; marcos de cada Aplicar nos gráficos; expansível para tela cheia.
- **Camadas:** vetores da corrente, iluminação, cones dos sensores, conflitos de densidade e mapa químico (PG, PF, C0–C7, Todos).

### Ainda não portado (próximas versões)

- Salvar e carregar runs (.ecosrun), presets de sessão e presets de mapa. Use a V1.1.0 para salvar runs.
- Configurações neurais Val 1.0 e Val 2.0 (dependem de uma run carregada).
- Laboratório (o layout está reservado; as campanhas ainda rodam no executável antigo).

### Requisitos

Windows 10 ou 11 com o WebView2 (já vem no Windows 11 e nas atualizações do Windows 10).
