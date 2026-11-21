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
- O cronômetro de vida avança sempre. Na sombra, o fitoplâncton não se
  reproduz; sob sol pleno, seu limite de vida recebe multiplicador fixo de
  `1,5×`, sem imortalidade nem rejuvenescimento.
- Ser consumido continua removendo o fitoplâncton imediatamente.

## Próximas camadas

A consulta do mapa já é exposta como intensidade numérica. Hoje retorna apenas
`0` ou `1`; no próximo passo, uma máscara contínua poderá regular diretamente
a taxa de reprodução e interpolar o bônus de longevidade do fitoplâncton. Nesta
primeira versão, o sol pleno aplica um multiplicador fixo de vida de `1,5×`, sem
parar o envelhecimento. Essa mesma luz alimentará a futura temperatura do mapa,
que então afetará metabolismo das células e, depois, as correntes.

## Visão de alimento e quimiorrecepção

Estado: **implementado na build experimental de PF**. Os quatro slots antes
chamados de sensores de alimento agora representam **visão direcional
genérica**: esquerda, direita, cima e baixo. Eles respondem a qualquer entidade
visível, não somente fitoplâncton. O alcance genético continua anatômico; a
limitação por luz permanece pendente.

O fitoplâncton agora deposita a substância ambiental `PF`, com sensores
neurais de intensidade e gradiente. Eles funcionam sem luz e permitem seguir
o alimento. Isso cria duas estratégias fisiológicas distintas:

- **visão:** presença direcional de qualquer entidade, de alcance limitado pela
  luz;
- **quimiorrecepção:** identificação de alimento por PPM e orientação pelo
  vetor gradiente, disponível no escuro, mas dependente de um campo químico e
  de novos slots sensoriais.

Ainda falta decidir a curva que transforma luz em raio efetivo de visão e se PF
ganhará persistência, difusão e decaimento. A descrição atual do canal está em
[PG, visão e canal de alimento](PG_E_CANAL_DE_ALIMENTO.md).
