# Proposta: Sol contínuo, pincel de luz e visão pela luz

**Estado: proposta da V5, aguardando aprovação.** Registrada em 22/11/2026.
Base de código lida: `main` @ `3f41f7a` (V3.0.4 oficial).
Plano geral: [V5_FUNCOES_NOVAS.md](../Backlog/V5_FUNCOES_NOVAS.md).
Documento do estado atual: [LUZ_SOLAR_E_FITOPLANCTON.md](../Documentação/LUZ_SOLAR_E_FITOPLANCTON.md).

## 1. Intenção

Hoje o sol é liga/desliga: cada casa do mapa vale 0 (sombra) ou 1 (sol
pleno), e o sol é desenhado como um contorno fechado. A V5 transforma isso
numa **malha de intensidade contínua**, com:
- focos de luz desenhados com pincel e com borda esmaecida (*feathering*,
  como no Photoshop);
- uma luz basal no mapa inteiro, que nunca é zero;
- uma intensidade geral do sol;
- uma escala de cor para ver e consultar a luz em qualquer ponto.

Essa luz passa a controlar a reprodução e a vida do fitoplâncton e o alcance
da visão das células. A temperatura fica para uma etapa futura (seção 5.3).

## 2. Decisões já tomadas (22/11/2026)

| Tema | Decisão |
|---|---|
| Onde fica | aba **Física**, seção **Física do mapa › Sol** (sai da aba Mapa) |
| Desenho | pincel com tamanho e força da borda; contorno fechado com borda esmaecida, na mesma lógica de hoje |
| Luz mínima | existe luz basal em todo o mapa, quase zero mas nunca zero |
| Reprodução | o fitoplâncton precisa de uma intensidade mínima para se reproduzir (ver pergunta S1) |
| Visão | a luz limita o alcance da visão: entra na V5 |
| Consulta | escala de intensidade ligável no mapa |
| Temperatura | **não entra na V5** (decisão de 22/11, 1h44). Ela entra junto com o custo térmico e a corrente dependente da temperatura, numa etapa futura |
| Sol vivo (futuro) | depois desta proposta, os focos de luz vão se mover, crescer e diminuir com o tempo. A temperatura, quando existir, segue a luz por **correlação direta** (mais luz agora = mais quente agora), sem acumular calor |
| Presets | o preset de mapa guarda a malha e as configurações do sol |

## 3. A malha de luz

### 3.1 Estrutura

A grade de sol que já existe (`MapDefinition.sunlight`, 320 × 180 casas de
40 × 40 unidades) passa a guardar a intensidade pintada, de 0 a 255, no lugar
de 0 ou 1. Não muda a resolução nem a memória (57.600 bytes). Muda só o
significado do valor e a versão do formato do mapa.

### 3.2 Luz efetiva num ponto

```text
L(x) = luz_basal + (1 − luz_basal) × intensidade_do_sol × malha(x)
```

| Parâmetro | O que é | Faixa | Padrão |
|---|---|---|---:|
| `malha(x)` | o que foi pintado naquela casa | 0–1 | — |
| `intensidade_do_sol` | força geral do sol (um dia mais forte ou mais fraco) | 0–100% | 100% |
| `luz_basal` | luz que existe no mapa inteiro, até na sombra | 0–10% | 1% |

Com basal de 1%, a região mais escura recebe 0,01 e nunca chega a zero. Um
foco pintado a 10% com o sol a 100% recebe cerca de 0,11. A leitura é
interpolada entre casas vizinhas para o valor não "dar degrau" de uma casa
para a outra.

A seção mostra também a **luz média do mapa** (média de `L` sobre as casas
sem parede). É a "intensidade média" que você comentou, calculada a partir do
desenho e não digitada.

### 3.3 Mapas antigos

Mapas e saves antigos com sol 0/1 são convertidos assim: 1 vira 255 (foco
máximo) e 0 vira 0 (só luz basal). Com luz basal de 0% e limiar de reprodução
acima de zero, a simulação antiga se comporta como antes. Assim dá para
comparar.

## 4. Desenhar o sol

### 4.1 Ferramentas

Na seção Sol, com o mesmo padrão "desenha, depois aplica" do editor de mapa:

| Ferramenta | O que faz |
|---|---|
| **Pincel de luz** | pinta luz enquanto arrasta |
| **Área de luz** | desenha um contorno fechado (laço) e preenche, com borda esmaecida |
| **Sombra** | o mesmo pincel, mas tira luz |
| **Limpar** | zera a malha (fica só a luz basal) |

### 4.2 Parâmetros do pincel

| Parâmetro | O que controla | Faixa |
|---|---|---|
| **Tamanho** | raio total do pincel | 1–40 casas (40–1.600 unidades) |
| **Força da borda** | o quanto do raio é miolo cheio e o quanto é degradê. 100% = borda dura, sem degradê; 0% = tudo degradê, do centro até a borda | 0–100% |
| **Intensidade** | quanto de luz o pincel deposita no miolo | 0–100% |

### 4.3 Como o esmaecimento é calculado

Para cada casa a uma distância `d` do centro do pincel:

```text
miolo  = tamanho × força_da_borda
peso(d) = 1                                   se d ≤ miolo
        = suave((tamanho − d) / (tamanho − miolo))   se miolo < d < tamanho
        = 0                                   se d ≥ tamanho

suave(t) = t² × (3 − 2t)      (smoothstep: começa e termina sem quina)

malha(x) = max(malha(x), intensidade × peso(d))      pincel de luz
malha(x) = malha(x) × (1 − peso(d))                  sombra
```

