# Longevidade hereditária

**Estado: implementado no código.**

## Regra

Cada célula recebe um alelo de longevidade no nascimento. Ele define sua idade
máxima, em ticks. Ao atingir essa idade, a célula morre mesmo que ainda tenha
energia.

- referência inicial: **100.000 ticks**;
- intervalo possível: 50.000 a 1.000.000 ticks;
- fundadores nascem perto da referência, com variação inicial de ±15%;
- a reprodução herda o alelo e, raramente, altera um bit dele.

O limite de energia continua sendo a pressão para dividir; o limite de idade
vira a segunda pressão. Uma linhagem que não se reproduz antes de sua própria
idade máxima desaparece.

## Separação do cérebro

O alelo não entra nos 1.216 bits do genoma neural. Portanto ele não muda:

- cor baseada em genoma;
- similaridade genética atual;
- assinatura de feromônio genética;
- conexões e pesos cerebrais.

É uma característica fisiológica hereditária, não uma ligação neural. Isso
evita que a métrica de diversidade neural passe a medir, sem aviso, somente
diferenças de expectativa de vida.

## Compatibilidade com runs antigas

Runs salvas antes desta mudança não possuem o alelo. Ao serem abertas, cada
célula recebe uma margem de vida adicional baseada na referência, para que uma
célula velha não morra imediatamente só por migrar de versão.

