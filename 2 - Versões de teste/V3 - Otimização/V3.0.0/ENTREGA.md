# V3.0.0 — Feromônio de alimento pré-calculado e clique alinhado

**Data:** 21/11/2026 · **Versão maior:** V3 — Otimização

## Pedido

- Primeira entrega da V3, seguindo `3 - Documentos/Relatórios/AVALIACAO_DE_DESEMPENHO.md`:
  tirar o maior custo do tick, o espalhamento do feromônio de alimento (PF).
- Corrigir o bug da V2.0.3: depois de um tempo rodando, o clique na arena
  desviava em X (o Y continuava certo).

## Entregue

- **PF pré-calculado.** A geometria do PF (para onde o feromônio de cada
  ponto da malha chega, já respeitando as paredes) passa a ser calculada uma
  vez e reaproveitada. Ela só é refeita quando as paredes mudam. O campo
  resultante é idêntico ao anterior, bit a bit.
- **Clique alinhado.** O desenho da arena agora acompanha o próprio tamanho.
  Antes ele só se ajustava quando a janela do Windows mudava. Ao arrastar,
  recolher ou alargar o painel lateral, o desenho ficava esticado na
  horizontal e o clique errava em X.

## Medição (GitHub, Windows 4 vCPU)

| Caso | V2.0.3 | V3.0.0 | Ganho |
|---|---:|---:|---:|
| Sessão padrão (~5.400 fitoplânctons) | 111,8 ticks/s | **793,1 ticks/s** | **7,1×** |
| Sessão, 200 células iniciais (~5.000 fitoplânctons) | 115,3 ticks/s | **862,0 ticks/s** | **7,5×** |
| Realista curto (100 células) | 1.349 ticks/s | 1.806 ticks/s | 1,34× |
| Estresse (300 células) | 243 ticks/s | 265 ticks/s | 1,09× |

O PF caiu de 7,8 ms para 0,2 ms por tick. Nos quatro casos, o estado final
(hash) é **igual** ao da V2.0.3: a simulação não mudou. Os 50 testes do motor
passam, inclusive um novo que compara o PF com a implementação de referência,
antes e depois de pintar paredes.

O novo maior custo da sessão é o movimento do fitoplâncton (0,65 ms). No
estresse, continua sendo o feromônio das células (1,8 ms), alvo da V3.0.1.

## Como testar

1. Abra `New Gods Unlimited V3.0.0.exe` e deixe a sessão padrão rodar com
   velocidade alta; compare os ticks/s com a V2.0.3 depois que o
   fitoplâncton se espalhar.
2. Arraste a borda do painel lateral, recolha e reabra o painel, e clique
   em células nas laterais da arena: o clique deve acertar.
3. Pinte uma parede no Mapa e ative a camada de PF: o feromônio deve
   respeitar a parede nova.

## Ficou de fora

- Assinatura genética em cache (V3.0.1), índices de vizinhos (V3.0.2),
  paralelismo (V3.0.3). Ver o roteiro na avaliação.
- Um cenário de medição que mantenha 200 células vivas na sessão (os
  fundadores de referência morrem nela).