O pincel de luz usa `max`: passar duas vezes no mesmo lugar não estoura a luz
acima da intensidade escolhida. Para ter um foco mais forte, você aumenta a
intensidade. Isso imita o pincel com opacidade do Photoshop e deixa o
resultado previsível.

**Área de luz:** para cada casa, mede-se a distância até a borda do contorno.
Dentro do contorno, a uma distância maior que a largura do esmaecimento, vale
`intensidade`. Na faixa junto à borda, vale o `suave` dessa distância. Fora,
nada. A largura do esmaecimento sai da mesma "força da borda", aplicada ao
tamanho da área. A conta é feita uma vez ao soltar o mouse, não a cada tick.

### 4.4 Ver e consultar a intensidade

- Um botão **Ver intensidade** liga um mapa de calor da luz sobre a arena,
  com uma **escala de cores** e os valores (0,00 … 1,00) no canto.
- Passando o mouse pela arena, aparece a luz `L` daquele ponto.
- A mesma camada fica em **Camadas › Mundo › Iluminação**, que hoje mostra só
  sol ou sombra.

## 5. O que a luz controla

### 5.1 Fitoplâncton

```text
reproduz se L ≥ luz_mínima_para_reproduzir
chance por tick = taxa_de_multiplicação × L
vida = vida_base × (1 + 0,5 × L)          (hoje: 1,5× só no sol pleno)
```

O limite de seis partículas por quadrado de 120 unidades continua.

### 5.2 Visão das células

```text
alcance_efetivo = alcance_genético × fator_de_luz(L)
fator_de_luz(L) = max(visão_no_escuro, min(1, L / luz_plena))
```

| Parâmetro | Significado | Padrão sugerido |
|---|---|---:|
| `luz_plena` | a partir dessa luz a visão é completa | 0,10 |
| `visão_no_escuro` | fração mínima de alcance no breu (encostar ainda "vê") | 10% |

A luz considerada é a do ponto onde está a célula que olha. O PF continua
funcionando no escuro. Aqui nasce o contraste ecológico: em focos de luz, a
visão é forte e há fitoplâncton reproduzindo; na penumbra, quem sobrevive é
quem fareja.

**Custo:** é uma consulta à malha por célula por tick, antes da visão. O
alcance efetivo menor também deixa a busca da visão mais barata no escuro.

### 5.3 Temperatura: adiada

Fica fora da V5. O que foi pensado até aqui, e fica guardado em
`PENDENCIAS_E_ROADMAP.md`:
- **temperatura = função direta da luz** daquele momento, sem acúmulo de
  calor;
- **custo térmico** para as células, de preferência com um **gene de
  temperatura preferida** por linhagem, para criar nichos;
- **corrente dependente da temperatura**;
- **sol vivo:** focos que se movem e variam, levando a temperatura junto.

Esses quatro entram juntos, porque uma temperatura sem efeito nenhum só
repetiria o mapa de luz.

## 6. Controles na aba Física › Sol

| Controle | Faixa | Padrão |
|---|---|---:|
| Intensidade do sol | 0–100% | 100% |
| Luz basal | 0–10% | 1% |
| Luz mínima para o fitoplâncton reproduzir | 0–100% | 0% (ver S1) |
| Luz para visão plena | 1–100% | 10% |
| Visão no escuro | 0–100% | 10% |
| Ferramentas e pincel | (seção 4) | |
| Ver intensidade | liga/desliga a escala | desligado |
| Aplicar sol / Descartar | igual ao editor de mapa | |

A luz média do mapa aparece como leitura, não como controle.

## 7. Saves e presets

- O formato do mapa (`MapDefinition`) sobe de versão, porque valores acima de
  1 passam a ser válidos.
- Os parâmetros do sol entram nas configurações de sessão (`.ecosrun`) e no
  preset de mapa.
- Runs antigas: sol 0/1 convertido como na seção 3.3; visão sem limite de luz
  enquanto o parâmetro não for gravado (`luz_plena = 0`, que desliga o fator).

## 8. Testes

- `L` nunca é menor que a luz basal e nunca passa de 1.
- O pincel com força 100% tem borda dura; com 0%, o peso cai continuamente
  até zero no raio.
- Pincel com `max`: duas passadas iguais dão o mesmo resultado que uma.
- Mapa antigo convertido reproduz o comportamento antigo com basal 0.
- Visão: no breu, o alcance é `visão_no_escuro × alcance`; na luz plena, igual
  ao de hoje.
- Determinismo: mesmo hash para a mesma seed.
- Benchmark com o fator de luz ligado e desligado.

## 9. Perguntas em aberto

| # | Pergunta | Recomendação |
|---|---|---|
| S1 | Você pediu uma luz mínima para o fitoplâncton reproduzir, e também quer que ele se reproduza "de qualquer forma", já que a luz nunca é zero. São compatíveis assim: o limiar existe, mas o padrão é **0%**, então a luz basal deixa reproduzir em todo canto, só que devagar. Subindo o limiar acima da basal, você cria desertos de verdade. Está bom? | Sim, padrão 0%. |
| S2 | A visão usa a luz no ponto da célula que olha, ou no ponto do alvo visto? | Da célula que olha: mais simples e mais barato. |
| S3 | Valores padrão de `luz_plena` (10%) e `visão_no_escuro` (10%) | Começar assim e calibrar nos presets. |
| S4 | A temperatura entra já na V5? | **Decidido (22/11):** não. Entra junto com custo térmico, corrente e sol vivo. |
