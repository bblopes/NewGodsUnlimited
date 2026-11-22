# V5.0.0 — Sensor de idade, indivíduo Baseline e fim das configurações neurais

**Estado: aprovada em 22/11/2026 e implementada na V5.0.0.**
Escrita em 22/11/2026, a partir do código da `main` @ `6ccbadc` (V4.0.0).
Plano geral: [V5_FUNCOES_NOVAS.md](../Backlog/V5_FUNCOES_NOVAS.md).

## 1. Por que esta entrega vem primeiro

> "Vai morrer de idade, se reproduz de última hora." (decisão do Bruno, 22/11/2026)

Hoje uma célula que vive muito, mas não chega ao limite obrigatório de
energia, morre de velhice sem deixar filhos. Nenhuma célula sabe que está
velha. A V5.0.0 dá à rede um **sensor de idade** e cria o indivíduo de partida
**Baseline**, que nada pelo cheiro da comida e se reproduz quando está perto
de morrer. Esse é um motor evolutivo que faltava.

A reprodução sexuada fica para um estudo futuro.

## 2. O que você vai ver no `.exe`

1. A aba **Sessão** não tem mais "Configuração neural" (Clássica, Motor, PF
   guia). No lugar dela, **População inicial** lista os indivíduos do banco,
   com o **Baseline** sempre em primeiro lugar e selecionado por padrão.
2. **Reiniciar** cria a run com cópias do indivíduo escolhido. **Inserir
   célula** insere uma cópia do mesmo indivíduo.
3. Na rede ao vivo (Células › Indivíduo), aparece um sensor novo, **Id**
   (idade), logo abaixo de `PFy`. Passar o mouse em qualquer nó mostra na hora
   o nome dele, e "inativo" quando ele estiver inativo.
4. Rodando com o Baseline, as células seguem o cheiro do fitoplâncton e,
   quando chegam a cerca de 89% da vida máxima, se dividem, desde que tenham
   energia acima do mínimo de reprodução.
5. Na **Demografia**, essas divisões entram como **nascimentos por decisão
   neural**, como qualquer outra decisão da rede (decisão D4: sem série
   própria).

## 3. Sensor de idade

| Item | Definição |
|---|---|
| Nome curto | `Id` |
| Nome completo | Sensor · idade relativa |
| Valor | `idade_em_ticks / idade_máxima_da_célula`, limitado a 0–1 |
| Leitura | 0 ao nascer; perto de 1 no último tick de vida |
| Custo de energia | nenhum, como o sensor de energia `En` |
| Posição na rede | slot **26**, logo depois de `PFy` |

A idade máxima é a do gene de longevidade da própria célula (de 1.000 a 11.000
ticks). Então o sensor diz "quanto da *minha* vida já passou", e não a idade
absoluta. Uma linhagem longeva e uma de vida curta recebem o mesmo tipo de
aviso.

Como desde a V3.0.4 o motor só calcula sensores que alguma ligação ativa lê,
células que não ligam o `Id` não pagam nada a mais.

### 3.1 Nova topologia

| Faixa | Antes (56 slots) | Depois (57 slots) |
|---|---|---|
| Sensores | 0–25 | 0–25 + **26 = Id** |
| Internos `I0`–`I15` | 26–41 | 27–42 |
| Temporizadores `T1`–`T3` | 42–44 | 43–45 |
| Motores (L, R, U, D, Re, Fa, EPG, EP1, EP2, EP3, Do) | 45–55 | 46–56 |

- Os identificadores de 6 bits continuam cabendo (até 63).
- **O tamanho do genoma não muda** (1.251 bits, 157 bytes). Os 30 bits de
  atividade dos neurônios continuam os mesmos, porque o número de internos e
  motores não muda.

## 4. Migração de tudo o que já existe

Como o tamanho do genoma não muda, a versão antiga é reconhecida por uma
marca de formato, e não pelo tamanho.

| O quê | Como é reconhecido | O que acontece ao carregar |
|---|---|---|
| Checkpoint / `.ecosrun` | versão do checkpoint sobe (mesmo padrão de `read_checkpoint_pre_food_pheromone`) | em cada genoma, toda referência a slot ≥ 26 vira +1 |
| Banco de indivíduos (`banco_de_individuos.json`) | `FORMATO_DO_BANCO` passa de 1 para 2 | os indivíduos são migrados (+1) e o arquivo é regravado no formato 2. Antes, é feita uma cópia `banco_de_individuos.v1.json` |
| Fundadores embutidos e presets de sessão | gerados pelo código | já nascem no formato novo |

A função nova `Genome::from_pre_age_sensor_bytes` copia a lógica de
`from_pre_food_pheromone_bytes`: desloca origem e destino ≥ 26 em +1 e
desliga registros que apontavam para fora da topologia antiga.

**Garantia:** um genoma migrado tem exatamente as mesmas ligações efetivas que
tinha. Nenhuma ligação antiga passa a ler o `Id`. O sensor só entra em uso por
mutação, ou nos indivíduos novos como o Baseline.

## 5. Fim das configurações neurais

