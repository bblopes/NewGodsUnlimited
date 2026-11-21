# Proposta: coeficientes hereditários de mutação

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

Hoje o genoma contém 64 registros de 19 bits:

```text
[origem: 6][destino: 6][peso: 6][ativo: 1]
```

Proposta: adicionar um cabeçalho de 16 bits antes desses registros.

```text
[mutação topologia: 4][mutação peso: 4][mutação ativação: 4][reserva: 4]
[64 × registro neural de 19 bits]
```

O genoma passa de 1.216 bits (152 bytes) para 1.232 bits (154 bytes). Os quatro
bits reservados ficam intactos por enquanto: servem para futuras regras sem
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

## Estado atual do projeto — lembrete

**Implementado e publicado no executável principal:**

- mundo 2D, fluxo ambiental e fitoplâncton;
- cérebro hereditário com 64 registros de conexão e 34 slots neurais;
- sensores de comida por cones direcionais, corrente, energia, contato e
  similaridade genética no contato;
- reprodução por decisão neural entre mínimo e limite, com divisão forçada no
  limite;
- raio físico sem teto: `14` em energia zero, `28` em 2.000 de energia e
  crescimento pela raiz quadrada da área após esse ponto;
- custo corporal baseado na área, comprimido de `1,00×` para no máximo `1,40×`;
- anti-cramming baseado no raio real;
- cor genética contínua e similaridade de Hamming;
- salvar/restaurar sessões e telemetria do ecossistema.

**Documentado, mas ainda não implementado no ecossistema principal:**

- osmose, engolfamento e sensores de força;
- mutação adaptativa descrita neste documento;
- leitura desses salvamentos pelo laboratório para campanhas derivadas.

## Próxima decisão antes de programar

Validar somente a faixa inicial: manter `0,25× … 3,36×` é a opção recomendada.
Se quisermos uma evolução mais agressiva, podemos abrir o teto para `4×`, mas
isso deve ser uma escolha explícita porque aumenta muito a variância entre
linhagens.
