# Proposta: osmose forçada, tamanho e engolfamento

## Intenção

Não haverá um ataque instantâneo que aplica dano. O motor da célula será **Osmose**: quando ativado, ele tenta puxar uma célula próxima para o próprio núcleo. A outra célula pode gastar energia para nadar na direção contrária e escapar.

Uma célula só é engolida quando o núcleo dela entra suficientemente dentro da área da outra **e** existe vantagem real de tamanho. Assim, a física decide a aproximação; a rede decide insistir, fugir ou desistir.

> Estado atual: o **raio dinâmico**, o custo físico maior e o **anti-cramming por raio real** já foram implementados no ecossistema. Osmose, engolfamento e os novos sensores ainda são proposta.

## Raio físico variável

O raio deixa de ser apenas visual. Ele passa a representar o espaço físico ocupado pela célula e cresce com a energia acumulada.

Regra publicada:

```text
raio = raio_base × √(1 + 3 × energia_atual / 2.000)
```

Com `raio_base = 14`, uma célula com energia `0` tem raio `14`; com energia `2.000`, tem raio `28` — exatamente o dobro; com `10.000`, chega a `56`. Não existe teto físico de raio: energia adicional continua virando área, enquanto o raio cresce pela raiz quadrada dessa área.

Consequências necessárias:

- manutenção e nado usam uma curva sublinear de raio: `1R = 1,00×` e
  `2R = 1,40×` de custo; acima disso o custo segue crescendo, sem salto;
- a colisão e o anti-cramming passam a usar o raio real de cada par;
- uma célula grande ganha capacidade física, mas paga para permanecer grande.

## Anti-cramming compatível com tamanho

Hoje o afastamento entre células pode usar um raio-base fixo. Isso deixa de servir: uma célula grande poderia atravessar outra antes da regra perceber.

Para cada par `A` e `B`:

```text
distância_mínima = raio_A + raio_B
```

Se a distância entre os núcleos ficar abaixo desse valor, o sistema aplica uma força de separação proporcional à sobreposição. A força pode continuar configurável no slider anti-cramming.

O cálculo continua eficiente com grade espacial: apenas células no mesmo quadrado da grade, ou nos quadrados vizinhos, são comparadas. O tamanho da célula só altera a condição do par, não transforma a simulação em comparação de todas contra todas.

## Novos sentidos

Para a célula ter alguma base para decidir interações, entram dois sensores de contato e quatro sensores direcionais de outras células.

| Sensor | Saída | Função |
|---|---:|---|
| Contato celular | `0` ou `1` | Indica que existe outra célula tocando a área física dela. |
| Similaridade no contato | `0..1` | Similaridade genética da célula tocando. Só tem significado se o sensor de contato for `1`. |
| Célula esquerda/direita/cima/baixo | `0..1` | Presença da célula mais próxima dentro do cone correspondente. |

Os quatro sensores direcionais seguem a mesma ideia dos sensores de alimento: quatro cones de 90°, alcance configurável e um único valor simples por cone. Eles não entregam coordenadas, ângulos nem uma lista de vizinhos.

### Bloco 2 detalhado — contato e similaridade, sem osmose

Este é o próximo bloco a ser implementado, mas permanece **somente especificado** até a validação visual do tamanho físico.

**Contato celular** vale `1` quando, no início do tick, existe ao menos uma célula cujo centro esteja nesta condição:

```text
distância entre núcleos ≤ raio_próprio + raio_da_vizinha
```

Não é preciso calcular uma nova busca global: a grade espacial já usada pelo anti-cramming encontra apenas vizinhos locais.

**Similaridade no contato** é a similaridade de Hamming já usada pela telemetria genética: a fração de bits iguais entre os dois genomas. Ela varia de `0` a `1`.

- nenhuma célula em contato: `contato = 0`, `similaridade = 0`;
- uma ou mais células em contato: `contato = 1`;
- havendo várias, a referência é a de núcleo mais próximo; empate é resolvido pelo menor identificador da célula, para a simulação ser reprodutível;
- a rede sempre recebe os dois valores. Assim ela diferencia “não há ninguém” de “há alguém geneticamente muito diferente”.

O anti-cramming não será desligado por isso. Se o slider estiver em `1R`, células podem tocar e o sensor pode ativar. Se o usuário configurar mais espaço que `1R`, ele mesmo cria uma distância de segurança e contatos ficam raros ou inexistentes — comportamento desejado e visível.

