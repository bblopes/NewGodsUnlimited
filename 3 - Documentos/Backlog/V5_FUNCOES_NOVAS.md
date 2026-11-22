# V5 — Funções novas (plano para aprovação)

**Estado: em discussão.** Aberto em 22/11/2026; revisado no mesmo dia, à 1h e às 1h44.
Base: `main` @ `6ccbadc` (V4.0.0). Pasta de trabalho: `D:\Bruno\Times New Ramming\ECOSSYSTEM`. A V4 segue em paralelo com
otimização. Depois, as duas versões são fundidas.

## Como ler este documento

Cada item tem um estado:

- ❓ **precisa de definição**: falta você responder algo antes;
- 🟡 **aguardando aprovação**: especificado, falta o seu ok;
- ✅ **aprovado**: entra no cronograma de implementação.

Os detalhes de cada item ficam na proposta própria, em `3 - Documentos/Propostas/`.
Este documento junta as decisões, as perguntas e o cronograma.

## Regras para conviver com a V4

- Cada entrega da V5 sai de uma branch a partir da `main` atualizada.
- Função nova entra em módulo novo sempre que possível (`biomass.rs`,
  `genealogy.rs`, `light.rs`, `spotter.rs`…). O `world.rs` recebe só os
  ganchos mínimos, porque é o arquivo que a V4 mais mexe.
- V4 e V5 vão subir a versão do checkpoint e do mapa. Antes de fundir,
  combinamos uma numeração única e um leitor que entenda as duas.
- Toda entrega mantém o determinismo (mesma seed, mesmo hash no
  `benchmark_simulacao`) e mede o custo do que entrou.

---

## 1. Biomassa 🟡

Proposta: [PROPOSTA_BIOMASSA.md](../Propostas/PROPOSTA_BIOMASSA.md).

**Decidido:**
- morte por idade libera 75% da energia; morte por fome não libera nada;
- fitoplâncton que expira vira biomassa;
- sem cheiro, só visível; partículas soltas e sem limite de quantidade;
- gravidade **de curto alcance** que funde as partículas em bolotas;
- a célula come encostando, aos poucos;
- sem custo do corpo no nascimento;
- controles em Biologia › Biomassa.

**Aprovado (22/11):** gravidade de curto alcance.

**Sobre a conta da gravitação:** faz sentido do jeito que você descreveu.
A atração só existe até um alcance curto (150 unidades, crescendo com o
tamanho da bolota até no máximo 300) e vai a zero suavemente na borda.
O mapa é dividido numa grade com casas do tamanho desse alcance máximo, e
cada partícula só olha a própria casa e as 8 vizinhas. Isso deixa o custo
proporcional ao número de partículas, não ao quadrado. Como as partículas
próximas se fundem, cada casa tende a ter poucas bolotas. Ainda há uma
válvula: uma casa lotada é tratada como um ponto só, no centro de massa.

**Falta decidir:** D1 (o excedente da fagocitose vira biomassa?),
D2 (a corrente arrasta menos as bolotas grandes?), D3 (a gravidade atravessa
paredes?), D4 (mordida de 25% da unidade por tick?), D6 (cor ocre?).

## 2. Sol contínuo, pincel de luz e visão pela luz 🟡

Proposta: [PROPOSTA_SOL_E_LUZ.md](../Propostas/PROPOSTA_SOL_E_LUZ.md).

**Decidido:**
- O sol vira uma malha de intensidade contínua, desenhada na aba Física ›
  Sol:
  - pincel com tamanho, força da borda (esmaecimento tipo Photoshop) e
    intensidade;
  - área fechada com borda esmaecida;
  - ferramenta de sombra.
- Luz basal no mapa inteiro, nunca zero; intensidade geral do sol; leitura da
  luz média do mapa.
