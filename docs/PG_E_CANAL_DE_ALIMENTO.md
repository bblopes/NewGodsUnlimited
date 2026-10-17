# PG, visão e o feromônio ambiental de alimento

Este documento registra a implementação a partir da versão experimental de
visão + PF. Ele separa três conceitos que antes apareciam próximos na
simulação, mas têm papéis biológicos diferentes:

- **PG** é identidade química de parentesco;
- **visão** informa que há alguma entidade no cone visual;
- **PF** é um odor ambiental universal produzido pelo fitoplâncton e indica
  alimento.

## PG: assinatura genética atual

`Genome::pheromone_signature()` produz um vetor bidimensional normalizado.
Para cada um dos 1.256 bits do `Genome`, o algoritmo soma ou subtrai uma
contribuição determinística em dois eixos, cada um com um sal diferente. Uma
mutação pequena altera uma parcela pequena da soma, portanto costuma deslocar
a assinatura pouco. A mesma família de projeções também alimenta cor e outras
leituras genéticas.

O campo químico mantém, por célula de grade, três acumuladores:

1. massa total de PG;
2. soma ponderada da componente X das assinaturas emissoras;
3. soma ponderada da componente Y das assinaturas emissoras.

Toda célula emite PG basal (`1.0 ppm`); o motor `emit_pg` o reforça. A
quantidade emitida cresce com `raio³`, enquanto o custo energético cresce com
`raio²`. O receptor subtrai sua própria massa basal da leitura local, compara
sua assinatura com a direção média das assinaturas presentes e obtém a
intensidade `PG`. Já `PGx` e `PGy` são o gradiente da massa total de PG: eles
apontam para maior concentração química, não exclusivamente para parentes.

Há duas consequências importantes desta definição:

- `LongevityGene` fica fora de `Genome`; logo, longevidade **não** altera PG,
  cor nem similaridade genética atualmente. Isto continua uma decisão a rever
  se a regra passar a ser “todo alelo entra na assinatura”.
- O campo é uma fotografia instantânea. Em cada tick ele é zerado e
  reconstruído; não há difusão persistente, advecção ou memória química.
  Como a reconstrução ocorre antes do cérebro, a emissão escolhida no tick
  anterior é o que os sensores leem agora.

O alcance químico é 500 unidades, aplicado por uma gaussiana sobre a grade
128×72. Paredes bloqueiam a linha entre emissão e amostra.

## Visão

Os quatro slots que antes eram descritos como alimento agora são visão por
cone: `V<`, `V>`, `V^` e `Vv`. Eles percorrem o mesmo alcance hereditário que
antes era chamado de alcance de alimento (incluindo o teto suave de 600), mas
emitem sinal positivo para a entidade visível mais próxima — fitoplâncton ou
outra célula. A visão não classifica o alvo como comida; paredes continuam
ocluindo a linha de visão.

Esta escolha deixa a iluminação futura simples: luz pode reduzir ou zerar a
visão sem prejudicar o canal químico que orienta a busca de alimento.

## PF: feromônio ambiental de fitoplâncton

PF é uma camada química separada de PG e P1/P2/P3. Não usa motor celular,
não custa energia à célula receptora e não depende de genes. Cada partícula de
fitoplâncton contribui passivamente para o mapa; a célula recebe:

- `PF`: intensidade local;
- `PFx`: sinal positivo quando a concentração é maior à direita;
- `PFy`: sinal positivo quando a concentração é maior acima no mundo visual.

Não reutilizamos PG porque PG significa parentesco. Também não reutilizamos
P1/P2/P3 porque cada célula escolhe geneticamente os próprios canais; isso
formaria dialetos e não um sinal alimentar universal.

### Custo de cálculo

O PF não espalha uma gaussiana por partícula. O algoritmo conta primeiro o
fitoplâncton na malha química 128×72 (`O(n)` no total de partículas). Depois
espalha uma única fonte por célula de grade ocupada, com a mesma gaussiana,
alcance e oclusão por paredes do campo químico. Assim, mesmo com muitos
fitoplânctons na mesma região, eles compartilham o custo da deposição; o teto
é 9.216 fontes de grade, não `n × amostras-do-raio`.

PF também é instantâneo nesta primeira versão. Persistência, difusão gradual e
transporte por corrente podem ser introduzidos mais tarde apenas nesta camada,
sem alterar PG ou a rede neural.

## Fundador experimental `PF guia`

A configuração neural inicial **PF guia** cria um indivíduo de exercício com
quatro internos e motores direcionais ativos. `PFx` alimenta esquerda/direita
e `PFy` alimenta cima/baixo. Os internos calculam magnitudes de eixo e fazem
inibição cruzada: quando X é mais forte, só o movimento horizontal sobrevive;
quando Y é mais forte, só o vertical. Empate exato resulta em repouso, em vez
de uma diagonal arbitrária.

O fundador é propositalmente simples: ele serve para validar a leitura do
gradiente e a decisão de eixo. Seus descendentes continuam sujeitos à mutação
normal do genoma.

## Compatibilidade de saves

A topologia passa de 53 para 56 slots. `PF`, `PFx` e `PFy` ocupam os slots
23–25; internos e motores antigos são deslocados em +3 ao carregar uma run
V18 ou anterior. Os quatro antigos sensores de comida preservam seus fios,
mas passam a ler visão — uma mudança deliberada de semântica. A primeira
leitura após carregar recalcula PF a partir do fitoplâncton salvo.
