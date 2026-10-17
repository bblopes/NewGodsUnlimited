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
