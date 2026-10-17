# Proposta: feromônios, assinatura genética e comunicação emergente

> Estado: **Implementado no código — aguardando publicação.** A primeira
> versão já altera a simulação principal; os custos e a escala poderão ser
> calibrados depois de observação.

## Intenção

Adicionar comunicação química sem codificar o significado de nenhum sinal.
Cada célula terá quatro feromônios que ela **lê e emite**: uma assinatura
genética e três sinais genéricos escolhidos pelo próprio genoma. A rede neural
decide quando reforçar cada emissão; a seleção decide se o sinal vira atração,
repulsão, agregação, alarme, exploração ou desaparece.

O sistema não terá um bônus por “cooperar”, por compartilhar feromônio ou por
estar perto de parentes. Emitir sempre tem custo energético. Qualquer padrão
coletivo precisa melhorar, indiretamente, a sobrevivência e a reprodução.

## Quatro feromônios por célula

| Slot químico | Natureza | Como é escolhido | Leitura e emissão |
|---|---|---|---|
| `PG` | assinatura genética | derivada continuamente do genoma completo | toda célula emite a cada tick; a rede pode reforçar |
| `P1` | sinal genérico | tipo escolhido no genoma | receptor `P1` e motor `E1` usam o mesmo tipo |
| `P2` | sinal genérico | tipo escolhido no genoma | receptor `P2` e motor `E2` usam o mesmo tipo |
| `P3` | sinal genérico | tipo escolhido no genoma | receptor `P3` e motor `E3` usam o mesmo tipo |

O ambiente inicia com oito substâncias genéricas anônimas: `C0` a `C7`.
Elas não possuem significado semântico no código. Um perfil possível seria:

```text
PG = assinatura genética desta célula
P1 = C2
P2 = C5
P3 = C7
```

Essa célula sente e emite `C2`, `C5` e `C7`. Outra linhagem pode ter outro
perfil. O decodificador do genoma garante que `P1`, `P2` e `P3` sejam três
tipos distintos; uma mutação que produziria duplicata é normalizada para o
próximo tipo livre.

## Genética química

Na primeira versão, nove bits da cauda regulatória do genoma escolhem `P1`,
`P2` e `P3`: três seletores de três bits. Eles sofrem a mesma mutação básica
do restante do genoma e são herdados pelo filho. Usar a cauda existente evita
alterar o tamanho físico do arquivo genético e preserva saves já gravados.

`PG` não recebe um identificador aleatório. Ele é uma projeção contínua dos
bits do genoma inteiro para uma pequena assinatura numérica. A mesma ideia já
usada na cor genética é adequada: uma mutação pequena muda pouco o odor;
genomas próximos têm assinaturas próximas; genomas distantes têm assinaturas
mais diferentes. Não se tenta colocar literalmente todos os bits do genoma no
campo ambiental.

Consequência: descendentes normalmente compartilham o mesmo “dialeto” químico
dos pais, mas mutações podem deslocar tanto a assinatura genética quanto os
três sinais genéricos disponíveis à linhagem.

## Sensores e motores neurais

O sensor de similaridade genética no contato (`Sm`) é removido. O sensor de
contato físico (`Ct`) permanece.

| Papel | Slot proposto | Saída |
|---|---|---:|
| Sensor | `PG`, `PGx`, `PGy` | intensidade externa compatível e gradientes X/Y de PG |
| Sensor | `P1`, `P1x`, `P1y` | ppm normalizado de `C(P1)` e direção de maior concentração |
| Sensor | `P2`, `P2x`, `P2y` | ppm normalizado de `C(P2)` e direção de maior concentração |
| Sensor | `P3`, `P3x`, `P3y` | ppm normalizado de `C(P3)` e direção de maior concentração |
| Motor | `EG` | reforço da emissão de `PG`, `0..1` |
| Motor | `E1` | emissão de `C(P1)`, `0..1` |
| Motor | `E2` | emissão de `C(P2)`, `0..1` |
| Motor | `E3` | emissão de `C(P3)`, `0..1` |
| Motor | `Do` | transferência de energia para a célula em contato, `0..1` |

O identificador do químico não entra como número na rede. Ele é um parâmetro
do receptor, codificado no genoma. Para cada químico, a rede recebe três
escalares simples: intensidade local, gradiente X e gradiente Y. X positivo
indica sinal mais forte à direita; Y positivo indica sinal mais forte acima.
Isso permite quimiotaxia sem entregar coordenadas do mapa ao organismo.

`Ct` e `PG` juntos substituem a informação que `Sm` fornecia:

- `Ct` alto e `PG` alto: contato com célula de assinatura semelhante;
- `Ct` alto e `PG` baixo: contato com célula de assinatura distante;
- sem contato, `PG` ainda pode sinalizar uma concentração externa próxima.

A emissão basal de `PG` é fixa e não pode ser desligada. O motor `EPG` apenas
a reforça. A regra pertence ao corpo, não a um registro mutável do genoma: toda
célula nascida — Clássica, Motor Mínima ou Val 1.0 — volta a emitir `PG`, mesmo
que não possua qualquer ligação neural de emissão.

## Campo químico do ambiente

Os feromônios serão campos escalares em uma malha química grossa, independente
do desenho do mapa. Não haverá uma entidade/partícula por emissão.

O campo é **instantâneo**: ele não guarda concentração, não difunde e não
decai entre ticks. No início de cada tick, a malha é zerada; as células que
estão emitindo naquele tick depositam massa em uma pequena vizinhança suave;
os receptores leem esse retrato; no tick seguinte ele deixa de existir e é
reconstruído somente pelas emissões daquele novo instante.

