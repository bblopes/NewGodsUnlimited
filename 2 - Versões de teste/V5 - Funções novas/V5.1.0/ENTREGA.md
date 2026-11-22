# V5.1.0 — Sol contínuo e pincel de luz

**Data:** 22/11/2026 · **Versão maior:** V5 — Funções novas

## Pedido

- Malha de luz contínua, luz basal, intensidade do sol, limiar de reprodução
  e uma escala para ver a intensidade no mapa.
- Mais de uma região de sol: sai o desenho por contorno, entra um pincel com
  raio, intensidade máxima (o pico) e força. Força 100% pinta a intensidade
  máxima até a borda; força 0% deixa só o pico no centro, caindo a zero na
  borda.
- A visão limitada pela luz fica para depois.

## Entregue

- **Malha de luz de 0 a 100%** por casa do mapa (320 × 180). A luz efetiva é
  `basal + (1 − basal) × intensidade × luz pintada`.
- **Física › Sol:**
  - Intensidade do sol, Luz basal (padrão 1%) e Luz mínima para reproduzir
    (padrão 0%): valem ao aplicar;
  - Luz média do mapa e a luz mais escura;
  - **Pincel de luz** e **Sombra**, com Raio, Intensidade máxima e Força.
    O cursor mostra o raio e, tracejado, o miolo cheio;
  - Desfazer, Limpar sol, Aplicar mapa.
- **Escala de intensidade** (Ver intensidade, em Física ou em Camadas): mapa
  de cores da luz efetiva, legenda no canto e a luz sob o mouse.
- **Fitoplâncton:** se multiplica com chance `taxa × luz` onde a luz passa do
  limiar, e vive `1 + 0,5 × luz` vezes a vida-base.

**O que muda para você:** com a luz basal de 1%, o fitoplâncton se
multiplica bem devagar também fora dos focos. Para o comportamento antigo,
ponha a luz basal em 0%. Mapas e saves antigos abrem com o sol antigo como
luz máxima.

## Medição

A preencher com a comparação main × branch do CI.

## Como testar

1. Em Física › Sol, ligue Ver intensidade e pinte dois ou três focos com
   forças diferentes; aplique o mapa.
2. Passe o mouse no mapa e veja a luz sob o cursor na legenda.
3. Mude a luz basal e o limiar, aplique e veja onde o fitoplâncton cresce.

## Ficou de fora

Presets de Sol, presets de Construção e mapas embutidos (V5.1.1); visão
limitada pela luz.
