# Luz solar e ciclo do fitoplâncton

Estado: **implementado no código**.

O mapa possui uma camada independente de iluminação. Nesta primeira etapa, cada
pixel lógico vale `0` (sombra) ou `1` (sol pleno). A antiga área fértil é agora
a **região ensolarada**: mapas antigos preservam o mesmo desenho, mas seu
significado ecológico mudou.

## Regras atuais

- O **nascimento por abiogênese** é uniforme no mapa inteiro e pode ser
  regulado de `0` a `1` partícula por tick. Em `0`, não surge fitoplâncton do
  nada.
- A **multiplicação sob luz** é regulável de `0` a `2%` por tick. O valor
  padrão é `0,2%`: uma partícula iluminada possui
  `taxa de multiplicação × luz` de chance de duplicar por tick.
- A energia entregue por cada fitoplâncton também é um valor numérico livre na
  sessão; não depende mais de passos fixos de cinco unidades.
- A cópia nasce até 70 unidades da partícula-mãe, somente se também cair em
  luz e não atravessar parede.
- Cada quadrado local de 120 unidades aceita no máximo seis partículas; isso
  permite recuperação de regiões vazias sem explosão infinita de alimento.
- Na sombra, o fitoplâncton não se reproduz e seu cronômetro de vida avança.
  Ao voltar para luz, o relógio para; ele não rejuvenesce.
- A morte por idade ocorre apenas quando o alimento está fora de luz e já
  atingiu o limite de vida configurado. Ser consumido continua removendo-o
  imediatamente.

## Próximas camadas

A consulta do mapa já é exposta como intensidade numérica. Hoje retorna apenas
`0` ou `1`; no próximo passo, uma máscara contínua poderá regular diretamente
a taxa de reprodução e interpolar o bônus de longevidade do fitoplâncton. Nesta
primeira versão, o sol pleno aplica um multiplicador fixo de vida de `1,5×`, sem
parar o envelhecimento. Essa mesma luz alimentará a futura temperatura do mapa,
que então afetará metabolismo das células e, depois, as correntes.

## Backlog: visão de alimento e quimiorrecepção

Esta é uma proposta registrada, **não implementada**. Os quatro slots hoje
chamados de sensores de alimento passarão a representar **visão direcional
genérica**: esquerda, direita, cima e baixo. Cada slot deve produzir sinal
positivo ao detectar qualquer entidade visível, não somente fitoplâncton. O
alcance codificado no genoma continua sendo a capacidade anatômica máxima, mas
a iluminação local deverá limitar seu raio efetivo. Sem luz, esses quatro sinais
visuais devem chegar a zero, mesmo em uma célula com gene de visão de longo
alcance.

O fitoplâncton deverá depositar uma substância química alimentar em um campo de
PPM próprio. Uma etapa posterior definirá sensores neurais separados para ler
concentração e vetores de gradiente desse campo. Eles funcionariam com ou sem
luz e serão a forma pela qual a célula identifica e segue a comida. Isso cria
duas estratégias fisiológicas distintas:

- **visão:** presença direcional de qualquer entidade, de alcance limitado pela
  luz;
- **quimiorrecepção:** identificação de alimento por PPM e orientação pelo
  vetor gradiente, disponível no escuro, mas dependente de um campo químico e
  de novos slots sensoriais.

Antes de implementar, será preciso decidir a curva que transforma luz em raio
efetivo de visão, os custos energéticos dos novos receptores químicos, quantos
slots entrarão no cérebro e como o PPM alimentar difunde e decai. A mudança
também deve renomear textos da interface e da documentação para que “sensor de
alimento” não continue sugerindo que se trata de uma percepção química.