Esse bloco **não** adiciona perseguição, osmose, dano, morte, transferência de energia ou leitura da direção da vizinha. Ele entrega somente percepção local neutra.

<h2 style="color: #d32f2f;">Atualização — percepção de forças externas</h2>

Além de perceber a corrente do ambiente, a célula precisa perceber a **força física líquida** que está sendo aplicada sobre ela naquele tick. Para ela, não importa inicialmente se essa força veio de corrente, de anti-cramming, de osmose ou de uma futura parede: importa que seu corpo está sendo deslocado em certa direção.

Entram dois sensores contínuos:

| Sensor | Faixa | Significado |
|---|---:|---|
| Força externa X | `-1..1` | Esquerda é negativa; direita é positiva. |
| Força externa Y | `-1..1` | Cima é negativa; baixo é positivo. |

Eles recebem a soma normalizada das forças **que não foram escolhidas pelo motor de movimento da própria célula**. A corrente continua sendo um sentido separado: ela informa a tendência do meio. Estes novos sensores informam o efeito físico resultante sobre a célula.

Exemplo: se uma célula está sendo sugada para a direita por osmose, `força externa X` fica positivo. Se ela ativa o motor de nadar para a esquerda, a rede pode evoluir uma resposta de fuga sem receber uma etiqueta artificial como “estou sendo atacada”. O mesmo circuito também pode reagir quando estiver sendo repelida para fora de uma colisão.

Isso é deliberadamente mais geral do que um sensor de “ataque inimigo”: um neurônio interno pode aprender que uma força inesperada para perto de outra célula, combinada com contato e baixa similaridade, pede fuga; ou pode aprender a não resistir quando a situação for favorável. A causa exata permanece oculta da rede.

### Contrato de implementação futura — força externa e Osmose

O cérebro decide antes de a física daquele tick terminar. Portanto, os sensores `força externa X/Y` não podem antecipar uma sucção que ainda não aconteceu. A regra será:

```text
força externa entregue no tick T = força física sofrida no tick T - 1
```

Ao final de cada tick, o motor físico soma por célula somente as forças que não vieram da própria decisão de nadar:

```text
força_externa = força_de_osmose + força_de_separação + futuras_forças_de_obstáculo
```

A corrente não entra nessa soma porque já possui os sensores próprios `Cx` e `Cy`. O vetor é normalizado por uma constante de calibração e limitado a `-1..1`, evitando que uma colisão extrema faça a rede receber valores sem escala.

O motor **Osmose** seguirá estas regras operacionais:

1. A célula só considera a célula em contato ou, mais adiante, a vizinha direcionalmente mais próxima dentro da margem de interação.
2. A força de sucção é aplicada continuamente ao centro da presa; não existe dano por tick.
3. A presa pode se opor pelo seu vetor de nado. A componente que aponta para longe reduz o deslocamento e encarece a osmose.
4. O anti-cramming continua ativo para pares comuns. Para um par elegível à osmose, ele só é temporariamente relaxado depois que a regra de contenção confirmar vantagem real de raio.
5. O engolfamento é a única causa de morte predatória e exige as três condições geométricas já descritas neste documento.

Assim, a célula não recebe um sinal semântico de “predador” ou “presa”. Ela aprende a associar contato, similaridade, deslocamento físico e custo energético às consequências de sobreviver ou ser engolfada.

## Motor neural: Osmose

Entra um novo neurônio motor chamado **Osmose**. Sua ativação `o`, de `0` a `1`, expressa a intensidade com que a célula tenta puxar um alvo próximo.

Uma célula somente pode tentar osmose se houver contato ou proximidade curta:

```text
distância entre núcleos ≤ raio_A + raio_B + margem_de_interação
```

A margem inicial pode ser `15%` do menor raio. Ela permite iniciar a sucção antes de haver sobreposição visual completa.

O vetor de força aponta do núcleo da presa para o núcleo do predador. A força não teleporta a presa: ela é aplicada aos poucos a cada tick.

```text
força_de_osmose = força_base × o × vantagem_de_tamanho

vantagem_de_tamanho = max(0, raio_predador / raio_presa - 1)
```

Um predador menor não tem sucção útil. Um pouco maior puxa pouco; uma diferença grande puxa mais, mas também sofre o custo de ser grande.

## Fuga e custo energético

A presa pode escapar usando seus quatro motores de movimento. Só conta a parte do nado na direção oposta ao predador.

