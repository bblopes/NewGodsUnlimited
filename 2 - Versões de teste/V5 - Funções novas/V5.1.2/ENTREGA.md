# V5.1.2 — Correntes Redemoinho e Circular

22/11/2026 · V5 - Funções novas

## Pedido
Duas correntes novas em Física › Corrente, pelo desenho: (1) Redemoinho, como
um tornado, espiral abrindo do centro e meio circular em volta; (2) Circular,
vários anéis girando em torno de um eixo.

## Entregue
- **Redemoinho:** gira no sentido horário e empurra para fora perto do centro,
  formando braços em espiral que se abrem. A ~85% do raio o empurrão inverte,
  então o que chega na borda fica girando num anel, sem se amontoar nas paredes.
- **Circular:** anéis concêntricos girando no sentido anti-horário, parados no
  eixo e com velocidade constante a partir de ~30% do raio até os cantos.
- As duas usam a elipse que cabe no mapa (o mundo é retangular), centradas no
  meio do mapa. A intensidade da corrente vale para as duas.
- Ficam salvas em runs e presets de mapa.

## Medição
A preencher com a comparação main × branch do CI.

## Como testar
1. Física › Corrente › Modelo "Redemoinho", Aplicar.
2. Camadas › ligar os vetores da corrente: espiral saindo do centro e anel na borda.
3. Trocar para "Circular" e aplicar: anéis girando em volta do centro.
4. Ver o fitoplâncton: no Redemoinho ele tende a se juntar no anel da borda.

## Ficou de fora
Mover o centro, mudar o sentido de giro e o raio do anel (dá para virar
controles se quiser). Otimização da luz passa para a V5.1.3.
