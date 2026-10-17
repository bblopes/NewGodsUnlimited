# Luz solar e ciclo do fitoplâncton

Estado: **implementado no código**.

O mapa possui uma camada independente de iluminação. Nesta primeira etapa, cada
pixel lógico vale `0` (sombra) ou `1` (sol pleno). A antiga área fértil é agora
a **região ensolarada**: mapas antigos preservam o mesmo desenho, mas seu
significado ecológico mudou.

## Regras atuais

- A emissão artificial é uniforme no mapa inteiro e tem um único controle.
- O fitoplâncton iluminado não envelhece e possui `0,002 × luz` de chance de
  duplicar por tick.
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
a taxa de reprodução. Essa mesma luz alimentará a futura temperatura do mapa,
que então afetará metabolismo das células e, depois, as correntes.

## Backlog: visão de alimento e quimiorrecepção

Esta é uma proposta registrada, **não implementada**. Os quatro slots hoje
chamados de sensores de alimento passarão a representar **visão direcional de
alimento**: esquerda, direita, cima e baixo. O alcance codificado no genoma
continua sendo a capacidade anatômica máxima, mas a iluminação local deverá
limitar seu raio efetivo. Sem luz, esses quatro sinais visuais devem chegar a
zero, mesmo em uma célula com gene de visão de longo alcance.

Para que uma célula não fique sem qualquer forma de encontrar alimento no
escuro, o fitoplâncton poderá depositar uma substância química alimentar em um
campo de PPM próprio. Uma etapa posterior definirá sensores neurais separados
para ler concentração e vetores de gradiente desse campo. Eles funcionariam
com ou sem luz e criariam duas estratégias fisiológicas distintas:

- **visão:** informação direcional direta, de alcance limitado pela luz;
- **quimiorrecepção:** leitura de PPM/gradiente de alimento, disponível no
  escuro, mas dependente de um campo químico e de novos slots sensoriais.

Antes de implementar, será preciso decidir a curva que transforma luz em raio
efetivo de visão, os custos energéticos dos novos receptores químicos, quantos
slots entrarão no cérebro e como o PPM alimentar difunde e decai. A mudança
também deve renomear textos da interface e da documentação para que “sensor de
alimento” não continue sugerindo que se trata de uma percepção química.
