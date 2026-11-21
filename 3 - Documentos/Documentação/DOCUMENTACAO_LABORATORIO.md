# Laboratório de Experimentos — especificação de encerramento combinável

## Objetivo

Uma campanha não deve depender de uma única meta de encerramento. Ela deve aceitar várias condições, combinadas de forma explícita, para responder perguntas diferentes sobre evolução, sobrevivência e estabilidade.

O modelo separa duas categorias:

- **metas experimentais**: dizem que o fenômeno que se queria observar foi alcançado;
- **limites de segurança**: interrompem uma execução que ficou inviável, longa demais ou sem organismos vivos.

O encerramento total será:

```text
(metas experimentais alcançadas) OU (algum limite de segurança atingido)
```

Assim, a campanha pode encerrar por sucesso ou por uma razão terminal bem identificada.

## Construtor de condições

A configuração terá dois grupos.

### 1. Metas experimentais

Cada meta pode ser ligada ou desligada. Entre as metas ligadas, o usuário escolhe uma lógica:

- **todas**: cada meta precisa ser satisfeita (`E` / `AND`);
- **qualquer uma**: basta uma meta ser satisfeita (`OU` / `OR`).

Metas iniciais:

| Meta | Regra | Uso principal |
|---|---|---|
| Geração máxima | `maior geração viva ou já nascida >= alvo` | Ver se evolução/reprodução encadeada aconteceu. |
| Nascimentos totais | `nascimentos acumulados >= alvo` | Medir capacidade reprodutiva. |
| População-alvo | `população viva >= alvo` | Opcional; útil para testes de explosão populacional, mas não será padrão. |
| Ticks mínimos | `ticks executados >= alvo` | Garantir uma janela mínima de observação antes de declarar sucesso. |

**Decisão atual:** população-alvo continua disponível, mas fica desligada por padrão. Ela não é uma boa medida geral de evolução: uma população grande pode ser apenas um ambiente permissivo, não uma estratégia melhor.

### 2. Limites de segurança

Essas condições sempre usam lógica `OU`: qualquer uma encerra a execução imediatamente.

| Limite | Regra | Motivo registrado |
|---|---|---|
| Extinção | `população viva = 0` | `Extinção` |
| Teto de ticks | `ticks >= máximo` | `Teto de segurança` |
| População máxima | `população viva >= limite máximo` | `Proteção contra explosão populacional` |

O limite de população máxima não é uma meta. Ele existe apenas para proteger memória, tempo de CPU/GPU e tamanho de snapshots. Pode ficar invisível no uso básico, com um valor técnico alto definido pelo aplicativo.

## Exemplos práticos

### A. “Quero descobrir se uma linhagem evolui”

```text
Metas: geração máxima >= 20
Lógica das metas: todas

Segurança: extinção OU teto de 500.000 ticks
```

Encerra por sucesso quando surgir a geração 20. Se todos morrerem antes, o resultado é extinção.

### B. “Quero uma evolução que seja também reprodutivamente estável”

```text
Metas: geração máxima >= 20 E nascimentos totais >= 1.000
Lógica das metas: todas

Segurança: extinção OU teto de 1.000.000 ticks
```

Chegar à geração 20 sozinho não basta: a linhagem precisa ter sustentado reprodução suficiente.

### C. “Quero gerar candidatos rápidos para análise”

```text
Metas: geração máxima >= 10 OU nascimentos totais >= 500
Lógica das metas: qualquer uma

Segurança: extinção OU teto de 100.000 ticks
```

É uma triagem: aceita qualquer sinal inicial de dinâmica evolutiva.

### D. “Quero observar apenas a estabilidade após tempo suficiente”

```text
Metas: ticks mínimos >= 200.000
Lógica das metas: todas

Segurança: extinção OU população máxima técnica
```

Aqui o experimento acaba após uma janela temporal fixa; geração e nascimento viram métricas de relatório, não critérios.

## Semântica da geração

Para evitar perder um evento relevante, a condição de geração será avaliada pelo maior valor **já alcançado** na campanha, não apenas pela maior geração viva no tick atual.

Exemplo: uma célula de geração 30 nasce e morre dois ticks depois. A meta de geração 30 foi atingida e a campanha termina como sucesso.

## Registro do resultado

Cada mundo salvo registrará:

- configuração completa das metas e limites;
- condição exata que encerrou o mundo;
- tick de encerramento;
- valores observados no encerramento: população, maior geração alcançada, nascimentos, energia média;
- se encerrou por **meta alcançada** ou **limite de segurança**.

Isso permite comparar campanhas sem confundir uma execução que prosperou com outra que apenas atingiu o teto de ticks.

## Interface futura

Na tela de campanha, a seção ficará assim:

```text
ENCERRAMENTO
Metas experimentais      [todas v]
  [x] geração máxima     >= [20]
  [x] nascimentos        >= [1.000]
  [ ] população-alvo     >= [----]
  [ ] ticks mínimos      >= [----]

LIMITES DE SEGURANÇA
  [x] extinção
  [x] teto de ticks      [500.000]
  [x] população máxima   [100.000]   (proteção técnica)
```

