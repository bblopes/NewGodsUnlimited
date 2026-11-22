# Proposta: biomassa, gravitação e reciclagem de energia

**Estado: proposta da V5, aguardando aprovação.** Registrada em 22/11/2026;
revisada no mesmo dia (gravidade de curto alcance, custo do corpo descartado).
Base de código lida: `main` @ `3f41f7a` (V3.0.4 oficial).
Plano geral da V5: [V5_FUNCOES_NOVAS.md](../Backlog/V5_FUNCOES_NOVAS.md).

## 1. Intenção

Hoje, quando uma célula morre de velhice, toda a energia dela some, e o
fitoplâncton que expira também some. A biomassa mantém essa energia no
sistema. Ela é matéria inerte: não se reproduz, não envelhece, não some
sozinha e não tem cheiro. As partículas se atraem como se tivessem
gravidade e vão se juntando em bolotas cada vez maiores. Uma célula que
encosta numa bolota vai comendo, e a bolota encolhe.

## 2. Decisões já tomadas (22/11/2026)

| Tema | Decisão |
|---|---|
| Célula que morre de velhice | libera **75%** da energia que tinha (rendimento configurável) |
| Célula que morre de fome | não libera nada: morreu sem energia, perde tudo |
| Célula fagocitada | a energia vai para o predador, como hoje (ver decisão D1 sobre o excedente) |
| Fitoplâncton que expira | vira 1 partícula de biomassa com a energia de um fitoplâncton |
| Fitoplâncton comido | não deixa biomassa |
| Tamanho da unidade | a energia por fitoplâncton (controle que já existe). Ex.: 800 × 75% = 600; com 100 por unidade, saem 6 partículas |
| Quantidade | sem limite, e a biomassa não decai |
| Movimento próprio | gravitação: as partículas se atraem e se fundem |
| Cheiro | não emite PF nem nenhum outro químico |
| Percepção | só é vista, pela visão genérica (`V<`, `V>`, `V^`, `Vv`), como qualquer entidade |
| Comer | automático, por contato, aos poucos: dá para comer só de passagem |
| Custo do corpo no nascimento | fora desta proposta (ver D5) |

## 3. A entidade

Existe um único tipo novo, com posição e energia:

```text
Biomassa { posição, energia }
```

"Bolinha" e "bolota" são a mesma entidade em tamanhos diferentes. A área é
proporcional à energia, como no corpo das células:

```text
raio = 5 × √(energia / 50)
```

`5` é o raio do fitoplâncton e `50` a energia padrão de um fitoplâncton. A
referência é fixa, para o tamanho de uma bolota não mudar quando alguém mexer
no controle de energia por fito.

| Energia | Raio | Comparação |
|---:|---:|---|
| 50 | 5,0 | um fitoplâncton |
| 100 | 7,1 | |
| 600 | 17,3 | um pouco maior que uma célula em repouso (14) |
| 2.000 | 31,6 | parecido com uma célula de 2.000 de energia (28) |
| 10.000 | 70,7 | uma "ilha" |

Cada partícula guarda a própria energia. Por isso, mudar a energia por fito
no meio da run não altera a biomassa que já existe.

## 4. De onde vem

### 4.1 Morte de célula por idade

```text
energia_liberada = rendimento × energia_no_momento_da_morte
n     = parte inteira de (energia_liberada / energia_por_fito)
resto = energia_liberada − n × energia_por_fito
```

Nascem `n` partículas com uma unidade cada e, se `resto > 0`, mais uma com o
resto. Nenhuma energia some por arredondamento.

Exemplos com 100 por unidade e rendimento de 75%:

| Energia ao morrer | Liberada | Partículas |
|---:|---:|---|
| 800 | 600 | 6 × 100 |
| 650 | 487,5 | 4 × 100 + 1 × 87,5 |
| 90 | 67,5 | 1 × 67,5 |

As partículas nascem distribuídas num anel do tamanho do raio da célula, com
um ângulo inicial derivado do `id` dela (é determinístico). Se uma posição
cair dentro de parede, a partícula nasce no centro da célula. Como se atraem,
elas se juntam em poucos ticks: o corpo se desfaz e vira uma bolota.

### 4.2 Morte de fitoplâncton por idade

Na limpeza do fitoplâncton (etapa 12 do tick), cada partícula que expirou sem
ser comida deixa uma biomassa no mesmo ponto, com a energia por fito em vigor.

### 4.3 Morte por fome

Não gera nada. A célula morre justamente porque a energia chegou a zero.

## 5. Gravitação

### 5.1 Regra: gravidade de curto alcance

A atração **não é infinita**. Ela só existe até uma distância curta (o
alcance, padrão 150 unidades, cerca de 4 casas da malha do mapa). Além disso,
ela vai a zero suavemente perto do limite, para nenhuma partícula "pular"
quando cruza a borda do alcance:

