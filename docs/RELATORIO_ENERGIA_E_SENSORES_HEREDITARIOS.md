# Relatório de energia e proposta de sensores hereditários

**Estado: diagnóstico do código atual + proposta.** Nenhuma regra energética é
alterada por este documento.

## Resumo executivo

Hoje a energia é cobrada por manutenção do corpo, nado, processamento neural,
emissão de feromônios, fagocitose e perda operacional de doação. Os sensores
— alimento, corrente, energia própria, contato, química e força — têm alcance
fixo onde aplicável e **não têm custo energético próprio**.

Isso abre uma melhoria evolutiva coerente: o alcance do sensor de alimento
pode ser uma característica hereditária e custar energia continuamente. Uma
célula que enxerga mais longe terá vantagem informacional, mas precisará pagar
por uma área de percepção maior. A recomendação é introduzir primeiro só o
sensor de alimento; os sensores químicos têm um campo e alcance próprios e
merecem uma decisão separada.

## Fluxo energético atual por tick

Na etapa de metabolismo de `World::tick()`, a fórmula normal é:

```text
perda metabólica =
  ((basal + nado) × escala_do_corpo
   + cérebro
   + emissão_de_feromônios)
  × multiplicador_de_envelhecimento
```

Os custos de fagocitose e de doação ocorrem antes, na fase de interações. A
energia do alimento é recebida depois do metabolismo. Portanto, uma célula
precisa sobreviver aos seus gastos antes de se beneficiar de comida encontrada
naquele tick.

### O que efetivamente tira energia

| Processo | Quando ocorre | Regra atual | Relação com tamanho | Relação com idade |
|---|---|---|---|---|
| Manutenção basal | todo tick | `0,03 × escala do corpo` | escala corporal | multiplicada pela senescência |
| Nado | todo tick, conforme os quatro motores direcionais | `0,20 × (esq² + dir² + cima² + baixo²) × escala do corpo` | escala corporal | multiplicada pela senescência |
| Cérebro | todo tick | `conexões ativas × custo mental` | nenhuma | multiplicada pela senescência |
| PG basal e emissão química | todo tick; PG basal sempre, canais extras conforme motores | `(0,0004 × (1 + PG) + 0,001 × (P1 + P2 + P3)) × área relativa` | área relativa direta | multiplicada pela senescência |
| Fagocitose | há um alvo no alcance de interação e o motor está ativo | `0,04 × ativação` | nenhuma | multiplicada pela senescência |
| Doação: energia enviada | há receptor em contato, reserva e espaço no receptor | até `4,0 × ativação` sai do doador e vai ao receptor | nenhuma | a transferência não é multiplicada |
| Doação: perda operacional | junto da transferência | `2% × energia transferida` | nenhuma | multiplicada pela senescência |

Detalhes relevantes:

- O controle **Custo mental** da interface define o custo por conexão por
  tick; seu padrão é `0,0025`. Ele não depende do tamanho corporal.
- O nado cobra pelos comandos direcionais, não pelo deslocamento efetivo. Por
  exemplo, esquerda e direita ativados ao mesmo tempo se cancelam no movimento,
  mas ainda geram custo.
- Uma tentativa de fagocitose contra célula maior ainda paga o custo de ataque
  se havia alvo próximo; ela apenas não puxa nem consome.
- A doação separa transferência de gasto: a energia transferida não some, mas
  a perda operacional é dissipada. A reserva de doação atual é `60`.
- A senescência não é uma taxa independente. Ela aumenta os custos aplicáveis
  por meio de `1 / eficiência`; perto do limite de idade, o custo cresce muito.

### Processos que hoje não cobram energia

| Processo | Efeito atual | Observação |
|---|---|---|
| Sensoriamento de alimento | quatro cones, alcance fixo de `240` | custo energético zero |
| Sensoriamento químico | intensidade e gradientes de PG/P1/P2/P3 | custo energético zero; o alcance do campo é `500` |
| Sensores de corrente, energia, contato e força | leitura do estado físico | custo energético zero |
| Corrente ambiental | desloca a célula | deslocamento passivo, sem custo |
| Anti-cramming, colisões com parede e limites do mundo | corrigem posição/força | sem custo |
| Comer fitoplâncton | soma a energia configurada do alimento | sem custo de ingestão; excesso acima do limite é perdido |
| Receber doação | recebe a transferência | sem custo para o receptor |
| Reprodução | divide a energia do pai pela metade e dá a outra metade ao filho | transferência de energia, não dissipação direta |
| Consumir outra célula | recebe a energia positiva da presa | sem custo além da tentativa de fagocitose; excesso acima do limite é perdido |

## Tamanho corporal: o que já escala e o que não escala

O tamanho não é um gene hoje: ele cresce com a energia acumulada. A área
relativa do corpo é `1 + 3 × energia / 2.000`; o raio é a raiz quadrada dessa
área. Assim, em `2.000` de energia, o corpo tem 4× a área e 2× o raio base.

O custo basal e o de nado não sobem 4× nesse ponto: usam uma curva deliberadamente
mais suave e chegam a **1,40×** com raio dobrado. Já a emissão de feromônios
acompanha a área completa e chega a **4×**. Cérebro, fagocitose e operação de
doação não recebem fator de tamanho.

Isso significa que, para uma célula grande, a emissão química é hoje o custo
que mais cresce geometricamente; o cérebro e os sensores não ficam mais caros.
Em compensação, o raio maior aumenta a área que pode conter alimento e ajuda na
vantagem geométrica da fagocitose. O limite obrigatório de energia padrão da
interface é `5.000`, então células grandes são possíveis enquanto a rede não
decidir se reproduzir antes desse teto.

## Proposta: gene de alcance do sensor de alimento