Assim, um feromônio genérico para de existir imediatamente quando o motor para
de emitir. `PG` é a exceção prevista: como toda célula o emite continuamente,
ele sempre reaparece no quadro de cada tick enquanto a célula estiver viva.

Cada depósito usa um núcleo espacial curto, centrado na célula. Ele cria uma
concentração maior perto da emissora e menor no limite do alcance químico, sem
precisar manter uma nuvem histórica. Paredes anulam a contribuição entre a
emissora e uma célula da malha bloqueada, seguindo a mesma noção de linha de
visão já aplicada aos sensores de alimento.

Os oito químicos genéricos usam oito camadas momentâneas da malha. A assinatura
genética usa uma camada de massa mais componentes compactos da assinatura média,
o que permite calcular afinidade sem criar uma camada para cada genoma existente.

O sensor `PG` exclui a contribuição basal da própria célula antes de normalizar
a leitura. Isso evita autoestimulação e permite que o sinal aumente de fato ao
encostar em outra célula.

### Ordem lógica dentro do tick

1. o quadro químico anterior é descartado;
2. a emissão basal de `PG` e as ativações disponíveis no início do tick
   constroem o quadro químico atual;
3. sensores `PG`, `P1`, `P2` e `P3` leem esse quadro;
4. a rede calcula as ativações que serão usadas na emissão do tick seguinte.

Há um atraso lógico de um tick entre decidir emitir e a substância ser
percebida. Não há memória ambiental de feromônio entre ticks.

## Escala por tamanho e custo energético

A massa emitida e seu custo usam a área física relativa da célula:

```text
área_relativa = (raio_atual / raio_base)²
              = 1 + 3 × energia / 2.000
```

Para cada feromônio genérico `Pi`:

```text
emissão(Pi) = emissão_base × ativação(Ei) × área_relativa
custo(Pi)   = custo_base   × ativação(Ei) × área_relativa
```

Para a assinatura genética:

```text
emissão(PG) = emissão_basal × (1 + reforço(EG)) × área_relativa
custo(PG)   = custo_basal   × (1 + reforço(EG)) × área_relativa
```

Uma célula com `2R` tem quatro vezes a área de uma célula de `1R`; portanto,
emite quatro vezes mais feromônio e também paga quatro vezes mais custo químico
para a mesma ativação. Uma célula de `4R` emite e paga dezesseis vezes mais.
Não existe limite de tamanho ou de emissão imposto por essa regra; o limite
real é a energia que a célula consegue sustentar.

## Mapa de calor químico e monitoramento

O mesmo quadro momentâneo usado pelos sensores alimenta uma sobreposição de
depuração no mapa. A simulação visual normal continua sem coloração química.
Quando o modo for ativado, o usuário escolhe uma camada:

| Seleção do mapa de calor | O que é mostrado |
|---|---|
| `PG` | concentração instantânea total da assinatura genética |
| `C0` a `C7` | concentração instantânea de cada químico genérico |
| `Todos` | soma das camadas genéricas, sem misturar identidade química |

A intensidade de cor representa massa emitida no tick atual: transparente para
zero, mais intensa para concentração maior. No tick seguinte, áreas sem emissão
somem completamente. A célula emissora continua visível por cima do mapa.

O overlay mostra a camada selecionada e o pico em ppm no tick atual. Massa
total, contagem de emissoras e custo químico agregado ficam reservados para a
próxima expansão da telemetria.

## Energia e ausência de significado programado

Cada motor de emissão gera uma quantidade proporcional à ativação neural, à
área física e ao custo energético correspondente. A taxa basal de `PG` também
tem custo de manutenção proporcional ao tamanho.

Nenhuma regra interpreta os químicos como “pedido de ajuda”, “aliado”,
“inimigo” ou “comida”. O sinal só é concentração. O significado vem de quais
conexões evoluírem entre os sensores químicos, neurônios internos e motores.

Feromônio por si só não cria cooperação. Ele cria um canal de comunicação.
Para uma cooperação estável surgir, uma estratégia coletiva precisa, em algum
cenário futuro, entregar vantagem física ou ecológica suficiente para pagar o
custo da emissão — por exemplo, navegar melhor em corrente forte, explorar
recursos agregados ou formar grupos de parentes.

## Alteração da topologia atual

Havia 37 slots neurais. `Sm` vira a intensidade de `PG`; entram os componentes
de gradiente e os demais sensores químicos, além de quatro motores de emissão.
A rede passa a ter **53 slots**: 23 sensores, 19 internos e 11 motores.

Essa alteração exige migrar genomas e checkpoints salvos, mantendo os registros
de conexão já existentes e deslocando índices posteriores quando necessário.
Runs antigas carregam com os novos traços em valores padrão neutros.

## Escopo da primeira entrega

Incluído:

- os quatro receptores e quatro emissões;
- intensidade e gradiente X/Y de cada receptor químico;
- perfil químico herdável;
- assinatura genética contínua;
- quadro químico instantâneo, alcance local e bloqueio por paredes;
- escala de emissão e custo pela área da célula;
- custo energético da emissão;
- mapa de calor químico selecionável e métricas de emissão por tick.

Fora desta entrega:

- mensagens com sintaxe predefinida;
- recompensa direta por cooperação;
- evolução de potência, alcance ou custo dos receptores;
- laboratório de seleção automática por comportamento cooperativo.

## Próxima decisão após validação

Após aprovar esta especificação, a implementação deve começar pela migração
segura do genoma e da topologia neural. Só então entram a malha química e os
novos controles visuais, para que saves antigos permaneçam legíveis.