```text
velocidade_i = Σ  G × energia_j × suavização(d_ij) / (d_ij² + ε²)   na direção de j
               j ≠ i, d_ij ≤ alcance

suavização(d) = (1 − (d / alcance)²)²     → 1 perto, 0 exatamente no alcance
velocidade_i  limitada a v_máx
```

O alcance cresce um pouco com o tamanho da bolota, com teto:

```text
alcance_i = min(alcance_base + 2 × raio_i, alcance_máximo)     (150 → no máximo 300)
```

Assim uma bolota grande "segura" uma vizinhança um pouco maior, mas nunca
o mapa inteiro. O `alcance_máximo` define o tamanho da casa da grade de
cálculo (5.3), e é isso que garante o custo baixo.

- O movimento é amortecido, como tudo no mapa: a força vira velocidade direto,
  sem inércia. A biomassa não orbita, ela escorre na direção da massa.
- Uma bolota grande puxa as pequenas rápido e quase não se mexe por causa
  delas. Daí sai o comportamento de "bola de neve".
- `ε` (por exemplo 10 unidades) evita força infinita a curta distância.
- `v_máx` (por exemplo 1,5 unidade por tick) mantém o movimento suave e
  impede que uma partícula atravesse outra sem colidir.

### 5.2 Fusão

Quando duas biomassas se tocam (`d ≤ raio_a + raio_b`), viram uma só:

```text
energia  = energia_a + energia_b
posição  = centro de massa (média das posições ponderada pela energia)
```

Em cada tick, as fusões são resolvidas em ordem de índice, para o resultado
ser reproduzível com a mesma seed.

A fusão também resolve o desempenho: a quantidade não tem limite, mas
partículas soltas tendem a virar poucas bolotas grandes.

### 5.3 Custo de cálculo (por que a conta faz sentido)

Comparar todas as partículas entre si seria quadrático: com 5.000 partículas,
25 milhões de pares por tick. Com alcance curto, só importam as vizinhas:

1. A biomassa é distribuída numa grade cujas casas têm o tamanho do alcance
   máximo (300 unidades: 43 × 24 casas no mundo de 12.800 × 7.200).
   Montar essa grade custa `O(n)`.
2. Tudo o que pode puxar uma partícula está na casa dela ou nas 8 vizinhas.
   Nada mais longe é olhado.
3. **Perto** (na própria casa e nas vizinhas), a conta é exata, partícula a
   partícula. Como as partículas próximas se fundem, uma casa costuma ter
   poucas bolotas, e o custo fica perto de `O(n)`.
4. **Válvula de segurança:** se uma casa passar de um limite (por exemplo 64
   partículas), ela é tratada como um único ponto no centro de massa dela.
   Isso evita o pior caso quando a gravitação estiver desligada e as
   partículas se acumularem soltas.
5. A fusão usa a mesma grade: só testa pares de casas vizinhas.

É o mesmo princípio que tornou o PF barato na V3: agrupar antes de espalhar.
A estimativa é um custo da ordem do movimento do fitoplâncton. O número real
sai do `benchmark_simulacao`, com e sem biomassa, na entrega.

### 5.4 Paredes e corrente

- A parede bloqueia o movimento (`constrain_to_walls_fast`, igual ao
  fitoplâncton). Na primeira versão a atração atravessa paredes (D3).
- A corrente também arrasta a biomassa, com acoplamento que cai com o
  tamanho (D2):

```text
acoplamento = 0,42 / √(energia / 50)
```

  Com isso, uma partícula pequena se move como fitoplâncton e uma bolota
  grande vira quase um recife parado.

## 6. Comer biomassa

- **Contato:** a célula come quando `distância ≤ raio_célula + raio_biomassa`.
  O fitoplâncton exige que o centro dele esteja dentro da célula; a biomassa
  basta encostar, porque pode ser maior que a própria célula.
- **Mordida por tick:**

```text
mordida = fração_de_mordida × energia_por_fito × área_relativa_da_célula
```

  Com o padrão de 25%, uma célula em repouso com 100 por unidade come 25 por
  tick. Uma passagem de 10 ticks encostada rende cerca de 2,5 fitoplânctons.
  Células maiores comem mais rápido, coerente com as outras escalas de
  tamanho (D4).
- **Teto:** a célula só tira o que cabe até o limite de reprodução. O resto
  fica na bolota.
- **Vários comendo a mesma bolota:** as células são processadas em ordem de
  `id`, e cada uma tira `min(mordida, o que sobrou)`.
- **Uma célula encostada em várias bolotas:** come só da mais próxima.
- **Encolhimento:** o raio sai da energia, então a bolota diminui sozinha. Com
  menos de 0,5 de energia, ela é removida.
- Comer fitoplâncton e biomassa no mesmo tick é permitido.

## 7. Como a célula identifica a biomassa

**Proposta: nenhum sensor novo na primeira versão.**

- A visão genérica já responde a qualquer entidade e passa a enxergar a
  biomassa também. Ela não diz *o que* é, do mesmo jeito que hoje não
  distingue fitoplâncton de célula.