| Onde | Antes | Depois |
|---|---|---|
| Sessão › população inicial | Clássica / Motor / PF guia / indivíduos salvos | **só indivíduos do banco**, Baseline primeiro |
| Inserir célula | mesma lista | mesma lista nova |
| Run antiga carregada | — | continua como estava (a população vem do save) |

- O banco ganha o **Baseline** e o **PF guia** (com a reprodução de última
  hora) como indivíduos embutidos. Eles não ficam no arquivo JSON: são
  gerados pelo código, nunca somem, mesmo se o arquivo for apagado, e não
  podem ser excluídos. Aparecem como "embutido".
- Os indivíduos que você já salvou, como o "Rede de espaçamento", continuam
  na lista, migrados.
- **Na biblioteca do motor**, as configurações antigas (Clássica, Motor, Val,
  PF guia) continuam existindo, porque o Laboratório e o `benchmark_simulacao`
  as usam. Só saem da interface.
- O executável antigo da V1 (`src/app.rs`) recebe apenas o nome do slot novo,
  para continuar compilando e mostrando a rede sem erro.

## 6. O indivíduo Baseline

É o PF guia **sem os internos**, mais a reprodução de última hora.

### 6.1 Nós

| Grupo | Estado no Baseline |
|---|---|
| Sensores | existem todos; ele só lê `PFx`, `PFy`, `Id` e `+1` |
| Internos (`I0`–`I15`, `T1`–`T3`) | **todos inativos** |
| Motores (os 11) | **todos ativos**, inclusive fagocitar, emitir PG/P1/P2/P3 e doar; esses só não têm ligação nenhuma por enquanto |
| Alcance da visão | o padrão de hoje |
| Canais químicos P1/P2/P3 | os padrões de hoje |

### 6.2 Ligações (26 das 64)

Os pesos são o máximo que o genoma permite (±1, ou seja ±8 passos de 1/8).

**Navegação pelo cheiro (12 ligações), como no PF guia:**

```text
PFx ─(+1) ×3→ nadar à direita      PFx ─(−1) ×3→ nadar à esquerda
PFy ─(+1) ×3→ nadar acima          PFy ─(−1) ×3→ nadar abaixo
```

As 12 ligações dos internos para os motores, que o PF guia usava para a
inibição cruzada, **saem**: sem internos ativos elas seriam ligações inertes,
sem efeito. Com isso, o Baseline **nada na diagonal** quando sente cheiro em
X e em Y ao mesmo tempo. A diagonal é a direção real do gradiente.

**Reprodução de última hora (14 ligações):**

```text
Id ─(+1) ×8→ Reproduzir
+1 ─(−1) ×5→ Reproduzir
+1 ─(−0,75) ×1→ Reproduzir
```

O motor usa a sigmoide centrada `2/(1+e^−x) − 1`, e Reproduzir só conta com
ativação ≥ 0,60, ou seja, entrada ≥ 1,386:

```text
entrada = 8 × Id − 5,75      →  dispara quando Id ≥ 0,89
```

| Idade relativa | Entrada | Ativação de Reproduzir | Quer reproduzir? |
|---:|---:|---:|---|
| 0,50 | −1,75 | 0 | não |
| 0,85 | 1,05 | 0,48 | não |
| 0,89 | 1,37 | ≈ 0,60 | começa aqui |
| 0,95 | 1,85 | 0,73 | sim |
| 1,00 | 2,25 | 0,81 | sim |

Com a vida padrão de 3.000 ticks, a janela é de ~330 ticks. Nas outras células
ela muda com o gene de longevidade.

Todas as 26 ligações sofrem a mutação normal nos filhos. A janela é definida
pelos pesos que saem do sensor de idade: a seleção pode antecipar, atrasar ou
apagar a urgência.

**Hipótese do Bruno (registro, 22/11/2026):** esta amarração não deve sumir
das populações, porque dá sempre vantagem: permite que as vantagens
evolutivas permaneçam. Hoje muita célula vive muito e morre por idade sem
deixar filhos. Se uma linhagem perder a amarração, ela deixa de se manter
otimizando.

### 6.3 Fundadores de uma run

- A run começa com N cópias do indivíduo escolhido, como já acontece com os
  indivíduos do banco. A longevidade continua com variação de ±15%.
- **Assumido (D2):** cada fundador nasce com **uma rodada de mutação
  normal**, como se fosse filho do indivíduo salvo. Assim a run não começa com
  clones perfeitos. A opção "Amadeus" (uma célula só) continua.

## 7. Regra de reprodução

**Assumido (D1): o mínimo de energia continua valendo.** A divisão de última
hora acontece só se `energia ≥ mínimo de reprodução`, como qualquer decisão
neural de reproduzir. Motivo: um filho com quase nada de energia morre logo
de fome e só gasta a mãe. Se a seleção quiser aproveitar a urgência, a
população evolui para chegar à velhice com reserva.

Nada mais muda na reprodução: o filho continua recebendo metade da energia, e
o limite obrigatório continua forçando a divisão.

## 8. Rede neural ao vivo

