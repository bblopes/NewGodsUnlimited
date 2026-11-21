# Proposta: telemetria demográfica de energia

**Estado: proposta — não implementada.**

Esta nota guarda uma melhoria de produto para uma versão futura. Ela não muda
as regras biológicas nem o desempenho da simulação atual.

## Objetivo

Tornar a aba **Demografia** mais útil para interpretar a energia da população
e relacionar mudanças de configuração aos seus efeitos ao longo do tempo.

## Requisitos registrados

1. No gráfico de energia, desenhar uma referência visual para a energia mínima
   necessária à reprodução. A referência deve usar o valor que estava em vigor
   naquele ponto da execução, e não apenas o valor atual do painel.
2. Quando uma configuração que afeta a simulação for aplicada durante uma
   sessão, registrar um marco temporal. A aba Demografia deve poder mostrar
   esse marco sobre os gráficos, para indicar em que tick a mudança ocorreu.
3. O marco deve identificar pelo menos: tick da simulação, nome da configuração
   alterada e valor anterior/novo valor. Isso permite correlacionar uma mudança
   com cada série demográfica, sem fingir que a mudança foi causada pela
   população.
4. Os marcos precisam sobreviver ao salvamento e à restauração de uma sessão,
   pois fazem parte da leitura histórica daquele experimento.

## Decisões a estudar antes de implementar

- **Escopo de "mudança":** quais controles geram um marco? A recomendação é
  começar apenas por parâmetros que alteram a dinâmica do mundo (por exemplo,
  limiar de reprodução, custo metabólico e oferta de alimento), excluindo
  ajustes puramente visuais.
- **Precisão temporal:** o marco deve ser associado ao tick em que a alteração
  passa a valer. Se uma alteração vier da interface entre dois ticks, ela deve
  aparecer no próximo tick processado, evitando ambiguidade.
- **Apresentação:** confirmar se os gráficos receberão linhas verticais com
  rótulos, uma faixa discreta no eixo do tempo, ou ambos. Com muitos eventos,
  a interface deverá agrupar ou filtrar marcos para não esconder a série.
- **Energia de reprodução variável:** se o limiar puder mudar durante a sessão,
  a referência precisa ser uma série em degraus — não uma linha horizontal
  única. Caso ele seja imutável numa execução, uma linha horizontal basta.
- **Origem das mudanças:** diferenciar mudanças manuais, presets, restauração
  de sessão e automações, para que o histórico explique quem aplicou cada
  parâmetro.
- **Custo de memória:** guardar eventos esparsos, e não duplicar o conjunto de
  parâmetros em toda amostra demográfica.

## Desenho técnico inicial (a validar)

Criar um pequeno registro de eventos da sessão, separado das amostras
demográficas. Cada evento teria `tick`, `origem`, `campo`, `valor_anterior` e
`valor_novo`. A interface de Demografia consultaria esse registro para desenhar
os marcos no mesmo eixo temporal de suas séries. O limiar de reprodução seria
registrado como uma série de degraus derivada dos eventos, iniciando no valor
da configuração da sessão.

Antes de codificar, vale mapear o caminho atual entre controle da interface,
configuração da simulação, coleta de métricas e serialização da sessão. Assim
o ponto de registro será único e não perderá mudanças aplicadas por caminhos
diferentes.