O relatório mostrará, por exemplo:

```text
Mundo 04 — meta alcançada
Encerramento: geração máxima >= 20 E nascimentos >= 1.000
Tick: 184.230
População final: 614
Maior geração alcançada: 20
```

## Modos de exploração massiva

O Laboratório terá modos explícitos. Eles evitam misturar duas perguntas científicas diferentes em uma mesma campanha.

### 1. Varredura de seeds aleatórias

**Pergunta:** este mesmo ambiente produz resultados semelhantes quando a condição inicial muda?

```text
Ambiente: fixo
Seeds: aleatórias, uma por mundo
Número de mundos: N
```

Cada mundo recebe uma seed nova gerada pelo aplicativo. Essa seed define a condição inicial completa já existente no motor: genomas iniciais, posições iniciais, posições de alimento e topologia do campo de corrente dependente da seed.

O usuário não digita uma seed. Ela é gerada, registrada no resultado e pode ser reutilizada depois.

**Uso:** repetir o mesmo experimento em 100 ou 1.000 condições iniciais diferentes e verificar se uma estratégia aparece com frequência ou foi apenas sorte.

### 2. Estresse ambiental de uma seed fixa

**Pergunta:** uma condição inicial promissora continua funcionando quando o meio muda?

```text
Seed: uma seed fixa, escolhida de um mundo salvo
Ambiente: varia dentro de faixas definidas
Número de variações: N
```

O ponto de partida é sempre idêntico: mesmos genomas iniciais, posições e topologia-base da corrente. O que muda são apenas parâmetros ambientais, por exemplo:

| Variável ambiental | Pode variar? |
|---|---|
| Emissão de fitoplâncton | Sim |
| Vida do fitoplâncton | Sim |
| Energia por alimento | Sim |
| Intensidade da corrente | Sim |

Parâmetros que descrevem a célula, como meta de reprodução, ficam fixos neste modo. Assim a campanha mede resposta ao ambiente, não uma mistura de ambiente e fisiologia.

Cada variação registra os valores sorteados e o resultado. Um bom mundo pode ser reproduzido exatamente depois.

### 3. Matriz seed × ambiente — posterior

**Pergunta:** um comportamento é robusto tanto ao acaso inicial quanto ao ambiente?

```text
20 seeds aleatórias × 15 ambientes sorteados = 300 mundos
```

Esse é o modo mais caro e mais informativo. Ele fica para depois dos dois modos simples, porque exige relatórios agrupados por seed e por ambiente.

## Tela inicial de campanhas massivas

A ação **Gerar nova simulação** abre primeiro uma escolha de objetivo, antes de mostrar sliders.

```text
COMO VOCÊ QUER EXPLORAR?

[ Varredura de seeds aleatórias ]
  Mesmo ambiente; múltiplos começos aleatórios.

[ Estressar uma seed por ambiente ]
  Uma condição inicial; múltiplos ambientes.

[ Matriz seed × ambiente ]                 (posterior)
  Cruzar várias seeds com várias condições ambientais.
```

### Fluxo: varredura de seeds

```text
AMBIENTE FIXO
  emissão, vida fito, energia, corrente

REPETIÇÕES
  quantidade de mundos aleatórios: [100]

ENCERRAMENTO
  metas combináveis + limites de segurança

Previsão: 100 mundos independentes
```

Não há campo de seed nessa tela. Ao final, a tabela de resultados apresenta a seed de cada mundo.

### Fluxo: estresse ambiental

```text
SEED DE ORIGEM
  [Escolher mundo salvo]
  Mundo: Campanha 18 / Mundo 04 / seed 847291530

VARIÁVEIS DO AMBIENTE
  [x] emissão          mínimo [0,20] máximo [3,00]
  [x] vida fito        mínimo [300]  máximo [4.000]
  [ ] energia alimento  fixa    [50]
  [x] corrente          mínimo [20%] máximo [100%]

VARIAÇÕES
  quantidade de ambientes sorteados: [50]

ENCERRAMENTO
  metas combináveis + limites de segurança
```

O aplicativo gera 50 configurações ambientais, todas usando a mesma seed de origem. Cada linha do relatório mostra a seed fixa e os valores ambientais efetivos.

## Dados que cada mundo deve guardar

Independentemente do modo, cada mundo precisa registrar:

- modo da campanha;
- seed efetiva;
- parâmetros ambientais efetivos, inclusive os sorteados;
- condição de encerramento e tick final;
- métricas finais e snapshots para reprodução.

Isso é o que permite selecionar um mundo vencedor e criar, a partir dele, uma campanha de estresse ambiental ou uma reprodução exata.

## Próxima implementação

1. Remover a seed-base como campo normal da campanha.
2. Implementar varredura de seeds aleatórias, com ambiente fixo.
3. Implementar estresse ambiental de uma seed escolhida em um mundo salvo.
4. Aplicar o construtor de condições combináveis aos dois modos.
5. Construir a matriz seed × ambiente após validar os relatórios simples.
