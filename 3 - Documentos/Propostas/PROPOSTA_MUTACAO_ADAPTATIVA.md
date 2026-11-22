# Proposta: coeficientes hereditários de mutação

> **Revisão pedida pelo Bruno em 22/11/2026 (a detalhar quando este item
> entrar no cronograma):** em vez de três genes para a rede inteira, os
> reguladores formam uma **matriz**: cada **grupo de ligações** tem o seu gene
> de topologia, o seu de peso e o seu de ativação. As taxas devem ser
> apresentadas também como **chance**, não só como multiplicador. Ver a seção
> "Versão em matriz" no fim deste documento.

## Decisão resumida

O filho continuará a herdar a fiação neural do pai, com *flips* de bits. A
mudança é que uma pequena região reguladora do genoma define a chance de
mutação de cada **tipo de informação** da fiação.

Não vamos começar com um coeficiente para cada uma das 64 conexões. Vamos usar
três coeficientes por célula:

| Gene regulador | Parte afetada | Efeito evolutivo |
|---|---|---|
| `topologia` | origem e destino das conexões | muda quem fala com quem |
| `peso` | peso sináptico | ajusta a intensidade das decisões |
| `ativação` | bit que liga/desliga uma conexão | cria ou remove caminhos existentes |

Isso preserva exatamente a ideia de “hotspots” de mutação, mas em uma escala
que a seleção natural consegue aproveitar. Um coeficiente por conexão criaria
dezenas de genes reguladores com efeito minúsculo e muito ruído.

## Formato proposto

Hoje (conferido em 22/11/2026) o genoma tem 1.251 bits úteis, guardados em
157 bytes:

```text
[64 × registro neural de 19 bits: origem 6 | destino 6 | peso 6 | ativo 1]  1.216 bits
[alcance da visão: 5 bits]
[atividade dos neurônios internos e motores: 30 bits]
```

Proposta: acrescentar um bloco regulador de 16 bits **no fim** do genoma, como
as regiões anteriores foram acrescentadas. Colocá-lo antes dos registros
deslocaria todos os índices e complicaria a migração.

```text
[mutação topologia: 4][mutação peso: 4][mutação ativação: 4][reserva: 4]
```

Sozinha, esta proposta leva o genoma de 1.251 para 1.267 bits. No plano da V5
ela vem num pacote com a longevidade dentro do genoma e os canais químicos em
região própria, chegando a 1.292 bits (162 bytes); ver
[V5_FUNCOES_NOVAS.md](../Backlog/V5_FUNCOES_NOVAS.md). Os quatro bits
reservados ficam intactos por enquanto: servem para futuras regras sem
precisar alterar o formato outra vez.

## Leitura dos coeficientes

Cada campo de quatro bits representa um nível de `0` a `15`. Ele vira um
multiplicador da taxa-base de mutação:

```text
nível 0  → 0,25× a taxa-base
nível 8  → 1,00× a taxa-base (padrão)
nível 15 → aproximadamente 3,36× a taxa-base
```

Com a taxa-base atual de `1/1.024` por bit, uma célula com nível padrão mantém
o comportamento de hoje. Uma linhagem pode evoluir para ser mais conservadora
ou mais exploratória, mas não consegue chegar a taxas absurdas que destruam o
genoma a cada filho.

Para cada nascimento:

```text
chance(bit de peso)      = taxa_base × multiplicador_peso
chance(bit de topologia) = taxa_base × multiplicador_topologia
chance(bit ativo)        = taxa_base × multiplicador_ativação
```

Os valores finais são limitados a uma faixa segura antes do sorteio.

## Como os próprios genes reguladores mudam

Este é o ponto importante da sua ideia: os 12 bits reguladores **não usam o
coeficiente que eles mesmos carregam**. Cada um deles recebe apenas a taxa-base
fixa.

```text
chance(bit regulador) = taxa_base fixa = 1 / 1.024
```

Assim existe uma mutação de segunda ordem — uma linhagem pode herdar uma taxa
maior ou menor — sem o ciclo perigoso “taxa alta aumenta a chance de a taxa
ficar ainda mais alta” em uma única geração. Essa separação é o equivalente
computacional de uma maquinaria de reparo do DNA que também evolui, porém mais
lentamente.

