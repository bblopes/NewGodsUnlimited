# Proposta: abas Física e Mapa, presets de mapa e modo noturno

**Estado: proposta da V5, aguardando aprovação.** Registrada em 22/11/2026.
Base de código lida: interface Tauri 3.0.4 (`interface/ui/app.js`) e
interface V1 (`src/app.rs`, presets de mapa).
Plano geral: [V5_FUNCOES_NOVAS.md](../Backlog/V5_FUNCOES_NOVAS.md).

## 1. Abas da lateral

### 1.1 Hoje

Sessão · Biologia · **Ambiente** (Corrente, Densidade) · **Mapa**
(Ferramentas com parede, borracha e sol; Correnteza do mapa; Presets "chega
na próxima versão") · Células · Demografia · Camadas.

A corrente aparece duas vezes: o modelo está em Ambiente e de novo em Mapa.

### 1.2 Proposta

| Aba | Seções |
|---|---|
| Sessão | sem o seletor de configuração neural: a população inicial vem de um indivíduo do banco (ver `PROPOSTA_BASELINE_E_SENSOR_DE_IDADE.md`) |
| Biologia | Células · Fitoplâncton · **Biomassa** (nova) |
| **Física** (era Ambiente) | **Física do mapa:** Corrente (ligada, intensidade, modelo) e **Sol** (ver `PROPOSTA_SOL_E_LUZ.md`) · **Interações:** anti-cramming (era Densidade) |
| **Mapa** | **Construção** (paredes) · **Presets de mapa** |
| Células | Indivíduo · Genética (Resumo, **Linhagens**, Circuitos) |
| Demografia | igual, mais o gráfico de rio das linhagens |
| Camadas | Mundo · Mapa químico (a seção Clima espera a temperatura) |
| **Behavior Spotter** | nova, logo abaixo de Camadas |

- A corrente sai da aba Mapa e fica só em Física. Quando ela passar a
  depender do sol, já está no lugar certo.
- O sol sai da ferramenta do Mapa e ganha a seção própria em Física.

### 1.3 "Ferramentas" passa a se chamar **Construção**

Fica só o desenho do mapa físico: **Selecionar · Parede · Borracha**, raio do
pincel, Desfazer, **Aplicar mapa** e Descartar. O nome diz o que é: construir
as paredes da câmara. A outra opção que considerei foi "Arquitetura".

## 2. Presets de mapa (voltando a funcionar)

### 2.1 Como era na V1 (`src/app.rs`)

- Arquivo `presets_mapa.ecomap` na pasta de dados do app, com até 24 mapas.
- Cada preset tinha nome, desenho do mapa (paredes e sol) e modelo de
  corrente.
- Ações: **salvar** (com o mesmo nome, substitui), **carregar** (já aplica no
  mundo, sem um segundo clique) e **apagar**.
- Dois mapas embutidos, que o programa repara sozinho se o arquivo estiver
  velho: "Oásis original" e "3.344 · New Gods".
- Um modal para escolher, com campo de nome.

### 2.2 Como fica na V5

Mesmo funcionamento, com o conteúdo ampliado para tudo o que é físico:

| Vai no preset | Origem |
|---|---|
| Paredes | Mapa › Construção |
| Malha de sol pintada | Física › Sol |
| Intensidade do sol, luz basal, luz mínima para reproduzir, luz para visão plena, visão no escuro | Física › Sol |
| Corrente: ligada, intensidade, modelo | Física › Corrente |
| Anti-cramming | Física › Interações (ver pergunta P1) |

- **Carregar** aplica tudo de uma vez e gera um marco na telemetria (quando
  os marcos existirem).
- **Salvar** grava o estado *aplicado* do mundo, não o rascunho, para não
  salvar um desenho pela metade.
- O formato ganha uma versão nova (`ECOMAP02`). Presets da V1 (`ECOMAP01`)
  continuam abrindo: o sol 0/1 é convertido e os campos novos ficam no padrão.
- Na interface Tauri, em vez de modal, a seção **Presets de mapa** mostra a
  lista direto na aba, com Carregar, Salvar como… e Apagar, e uma miniatura
  do mapa (paredes e luz) em cada item.
- Também entram **Exportar** e **Importar** um preset como arquivo, para
  trocar mapas entre computadores.

### 2.3 Mapas embutidos (no fim da V5)

Cada um combina um desenho de paredes com um padrão de luz, sempre com luz
basal no mapa inteiro. Primeiras ideias:

| Nome | Paredes | Luz |
|---|---|---|
| Oásis original | o de hoje | foco único central, borda suave |
| Labirinto clássico | corredores em grade com portas | luz basal baixa e pequenos focos em becos sem saída, o que premia quem explora |
| Arquipélago | sem paredes | várias ilhas de luz de tamanhos diferentes, com penumbra entre elas |
| Fosso profundo | uma bacia fechada por paredes, com uma só entrada | luz forte fora, quase breu dentro: teste para quem fareja PF |
| Gradiente | sem paredes | luz crescendo do oeste para o leste, para ver nichos se formarem ao longo do gradiente |
| Duas câmaras | duas salas ligadas por um corredor estreito | uma sala clara e uma escura, para medir migração e especialização |
| Espiral | uma parede em espiral até o centro | foco no centro: chegar lá custa caminho |
| Recifes | blocos de parede espalhados | focos atrás dos blocos, protegidos da corrente |
| 3.344 · New Gods | o de hoje | o foco convertido para malha contínua |

Os mapas são desenhados e calibrados no fim da V5, depois que sol, visão e
biomassa existirem, porque o equilíbrio de cada mapa depende dessas regras.

## 3. Modo noturno

Pedido: um botão ao lado de Pausar, na barra superior, que liga e desliga o
modo noturno. Tudo o que for estético precisa ter as duas versões.

**Decidido (22/11/2026):** o tema escuro de hoje passa a ser o **noturno**, e
cria-se um **tema claro** (dia). O botão alterna entre os dois.

Como fica:
- as cores viram variáveis de tema (`:root` e `[data-theme=…]`), e tudo que
  já usa `var(--…)` acompanha sozinho;
- a arena (fundo, paredes, luz, mapas de calor, círculo do Behavior Spotter)
  ganha uma paleta para cada tema;
- a escolha fica salva e volta ao abrir o app;
- a regra fica escrita na documentação da interface: nenhum componente novo
  usa cor fixa, só variáveis de tema.

## 4. Perguntas em aberto

| # | Pergunta | Recomendação |
|---|---|---|
| P1 | O anti-cramming (Interações) entra no preset de mapa? | Não. Ele é regra de convivência das células, não do lugar. Você disse que talvez ele mude de aba, e isso reforça deixá-lo fora. |
| P2 | Modo noturno | **Decidido:** o atual vira noturno; cria-se o tema claro. Tudo o que for desenvolvido tem versão dia e noite. |
| P3 | O nome da seção: "Construção" está bom? | Sim. |