- O sensor de energia (`En`) sobe quando a célula está comendo. Isso basta
  para a seleção aprender "vi algo, fui até lá, a energia subiu, fico".
- Como a biomassa não tem cheiro, existe uma diferença ecológica real: no
  escuro, quem segue PF acha fitoplâncton; quem usa visão acha também a
  biomassa.

Fica registrado como opção futura, se a evolução mostrar que falta
informação: um sensor `Cb` (encostando em biomassa, 0 ou 1). Seria barato
porque reaproveita o teste de contato. Não entra agora.

## 8. Onde regular (interface)

Na aba **Biologia**, uma seção nova **Biomassa**, logo abaixo de
**Fitoplâncton**. Segue o padrão das outras: vale ao clicar em Aplicar e
gera marco na telemetria quando essa função existir.

| Controle | Faixa | Padrão |
|---|---|---:|
| Rendimento na morte | 0–100% | 75% |
| Força da gravitação (`G`) | 0 (desligada) até o máximo calibrado | a calibrar |
| Alcance da gravitação | 50–300 unidades | 150 (cresce com o tamanho até 300) |
| Mordida por tick | 0–100% da unidade | 25% |

Também entram:
- **Arena:** a biomassa é desenhada em ocre ou marrom, com raio pela energia.
- **Camadas:** um botão para mostrar ou esconder a biomassa.
- **Barra superior / Sessão:** contagem de bolotas e energia total em biomassa.

O executável Macroquad antigo (`app.rs`) não recebe esses controles. Só a
interface Tauri, que é a oficial.

## 9. Telemetria

Em cada amostra da Demografia:
- quantidade de biomassas e energia total guardada nelas;
- energia que entrou vindo de célula e vindo de fitoplâncton;
- energia consumida pelas células;
- maior bolota e distribuição de tamanhos.

Isso conversa com o **livro-caixa de energia**: com a biomassa, o balanço
`energia nas células + fitoplâncton + biomassa` precisa fechar com as entradas
(sol e abiogênese) e as saídas (metabolismo, rendimento perdido).

## 10. Ordem dentro do tick

Etapas atuais em `World::tick()`, com os ganchos novos:

| Etapa | Hoje | Com biomassa |
|---|---|---|
| 6 | movimento e reprodução do fitoplâncton | **6b:** movimento da biomassa (corrente + gravitação) e fusões |
| 8 | interações (fagocitose, anti-cramming, doação) | excedente da fagocitose vira biomassa, se D1 for aprovada |
| 10 | mortes | morte por idade gera biomassa |
| 11 | alimentação | depois do fitoplâncton, mordidas na biomassa |
| 12 | limpeza do fitoplâncton | fitoplâncton expirado vira biomassa |

## 11. Saves

- A versão do checkpoint sobe. A lista de biomassas é gravada depois do
  fitoplâncton.
- Runs antigas carregam sem biomassa e com os controles no padrão.
- `.ecosrun` e presets ganham os quatro parâmetros novos.

## 12. Convivência com a V4

A V4 mexe no laço do tick e no paralelismo do `world.rs`. Para a fusão das
duas versões dar pouco conflito:
- toda a lógica fica num módulo novo, `src/simulation/biomass.rs`;
- o `world.rs` recebe só os cinco ganchos da tabela acima;
- a versão do checkpoint deve ser combinada entre V4 e V5 antes de fundir.

## 13. Testes

- Morte por idade com 800 e unidade 100 gera 6 partículas somando 600.
- 650 gera 4 × 100 + 87,5.
- Morte por fome não gera nada.
- Fitoplâncton expirado gera 1 biomassa; fitoplâncton comido não gera.
- Duas partículas que se tocam se fundem conservando energia e centro de massa.
- Com `G = 0` e sem corrente, a biomassa fica parada.
- A mordida respeita o teto de energia e a ordem por `id`.
- A biomassa não aparece no PF; a visão a enxerga.
- Ida e volta no save mantém a biomassa idêntica.
- O mesmo hash do `benchmark_simulacao` para a mesma seed (determinismo).
- Benchmark com e sem biomassa, publicado em `MEDICAO_DE_DESEMPENHO.md`.

## 14. Decisões em aberto

| # | Pergunta | Recomendação |
|---|---|---|
| D1 | Quando o predador já está no teto, a energia que sobra da presa vira biomassa? | Sim, com o mesmo rendimento. Hoje essa energia some. |
| D2 | A corrente arrasta menos as bolotas grandes? | Sim (acoplamento ∝ 1/raio). |
| D3 | A gravidade atravessa paredes? | Na primeira versão, sim. Bloquear depois, se aparecerem bolotas grudadas na parede. |
| D4 | Mordida de 25% da unidade por tick, proporcional à área da célula? | Sim, para começar. Calibrar observando. |
| D5 | Custo do corpo no nascimento | **Decidido (22/11):** fica fora. Quem morre de fome perde tudo. |
| D6 | Cor da biomassa na arena | Ocre ou marrom, diferente do verde do fitoplâncton. |