- Escala de intensidade ligável, com valor ao passar o mouse.
- A luz controla a reprodução (com limiar mínimo) e a vida do fitoplâncton.
- A luz limita o alcance da visão das células.
- **Temperatura não entra na V5** (decisão das 1h44). Ela entra depois, junto
  com o custo térmico, a corrente dependente da temperatura e o **sol vivo**
  (focos que se movem e variam; a temperatura segue a luz por correlação
  direta, sem acumular calor). Tudo guardado em `PENDENCIAS_E_ROADMAP.md`.

**Falta decidir:** S1 (limiar de reprodução com padrão 0%?), S2 (luz medida
na célula que olha?), S3 (valores iniciais da visão pela luz).

## 3. Genealogia real e aba Linhagens ✅ genealogia · 🟡 aba

Proposta: [PROPOSTA_LINHAGENS.md](../Propostas/PROPOSTA_LINHAGENS.md).

**Aprovado:** `parent_id`, `root_lineage_id`, `birth_tick` em cada célula e um
diário compacto de nascimentos na telemetria.

**Desenho proposto para a aba:**
- **Grupos** vira **Linhagens**.
- Como um fundador acaba dominando tudo, as linhagens se dividem em
  **ramos**: um ramo novo nasce quando um descendente já difere do fundador
  do ramo em mais de 18 bits.
- A aba mostra:
  - a lista de ramos vivos, com cor, tamanho e tendência;
  - uma árvore de ramos (os extintos em cinza);
  - o detalhe do ramo: o que ele inventou em circuitos em relação ao
    ramo-pai, destacar na arena, ver o genoma fundador.
- A Demografia ganha um gráfico de rio (**aprovado**) (participação de cada ramo ao longo
  do tempo).
- O modo Indivíduo mostra a mãe, o ramo e os filhos vivos.

**Falta decidir:** L1 (limiar de 18 bits?), L2 (tirar Grupos de vez?),
L3 (gráfico de rio numa entrega própria?).

## 4. Pacote genoma V5 🟡

Três mudanças no formato do genoma, numa única migração de save:

### 4.1 Mutação adaptativa

**Respondendo a sua pergunta:** sim, é **um grupo de genes para toda a
topologia, um para todos os pesos e um para toda a ativação**. Não existe um
regulador por trecho ou por ligação. São três genes de 4 bits (níveis de 0 a
15), cada um valendo para as 64 ligações da rede:

| Gene | Vale para | Bits que controla |
|---|---|---|
| topologia | origem e destino das 64 ligações | 768 |
| peso | peso das 64 ligações | 384 |
| ativação | o liga/desliga das 64 ligações **e** os 30 bits de atividade dos neurônios | 94 |

- O nível 8 é a taxa de hoje; o 0 dá 0,25×; o 15 dá cerca de 3,36×.
- Os reguladores mutam sempre na taxa fixa de hoje.
- O alcance da visão, a longevidade e os canais químicos também ficam na
  taxa fixa.

Falta decidir: faixa de 0,25× a 3,36× (recomendada) ou teto de 4×.
Proposta: [PROPOSTA_MUTACAO_ADAPTATIVA.md](../Propostas/PROPOSTA_MUTACAO_ADAPTATIVA.md).

### 4.2 Longevidade dentro do genoma

O gene da idade máxima passa a fazer parte do genoma: entra na cor, no
cheiro de parentesco (PG), na semelhança e na regra dos ramos. A taxa de
mutação fica igual (1/64 por nascimento, que é 16 bits × 1/1.024).

### 4.3 Canais químicos fora da fiação

Os 9 bits que escolhem P1, P2 e P3 saem de dentro da ligação #63 e ganham
uma região própria.

### 4.4 Formato

O genoma vai de 1.251 bits (157 bytes) para 1.292 bits (162 bytes). As regiões
novas entram no fim, então os índices atuais não mudam.

## 5. Behavior Spotter ✅

Proposta: [PROPOSTA_BEHAVIOR_SPOTTER.md](../Propostas/PROPOSTA_BEHAVIOR_SPOTTER.md).

