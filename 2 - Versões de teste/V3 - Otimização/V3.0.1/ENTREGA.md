# V3.0.1 — Assinatura genética em cache, distância às paredes e demografia ampliada

**Data:** 21/11/2026 · **Versão maior:** V3 — Otimização

## Pedido

- Segundo passo do roteiro da V3: parar de recalcular a assinatura genética
  (PG) e os canais P1–P3 de cada célula a cada tick.
- Fitoplâncton vivo no topo, ao lado de células.
- Na Demografia: população de fitoplâncton e velocidade da simulação
  (ticks/s) evoluindo junto com o resto, mais um resumo rápido.

## Entregue

- **Assinatura e canais guardados no genoma.** Eles são calculados uma vez
  e descartados só se um bit do genoma mudar. Antes, cada célula percorria
  o genoma inteiro quatro vezes por tick para isso.
- **Distância até a parede mais próxima.** Um mapa calculado uma vez (e
  refeito quando as paredes mudam) diz, para cada ponto, a que distância
  está a parede mais próxima. Feromônios e visão deixam de traçar linha de
  visão até alvos mais próximos que essa parede, porque nenhuma parede pode
  estar no caminho. As linhas mais longas continuam sendo traçadas.
- **Interface:**
  - "Fitoplâncton" no topo, ao lado de Células.
  - Na Demografia, o resumo ganhou "Fitoplâncton vivo" e "Ticks/s".
  - Gráficos novos: **População de fitoplâncton** e **Velocidade da
    simulação** (ticks/s no momento de cada amostra).

## Medição (GitHub, Windows 4 vCPU)

| Caso | V2.0.3 | V3.0.0 | **V3.0.1** |
|---|---:|---:|---:|
| Realista curto (100 células) | 1.349 | 1.806 | **7.417** ticks/s |
| Estresse (300 células) | 243 | 265 | **1.137** ticks/s |
| Sessão padrão (~5.400 fitoplânctons) | 112 | 793 | **1.412** ticks/s |
| Sessão, 200 células iniciais | 115 | 862 | **1.518** ticks/s |

Nos quatro casos, o estado final (hash) é **igual** ao da V2.0.3. Os 51
testes do motor passam, inclusive um novo que confere, em 4.000 pontos do
mapa, que a distância calculada nunca passa da distância real até a parede.

Observação honesta: a parte da sessão sem células (movimento do
fitoplâncton) não mudou de código e mesmo assim saiu cerca de 40% mais
rápida. As máquinas do GitHub variam entre execuções. Para cenários com
células, o ganho real é o da coluna (4×).

Maiores custos restantes: no estresse, feromônio das células (0,36 ms) e
interações (0,24 ms); na sessão, movimento do fitoplâncton (0,40 ms).

## Como testar

1. Abra `New Gods Unlimited V3.0.1.exe`; o topo mostra "Fitoplâncton".
2. Vá em Demografia e acompanhe os gráficos de fitoplâncton e de
   velocidade; mude a velocidade da sessão e veja a curva de ticks/s
   responder.
3. Compare os ticks/s com a V3.0.0 numa sessão com muitas células.

## Ficou de fora

Índices de vizinhos mais leves e fim da cópia do mapa por tick (V3.0.2),
paralelismo (V3.0.3).
