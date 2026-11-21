# V3.0.2 — Índices de vizinhos leves e fitoplâncton mais barato

**Data:** 21/11/2026 · **Versão maior:** V3 — Otimização

## Pedido

Terceiro passo do roteiro da V3: índices de vizinhos mais leves e fim da
cópia do mapa a cada tick, com atenção especial ao custo do fitoplâncton.

## Entregue

- **Índice da visão em grade plana.** A visão procura alimento e células
  numa grade fixa, montada a cada tick sem alocar uma lista por célula. Sem
  células vivas, ela nem é montada.
- **Índices de células mais baratos.** Contato, alimentação, fagocitose e
  anti-cramming usam uma tabela com uma função de espalhamento simples no
  lugar da padrão, que era lenta.
- **Paredes no movimento.** Fitoplâncton e células só testam colisão com a
  parede quando ela está mais perto que o próprio raio. Isso usa o mesmo
  mapa de distância da V3.0.1.
- **Corrente Clássica.** Os redemoinhos são calculados uma vez por seed, e
  não mais a cada partícula, a cada tick.
- **Sem cópias do mapa.** A reprodução e a emissão de fitoplâncton deixaram
  de copiar o mapa inteiro a cada tick.
- **Alimentação** pulada quando não há células.
- **Correção de determinismo.** Quando um predador engolia duas presas no
  mesmo tick, a ordem da soma de energia podia variar de uma execução para
  outra. Agora é sempre a mesma.

## Medição (mesma máquina do GitHub, V3.0.1 × V3.0.2)

A partir desta versão, o workflow mede a versão anterior e a nova
alternadamente, na mesma máquina. As máquinas do GitHub chegaram a variar
1,7× entre execuções, então a comparação entre execuções diferentes não era
confiável.

| Caso | V3.0.1 | V3.0.2 | Ganho | Estado igual |
|---|---:|---:|---:|---|
| Realista (100 células) | 4.858 | 7.807 | 1,61× | sim |
| Estresse (300 células) | 776 | 1.121 | 1,44× | sim |
| Sessão (~5.400 fitoplânctons) | 1.011 | 2.105 | **2,08×** | sim |
| Sessão, 200 células iniciais | 1.102 | 2.291 | **2,08×** | sim |

Na sessão, o índice da visão caiu de 129 µs para 0,4 µs por tick, a
alimentação de 113 µs para 3 µs e o movimento do fitoplâncton de 514 µs para
256 µs. Os 51 testes do motor passam.

O que ainda pesa na sessão é o fitoplâncton: movimento (256 µs),
feromônio de alimento (160 µs) e limpeza (43 µs). No estresse, pesam o
feromônio das células (480 µs) e as interações (168 µs).

## Como testar

Abra `New Gods Unlimited V3.0.2.exe` e acompanhe o gráfico "Velocidade da
simulação" na Demografia, comparando com a V3.0.1 na mesma situação.

## Ficou de fora

Paralelismo nos núcleos do processador (V3.0.3).
