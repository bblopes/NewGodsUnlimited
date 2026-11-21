# Análise genética da população

## Objetivo

Responder, no estado atual da simulação, quais estruturas neurais estão se espalhando e quais grupos de genomas dominam a população. Esta análise não escreve comportamento: ela apenas lê os genomas das células vivas.

## O que o genoma realmente codifica

Cada célula possui 64 registros de 17 bits:

```text
[origem: 5 bits] [destino: 5 bits] [peso: 6 bits] [ativo: 1 bit]
```

Os 32 slots neurais são fixos em toda a população. Portanto, não existe um "neurônio que aparece" em algumas células e falta em outras. O que varia é se cada slot participa de uma conexão ativa, para onde aponta e com qual peso.

| Faixa | Slots | Significado |
|---|---:|---|
| Sensores | 0–7 | comida em quatro direções, corrente X/Y, energia e constante |
| Internos livres | 8–23 | dezesseis internos imediatos |
| Temporizadores | 24–26 | memória rápida, média e lenta |
| Motores | 27–31 | esquerda, direita, cima, baixo e reproduzir |

Registros ativos inválidos são ignorados pelo decodificador, como já ocorre no cérebro da simulação.

## Grupos genéticos

Nesta etapa, **grupo genético** não é uma árvore genealógica. É um agrupamento por similaridade de Hamming entre genomas vivos.

1. As células são processadas por ID, tornando o resultado reprodutível para o mesmo estado.
2. A primeira célula de um grupo torna-se sua representante.
3. Uma nova célula entra no grupo cuja representante é mais similar, se a similaridade for pelo menos **98,5%** — no máximo 16 bits diferentes em 1.088.
4. Se não atingir o limiar, abre um novo grupo.
5. Os grupos são ordenados por população.

O limiar separa ramos que acumularam mutações, mas mantém filhos próximos da representante no mesmo grupo. É uma heurística visual e estatística; uma linhagem genealógica real exigirá `parent_id` e `root_lineage_id` em cada organismo.

## KPIs disponíveis

### Resumo

| KPI | Cálculo | Leitura |
|---|---|---|
| População analisada | número de células vivas | tamanho da amostra |
| Grupos genéticos | quantidade de grupos pelo limiar de 98,5% | fragmentação atual |
| Grupo dominante | maior grupo / população | concentração evolutiva |
| Similaridade do grupo dominante | média representante–membro | coesão interna |
| Concordância do consenso | média do alelo majoritário em cada um dos 1.088 bits | convergência global; não representa uma célula real |

### Grupos

Para cada grupo mostrado no front:

| KPI | Cálculo |
|---|---|
| Cor | cor genética da célula representante |
| Células | membros absolutos e percentual da população |
| Similaridade interna | média de similaridade com a representante |
| Geração média | média da geração dos membros |
| Representante | ID da célula que pode ser aberta no acompanhamento unitário |

### Neurônios

Um slot é considerado **usado** por uma célula quando participa de pelo menos uma conexão ativa e legal naquele cérebro.

| KPI | Cálculo |
|---|---|
| Slot mais usado | percentual de cérebros com conexão incidente no slot |
| Origem mais usada | percentual de cérebros com ao menos uma conexão saindo do slot |
| Destino mais recrutado | percentual de cérebros com ao menos uma conexão chegando ao slot |
| Motor mais alimentado | destino motor presente no maior percentual de cérebros |

### Circuitos

Uma conexão é agrupada por topologia `origem → destino`; o peso é analisado separadamente. Isso evita que uma mutação pequena no peso faça um circuito estruturalmente idêntico parecer totalmente novo.

| KPI | Cálculo |
|---|---|
| Frequência | percentual de cérebros com ao menos uma conexão ativa naquele par |
| Peso médio | média dos pesos ativos do par |
| Sinal dominante | positivo quando a média é maior que zero; negativo quando menor |
| Conservação | presença do circuito entre grupos genéticos distintos |

Exemplo: `Corrente X → I3`, presente em 74% dos cérebros, peso médio `-0,63`, indica uma estrutura que tende a inibir o interno `I3` quando a corrente horizontal cresce.

## Controles do front

Na aba **Células** haverá dois modos:

| Modo | Função |
|---|---|
| Indivíduo | acompanhamento da célula escolhida: energia, idade, filhos, genoma e rede ao vivo |
| Genética | análise da população viva no snapshot atual |

O modo Genética terá três seletores:

| Seletor | Mostra | Ação disponível |
|---|---|---|
| Resumo | KPIs gerais, grupo dominante e circuitos principais | trocar para Grupos ou Circuitos |
| Grupos | lista dos maiores grupos genéticos | abrir a célula representante no modo Indivíduo |
| Circuitos | rankings de slots e pares origem → destino | alternar entre ranking de slots e de conexões |

## Custo e atualização

O relatório é recalculado apenas quando o modo Genética está aberto e o snapshot avançou pelo menos 250 ticks. A interface reutiliza o último relatório entre recalculações. A formação de grupos usa busca de candidatas por bandas do genoma e validação final por Hamming, evitando uma comparação quadrática integral da população.

## Limites desta versão

- Grupos por similaridade não são ancestralidade comprovada.
- O consenso de bits é uma estatística, não um genoma necessariamente existente.
- A análise é do estado presente; a evolução temporal de grupos entrará no pós-mortem da sessão.