```text
fuga = componente do movimento da presa para longe do predador
custo_osmose = custo_base × o × (1 + fuga_normalizada)
```

Logo:

- se a presa não reage, a osmose custa menos e tende a aproximá-la;
- se ela nada para fugir, ela gasta energia e o predador precisa gastar ainda mais para manter a sucção;
- a luta é uma disputa contínua de posicionamento e energia, não uma rolagem aleatória nem dano instantâneo.

O motor de osmose não precisa saber explicitamente qual é o mínimo ou o máximo de energia do outro organismo. Ele só recebe os sentidos disponíveis e evolui uma política sob as consequências físicas.

## Engolfamento

Engolfar exige três condições no mesmo tick:

```text
1. raio_predador ≥ 1,35 × raio_presa
2. distância_núcleos ≤ raio_predador - raio_presa
3. ativação do motor Osmose ≥ 0,60
```

A segunda regra é a contenção geométrica: o núcleo da presa só é considerado dentro do predador quando a circunferência dela cabe inteiramente no raio físico do predador. A proporção `1,35` cria uma folga para que não existam engolfamentos absurdos entre células quase iguais.

Em um contato causado por cramming, não vale a regra simples “quem tem mais energia ganha”. Primeiro aplicamos exatamente as três condições acima. Energia influencia indiretamente o resultado por aumentar o raio e por sustentar o custo da osmose.

Quando ocorre o engolfamento:

```text
energia_transferida = eficiência_de_assimilação × energia_da_presa
```

Sugestão inicial: eficiência de `75%`. O restante é perdido. A energia do predador respeita seu teto; excesso não vira energia gratuita. A presa é removida da simulação e a morte é registrada como `engolfada`.

## Ordem física de um tick

1. Atualizar sensores de alimento, corrente, energia e células.
2. Rodar o cérebro de cada célula.
3. Aplicar movimento voluntário, corrente e seus custos.
4. Montar pares locais pela grade espacial.
5. Aplicar forças de osmose e fuga aos pares elegíveis.
6. Avaliar engolfamentos, priorizando em conflito a maior razão de tamanho e, no empate, o menor identificador.
7. Aplicar anti-cramming por raio variável aos pares que não foram engolfados.
8. Processar alimento, reprodução, mortes, telemetria e renderização.

Essa sequência permite uma sobreposição temporária durante uma osmose válida, sem deixar duas células comuns se acumularem no mesmo ponto.

## Alteração do genoma

Hoje há 32 slots neurais. A proposta acrescenta:

- 6 sensores: contato, similaridade e quatro direções de células;
- 2 sensores: força externa X e força externa Y;
- 1 motor: Osmose.

Total planejado: **41 slots**. Os identificadores deixam de caber em 5 bits, então cada registro de conexão deve passar de 17 para 19 bits:

```text
origem: 6 bits
destino: 6 bits
peso: 6 bits
ativo: 1 bit
```

Mantendo 64 registros de conexão, o genoma passa de 136 para **152 bytes**. É uma migração explícita de formato: sessões antigas continuam lidas pela versão atual, mas a versão com osmose precisará sinalizar um novo formato no arquivo salvo.

Como os dois sensores de contato já elevam os slots de 32 para 34, a migração de 5 para 6 bits acontecerá junto com o bloco 2. O bloco 3 apenas reutiliza esse formato para chegar aos 41 slots.

## Implementação em blocos

1. ✅ Tornar o raio dependente da energia e trocar o anti-cramming para `raio_A + raio_B`.
2. ⏳ Adicionar contato e similaridade genética no contato, migrando o genoma para 34 slots; ainda sem predação.
3. Adicionar sensores direcionais de células e força externa, ampliando de 34 para 41 slots.
4. Adicionar o motor Osmose, força de puxar e custo contra a fuga.
5. Adicionar regra de engolfamento, transferência parcial de energia e telemetria.

Antes de cada bloco, calibraremos parâmetros para preservar o comportamento herbívoro atual quando o motor Osmose não estiver conectado.

## Telemetria necessária

- tentativas de osmose por tick;
- energia gasta em osmose e em fuga;
- engolfamentos e energia efetivamente assimilada;
- mortes por engolfamento;
- distribuição de raios e proporção de biomassa em células grandes;
- contatos e similaridade genética média dos pares em contato.
- intensidade média e picos de força externa em X e Y.

Esses dados permitem verificar se uma linhagem tornou-se predadora por estratégia real ou se apenas explora um erro de colisão.