**Decidido:**
- Aba nova abaixo de Camadas, com uma seção **Fagocitose**.
- A **lista ao vivo** das fagocitoses em curso está sempre disponível. **Ir
  até** e **Próxima** levam a câmera, que passa a rastrear o evento (segue o
  predador, com o círculo vermelho no ponto de contato).
- A opção **Pausar ao encontrar** pausa no primeiro tick com fagocitose e
  mostra os círculos; enquanto estiver ligada, fica pausado.
- Ao desligar a opção, a simulação volta a rodar sozinha e a câmera continua
  seguindo o predador.
- Ligar a opção com a simulação pausada não despausa; só aponta para a
  primeira ocorrência.
- Sem botão de avançar 1 tick (você usa a velocidade 1×). Sem salvar momento
  nem replay por enquanto.

## 5b. Sensor de idade, Baseline e fim das configurações neurais ✅ · especificação da V5.0.0 🟡

Proposta: [PROPOSTA_BASELINE_E_SENSOR_DE_IDADE.md](../Propostas/PROPOSTA_BASELINE_E_SENSOR_DE_IDADE.md).

**Decisão do Bruno (registro):** *vai morrer de idade, se reproduz de última
hora.* A célula precisa de senso de urgência para se reproduzir antes de
morrer de velhice; era um motor evolutivo forte que faltava. A reprodução
sexuada fica para um estudo futuro.

**Decidido:**
- Sensor novo de **idade relativa** (`Id`, de 0 a 1). A rede vai para 57 slots,
  com migração dos saves e do banco.
- Sai o seletor de **configuração neural** (Clássica, Motor, PF guia). A
  população inicial e as inserções vêm só do **banco de indivíduos salvos**.
- Novo indivíduo embutido **Baseline**:
  - `PFx`/`PFy` ligados direto aos motores de nado, como no PF guia mas sem os
    internos;
  - todos os internos desativados, e as ligações deles removidas;
  - todos os motores ativos;
  - `Id` → Reproduzir: dispara nos últimos ~11% da vida.
- Rede neural ao vivo: ligação inerte pode existir; passar o mouse em
  qualquer nó mostra o nome dele, com "inativo" quando estiver inativo.

**Falta decidir:** B1 (o mínimo de energia continua valendo na reprodução de
última hora?), B2 (janela de ~11%?), B3 (fundadores nascem com uma rodada de
mutação?), B4 (indivíduos do banco migrados?).

## 6. Interface: abas Física e Mapa, presets de mapa, modo noturno 🟡

Proposta: [PROPOSTA_INTERFACE_V5.md](../Propostas/PROPOSTA_INTERFACE_V5.md).

**Decidido:**
- **Ambiente vira Física**, com duas seções:
  - Física do mapa: Corrente e Sol;
  - Interações: o anti-cramming.
- A aba **Mapa** fica com:
  - **Construção**: só paredes (antes "Ferramentas");
  - **Presets de mapa**, voltando a funcionar como na V1, agora guardando
    paredes, sol e corrente.
- Vários mapas embutidos (labirintos e focos de luz), feitos no fim da V5.
- Botão de **modo noturno** ao lado de Pausar.

**Decidido:** o tema escuro atual vira o **noturno** e cria-se um **tema
claro**. Tudo o que for desenvolvido daqui em diante tem versão dia e noite.

**Falta decidir:** P1 (o anti-cramming fica fora do preset?), P3 (o nome
"Construção").

## 7. Candidatos que ainda não entraram

| Item | Observação |
|---|---|
| Livro-caixa de energia | balanço por tick que fecha. Recomendado junto da biomassa, para provar que a energia fica no sistema. **Entra?** |
| Marcos de configuração na Demografia | combina com os presets de mapa: carregar um preset vira um marco |
| Gravador de sessão | interface e nome em aberto |
| PF com memória (persistência, difusão, corrente) | conversa com o sol |
| Telemetria de predação | combina com o Behavior Spotter |