## Por que não um gene por conexão agora

Seria tecnicamente possível reservar, por exemplo, quatro bits para cada
registro neural. Mas isso adicionaria 256 bits reguladores e traria três
problemas:

1. cada gene passa a afetar pouquíssimos bits; a seleção recebe sinal fraco;
2. o genoma cresce, a comparação genética e os salvamentos ficam mais caros;
3. fica difícil interpretar por que uma linhagem venceu.

Os três grupos propostos já dão um resultado interessante: uma população pode
estabilizar a topologia e continuar refinando pesos; outra pode explorar novos
caminhos neurais, mas conservar pesos quando encontra uma arquitetura boa.

## Telemetria que entrará junto

Quando esta proposta for implementada, cada amostra demográfica deve guardar:

- média e desvio-padrão de cada um dos três níveis de mutação;
- taxa efetiva média de *flips* por nascimento;
- distribuição dos níveis por linhagem;
- correlação entre taxa de mutação, filhos produzidos e sobrevivência.

No painel genético, a leitura mais útil será: “a linhagem dominante está mais
conservadora ou mais exploratória que a população?”.

## Compatibilidade de sessões salvas

O leitor de sessões já diferencia versões de genoma. A implementação deverá:

1. criar uma nova versão de arquivo de sessão;
2. carregar genomas atuais com os três níveis iniciais em `8`;
3. escrever o novo cabeçalho apenas em novos salvamentos;
4. manter a reexecução determinística de sessões antigas.

## Estado atual do projeto — lembrete (conferido em 22/11/2026)

**Implementado no executável oficial (V3.0.4):**

- mundo 2D, corrente ambiental, luz solar binária e fitoplâncton;
- cérebro hereditário com 64 registros de conexão de 19 bits e **56 slots**
  (26 externos, 19 internos, 11 motores);
- visão genérica em quatro cones, corrente, energia, contato, feromônios
  (PG, P1–P3), força externa e quimiorrecepção de alimento (PF);
- fagocitose, doação de energia e emissão de feromônios como motores;
- reprodução por decisão neural entre mínimo e limite, com divisão forçada no
  limite;
- raio físico pela energia, custo corporal pela área e anti-cramming pelo raio
  real;
- cor genética, assinatura PG e similaridade de Hamming sobre os 1.251 bits;
- salvar/restaurar sessões e telemetria do ecossistema.

**Ainda não implementado:**

- a mutação adaptativa descrita neste documento (planejada para a V5);
- leitura dos salvamentos pelo laboratório para campanhas derivadas.

## Próxima decisão antes de programar

Validar somente a faixa inicial: manter `0,25× … 3,36×` é a opção recomendada.
Se quisermos uma evolução mais agressiva, podemos abrir o teto para `4×`, mas
isso deve ser uma escolha explícita porque aumenta muito a variância entre
linhagens.


## Versão em matriz (pedido de 22/11/2026, a detalhar)

**Ideia:** os 64 registros de ligação são divididos em grupos (por exemplo 8
grupos de 8 ligações). Cada grupo tem três genes reguladores de 4 bits:
topologia, peso e ativação. São 8 × 3 = 24 genes, 96 bits. Uma linhagem pode
deixar um grupo quase congelado (um circuito que já funciona) e outro bem
exploratório.

**Em chance, por nascimento, para um grupo de 8 ligações** (taxa-base de
1/1.024 por bit; "chance" é a de pelo menos um bit daquele tipo mudar no filho):

| Tipo (bits no grupo) | Nível 0 (0,25×) | Nível 8 (1×, hoje) | Nível 15 (3,36×) |
|---|---:|---:|---:|
| Topologia (96 bits: origem e destino) | 2,3% | 9,0% | 27,1% |
| Peso (48 bits) | 1,2% | 4,6% | 14,6% |
| Ativação (8 bits: liga/desliga) | 0,2% | 0,8% | 2,6% |

Pontos a decidir quando o item entrar no cronograma: o tamanho dos grupos
(8 de 8, 16 de 4, …), se os 30 bits de atividade dos neurônios entram num
grupo próprio, e o custo de genoma (96 bits na proposta acima, contra 16 da
versão simples).