### Regra biológica proposta

Cada célula teria um alelo de alcance visual/quimiorreceptor de alimento. O
valor padrão continua equivalente ao alcance atual de `240`; mutações podem
encurtar ou ampliar esse alcance. O custo é pago em todo tick, haja ou não
alimento no cone, pois manter o órgão sensorial ativo é o que consome energia.

Fórmula inicial recomendada:

```text
custo_sensor = CUSTO_SENSOR_BASE × (alcance / 240)²
```

O expoente 2 cobra pela área observada: dobrar o alcance cobre aproximadamente
quatro vezes mais ambiente. A recomendação é **não** multiplicar este custo
pela área corporal, pois ele já representa capacidade sensorial e o corpo já
tem manutenção/nado/emissão escalados separadamente. Como os outros custos
metabólicos, ele entra antes do multiplicador de envelhecimento e, portanto,
envelhece junto com a célula.

O valor de `CUSTO_SENSOR_BASE` não deve ser escolhido no escuro: a primeira
implementação deve medi-lo contra a renda média de alimento e rodar campanhas
com a mesma seed. Um bom experimento inicial testa valores que façam o alcance
padrão ser perceptível, mas menor que o custo basal de `0,03` por tick.

### Herança e assinatura genética

Este traço deve ficar dentro de `Genome`, não num gene fisiológico separado.
Isso atende à regra do projeto: uma mutação de capacidade sensorial também
altera similaridade genética, cor de linhagem e assinatura química de
parentesco. Hoje essas três leituras já projetam todos os bits de `Genome`.

Desenho sugerido:

1. Preservar os atuais 1.216 bits do genoma, inclusive os 9 bits finais hoje
   usados como região regulatória de canais químicos.
2. Acrescentar uma região de 5 bits para o alelo de alcance sensorial, com
   valor padrão que decodifica exatamente para `240`.
3. Fazer a mutação padrão de `Genome::mutated_from()` atingir essa região; não
   criar um mutador paralelo.
4. Fixar explicitamente o deslocamento dos 9 bits de canais químicos, em vez
   de continuar calculando-os como "os nove últimos", para que ampliar o
   genoma não mude silenciosamente os canais existentes.
5. Migrar checkpoints antigos preenchendo o novo alelo com o valor-padrão.
   Checkpoints novos passam a guardar o `Genome` ampliado e recebem uma versão
   de formato nova.

Cinco bits fornecem 32 degraus. Uma curva logarítmica centrada em `240` é uma
boa candidata, pois torna aumentos e reduções proporcionais comparáveis. O
intervalo exato (por exemplo, aproximadamente `120` a `460`) deve ser decidido
junto com o custo e validado por campanha, não definido pela interface.

### Alteração técnica necessária

1. Criar em `Genome` um decodificador público, por exemplo
   `food_sensor_range() -> f32`, e uma constante para o alcance-base.
2. Trocar o `SENSOR_RANGE` único nas funções de grade, busca e normalização
   pelo alcance da célula consultada. A grade espacial deve usar células do
   maior alcance possível, e a busca deve visitar todos os quadrados que cabem
   no alcance individual; a busca atual assume exatamente 3×3 quadrados porque
   todos usam `240`.
3. Adicionar `sensor_energy_cost(organism)` à fase de metabolismo e incluí-lo
   no futuro livro-caixa de energia abaixo.
4. Mostrar o alcance individual no painel da célula selecionada e desenhar os
   cones usando o valor da própria célula, não uma constante global.
5. Atualizar serialização, leitores de versões antigas, hashes/reprodução de
   benchmarks e testes de determinismo.

O primeiro recorte deve afetar somente os quatro sensores de alimento. O
alcance do campo de feromônios e seus sensores precisa ser escolhido depois:
alterá-lo envolve a malha química, custo de emissão e a própria comunicação,
não apenas a visão de alimento.

## Instrumentação recomendada antes de recalibrar custos

Antes de mudar números, criar um **livro-caixa de energia** por tick e por
amostra demográfica. Ele deve distinguir gasto metabólico de transferências e
de perdas por teto:

```text
saídas: basal, nado, cérebro, sensor, feromônios, fagocitose,
         perda operacional de doação, descarte por teto
transferências: doação enviada/recebida, divisão pai→filho,
                presa→predador
entradas externas: alimento
```

Na Demografia, mostrar totais por tick, médias por célula viva e proporção de
cada categoria. Os marcos de mudança de parâmetros propostos em
`PROPOSTA_TELEMETRIA_DEMOGRAFICA.md` devem aparecer sobre essas séries. Assim,
uma alteração de custo pode ser relacionada ao momento exato e não apenas à
população final.

## Ordem de implementação sugerida

1. Implementar o livro-caixa e validar que o balanço de energia fecha em uma
   run controlada.
2. Publicar a visualização demográfica dos gastos, ainda sem mudar regras.
3. Acrescentar o alelo de alcance com valor-padrão de `240` e custo-base zero;
   verificar que checkpoints migrados mantêm o mesmo comportamento.
4. Ligar o custo-base em campanhas reproduzíveis, testar várias sementes e
   comparar sobrevivência, tamanho corporal, alimento capturado e reprodução.
5. Só então escolher a curva e o valor de produção, e avaliar um segundo gene
   para percepção química.

## Fontes no código atual

- `src/simulation/world.rs`: constantes, ordem do tick, metabolismo,
  fagocitose, doação, alimentação, reprodução e curva corporal.
- `src/simulation/brain.rs`: custo de nado e custo por conexão neural.
- `src/simulation/pheromone.rs`: custo basal/ativo de emissão química.
- `src/config.rs`: alcance fixo atual de alimento, `SENSOR_RANGE = 240`.