## 8. Título da versão maior

Cada versão maior tem um título, que vira o nome da pasta. Sugestões:
**"V5 - Luz e matéria"**, "V5 - Ecologia" ou "V5 - Ecossistema vivo". Qual?

---

## Cronograma proposto

Ordem pensada para entregar cedo o que você quer ver e para cada etapa
apoiar a próxima. Cada linha é uma entrega com `.exe` próprio.

| Versão | Conteúdo | Por que nessa ordem | Status |
|---|---|---|---|
| V5.0.0 | Sensor de idade (57 slots, migração), Baseline embutido, fim das configurações neurais, hover com nome nos nós da rede, nascimentos de última hora na Demografia | pedido do Bruno: o mecanismo de sobrevivência primeiro. Especificação: `PROPOSTA_BASELINE_E_SENSOR_DE_IDADE.md` | ⏭️ próxima (aguardando aprovação da especificação) |
| V5.0.1 | Behavior Spotter: lista ao vivo da fagocitose, Ir até / Próxima com rastreio, opção Pausar ao encontrar, círculo vermelho no ponto de contato | decidido; independente e pequeno | pendente |
| V5.0.2 | Abas novas (Física com Corrente e Interações; Mapa com Construção) + tema claro (o atual vira noturno) | prepara o lugar do sol e o tema de tudo que vem depois | pendente |
| V5.0.3 | Presets de mapa de volta (paredes + corrente + sol atual), importar e exportar | você precisa para desenhar mapas melhores | pendente |
| V5.1.0 | Malha de luz contínua: luz basal, intensidade, limiar de reprodução, vida interpolada, escala de intensidade, conversão dos mapas antigos | base de toda a parte de sol | pendente |
| V5.1.1 | Pincel de luz com esmaecimento, área com borda, sombra; o preset passa a guardar a malha | precisa da malha contínua | pendente |
| V5.1.2 | Visão limitada pela luz | precisa da malha; muda a regra, então tem medição de comportamento | pendente |
| ~~V5.1.3~~ | ~~Temperatura só visual~~ | adiada: entra com custo térmico, corrente e sol vivo | ❌ descartada da V5 |
| V5.2.0 | Biomassa: entidade, liberação na morte, gravidade de curto alcance, fusão, comer por contato, controles, telemetria | a maior mudança de regra; entra depois da luz para calibrar junto | pendente |
| V5.2.1 | Livro-caixa de energia (se aprovado) | valida a biomassa | *a decidir* |
| V5.3.0 | Genealogia e ramos no motor; Indivíduo com mãe e filhos | aprovado; base da aba | pendente |
| V5.3.1 | Aba Linhagens (lista, árvore, detalhe, destacar na arena) | precisa da genealogia | pendente |
| V5.3.2 | Gráfico de rio na Demografia | aprovado; precisa da amostragem por ramo | pendente |
| V5.4.0 | Pacote genoma: mutação adaptativa, longevidade no genoma, canais químicos | uma migração só; depois da genealogia, para os ramos já verem a longevidade | pendente |
| V5.5.0 | Mapas embutidos: labirintos e focos de luz | por último, com todas as regras calibradas | pendente |

## Documentos corrigidos em 22/11/2026

Conferidos contra o código da `main`:
- `PROPOSTA_INTERACAO_CELULAR_E_COMBATE.md`: a fagocitose, os sensores de
  força e o contato estão implementados, com os números reais.
- `PROPOSTA_MUTACAO_ADAPTATIVA.md`: tamanho atual do genoma.
- `DOCUMENTACAO_ANALISE_GENETICA.md`: genoma atual.
- `BACKLOG.md`, `PENDENCIAS_E_ROADMAP.md`, `LUZ_SOLAR_E_FITOPLANCTON.md`,
  `PG_E_CANAL_DE_ALIMENTO.md`: visão + PF consta como publicada.
