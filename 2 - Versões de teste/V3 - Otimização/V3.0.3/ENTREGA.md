# V3.0.3 — Paralelismo opcional e medidor de ticks/s corrigido

**Data:** 21/11/2026 · **Versão maior:** V3 — Otimização

## Pedido

- Quarto passo do roteiro da V3: usar os núcleos do processador nas etapas
  do tick que só leem o estado do mundo, com controle de threads na
  interface.
- Da anotação "Pendências offline V3": em 1× a interface mostrava cerca de
  200 ticks/s (deveria ser cerca de 10), e o contador não mudava ao subir
  para 1.000.000×, embora as células andassem mais rápido.

## Entregue

- **Medidor de ticks/s corrigido.** O número mostrado era o *teto de
  cálculo*: quanto o motor faria por segundo se não esperasse. Ele não
  descontava as pausas do limitador de velocidade. Por isso aparecia ~200 em
  1× e não mudava com o multiplicador. Agora "Ticks/s" é a velocidade real
  (ticks avançados ÷ tempo de relógio), no topo, na Sessão, na Demografia e
  no gráfico de velocidade. O teto aparece à parte na Sessão, como "Teto de
  cálculo (sem limitador)". O limitador em si estava certo: 1× roda a ~10
  ticks/s.
- **Paralelismo nas etapas de leitura:** movimento do fitoplâncton, contato,
  visão, química, cérebro, alimentação, limpeza e o cálculo das amostras do
  feromônio das células. A soma do feromônio continua em sequência, na mesma
  ordem.
- **Seletor "Threads de cálculo"** na Sessão (Auto, 1, 2, 4, 8, máximo).
  **Padrão: 1 thread** (ver medição).
- Testes novos: 1, 3 e 4 threads terminam exatamente no mesmo estado, e o
  feromônio paralelo é idêntico ao sequencial (53 testes passam).

## Medição (mesma máquina do GitHub, 4 vCPU)

| Caso | 1 thread | todos os núcleos | Efeito do paralelismo | Estado igual |
|---|---:|---:|---:|---|
| Realista (100 células) | 5.521 | 4.961 | 0,90× | sim |
| Estresse (300 células) | 846 | 908 | 1,07× | sim |
| Sessão (~5.400 fitoplânctons) | 1.634 | 1.424 | 0,87× | sim |
| Sessão, 200 células iniciais | 1.739 | 1.506 | 0,87× | sim |

**Resultado honesto: o paralelismo não compensou nesta máquina.** O tick
leva cerca de meio milissegundo, e cada etapa paralela precisa acordar as
outras threads e esperar por elas. Esse custo fixo é da mesma ordem do
trabalho dividido. Uma medição anterior, em outra máquina, deu até 1,2× no
estresse. A variação entre máquinas é maior que o ganho. Por isso o padrão
ficou em 1 thread. No seu Ryzen (8 núcleos) vale testar Auto, 4 e 8 e
comparar o "Teto de cálculo".

Em 1 thread, a velocidade é a mesma da V3.0.2, dentro da variação da
medição.

## Como testar

1. Abra `New Gods Unlimited V3.0.3.exe`. Em 1×, "Ticks/s" deve ficar perto
   de 10. Suba o multiplicador e veja o número acompanhar até bater no teto.
2. Na Sessão, compare "Teto de cálculo" com 1, 4, 8 e Auto threads.

## Ficou de fora

- Paralelismo que compense de verdade exigiria ticks maiores (milhares de
  células) ou juntar etapas para acordar as threads uma vez por tick.
- Regras mais leves para o fitoplâncton (mudam a simulação; decisão sua).