| Pedido | Como fica |
|---|---|
| Ligação inerte pode existir | continua aparecendo tracejada |
| Ver o nome ao passar o mouse | tooltip próprio da interface, instantâneo, em **todos** os nós (sensores, internos, temporizadores, motores, ativos ou não) |
| "Inativo" | aparece junto do nome só quando o nó está inativo. Nos ativos, mostra nome e valor |
| Área de hover | um círculo invisível maior em volta de cada nó, para acertar até nos internos de 3 pixels |
| Legenda | revista, curta, legível nos temas dia e noite |

Os números fixos de slots espalhados no `app.js` (26, 42, 45…) passam a vir
do motor (`EngineInfo`). Assim a próxima mudança de topologia não quebra a
interface em silêncio.

## 9. O que muda no código

| Arquivo | Mudança |
|---|---|
| `src/simulation/brain.rs` | `SLOT_COUNT` 57, `EXTERNAL_SLOT_COUNT` 27, `INTERNAL_START` 27, `MOTOR_START` 46; campo `age` em `SensorInput` |
| `src/simulation/world.rs` | calcula `Id` em `update_sensors` (sempre, como a energia: é uma divisão); versão nova do checkpoint e leitor da anterior |
| `src/simulation/genome.rs` | `Genome::baseline()`, `from_pre_age_sensor_bytes`, rodada de mutação dos fundadores |
| `interface/src/banco.rs` | formato 2, migração com cópia de segurança, Baseline embutido |
| `interface/src/engine.rs` | catálogo com Baseline e PF guia embutidos e os salvos; reinício e inserção pelo banco; fundadores com uma rodada de mutação; botão para criar no banco uma cópia de um indivíduo com a reprodução de última hora |
| `interface/ui/app.js`, `index.html` | População inicial, nomes e descrições do slot `Id`, layout da rede com 27 sensores, tooltip, Demografia |
| `src/app.rs` (V1) | só o rótulo do slot 57 |

Fica em módulo próprio sempre que der, para conflitar pouco com a V4.

## 10. Testes

**Automáticos (rodam no CI):**
- `Id` vale 0 ao nascer e chega perto de 1 na idade máxima.
- Migração: genoma de antes, carregado depois, tem as mesmas ligações efetivas
  (comparação ligação a ligação); nenhuma ligação migrada lê `Id`.
- Checkpoint antigo abre na versão nova, e ida e volta no save novo preserva
  tudo.
- Banco formato 1 vira formato 2, com a cópia `.v1.json` criada.
- Baseline:
  - com `Id < 0,89` não decide reproduzir;
  - com `Id ≥ 0,89` e energia acima do mínimo, se divide;
  - com energia abaixo do mínimo, não se divide;
  - com cheiro só em X, nada só em X; com cheiro nos dois eixos, nada na
    diagonal.
- O banco sempre lista o Baseline, mesmo sem arquivo.

**Interface (Playwright, no contêiner):** a lista de população inicial
mostra o Baseline primeiro; o tooltip aparece em nó interno inativo com
"inativo"; a rede desenha 27 sensores.

## 11. Medição

- `benchmark_simulacao` main × branch na mesma execução, como sempre.
- **O hash de estado vai mudar, e isso é esperado.** A topologia nova muda o
  que uma mutação pode produzir: um registro que antes apontava para um slot
  inexistente pode agora cair num slot válido. O que se compara é o custo do
  tick. O esperado é ficar igual, porque o sensor é uma divisão e só é
  calculado quando lido.
- Comportamento: os testes automáticos confirmam a decisão de reproduzir a
  partir de Id ≈ 0,89 e a navegação pelo PF.

## 12. Como testar no `.exe`

1. Abra a V5.0.0. Em Sessão, confira que a População inicial mostra o
   Baseline selecionado.
2. Reinicie a run. Em Células › Indivíduo, abra uma célula e passe o mouse no
   nó `Id` e num interno cinza: os nomes aparecem, o interno com "inativo".
3. Acompanhe uma célula velha (Tempo viva perto do limite): perto de 89% ela
   se divide, se tiver energia.
4. Na Demografia, as divisões aparecem em "decisão neural".
5. Abra uma run salva antes da V5 e um indivíduo antigo do banco: tudo
   carrega, e a rede mostra as mesmas ligações de antes.

## 13. Fora desta entrega

Behavior Spotter (V5.0.1), abas Física e Mapa, tema claro, sol, biomassa,
linhagens, pacote genoma.

## 14. Decisões para você aprovar

| # | Decisão | O que assumi |
|---|---|---|
| D1 | Mínimo de energia na reprodução de última hora | **continua valendo** |
| D2 | Fundadores com uma rodada de mutação | **sim** |
| D3 | Janela de urgência | **últimos ~11% da vida** (Id ≥ 0,89) |
| D4 | Série própria de "nascimentos de última hora" | **não**: conta como nascimento por decisão neural |
| D5 | Indivíduos do banco migrados, com cópia de segurança. "PF guia" volta como embutido com a amarração nova; para os seus (como o "Rede de espaçamento"), um botão cria uma cópia com a amarração | **sim** |
| D6 | Título da pasta da versão maior | **"V5 - Funções novas"** (genérico; cada entrega tem título próprio) |
