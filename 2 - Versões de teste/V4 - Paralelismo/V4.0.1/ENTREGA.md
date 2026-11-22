# V4.0.1 — Threads que ficam acordadas entre as etapas

**Data:** 22/11/2026 · **Versão maior:** V4 — Paralelismo

## Pedido

- Fazer o paralelismo render: as threads eram acordadas a cada etapa do
  tick, e esse custo comia o ganho de dividir o trabalho. Última entrega da
  V4 (as regras mais leves para o fitoplâncton foram descartadas).

## Entregue

- **Grupo de threads próprio, que fica acordado.** Depois de cada etapa
  paralela, as threads giram por até 0,3 ms esperando a próxima, em vez de
  dormir. Sincronizar uma etapa caiu para poucos microssegundos. Com o
  limitador de velocidade (1×, 10×…), elas dormem entre ticks e não gastam
  processador.
- O trabalho é cortado em blocos e cada bloco é feito inteiro por uma
  thread: **o resultado é idêntico com qualquer número de threads**.
- O número de threads é limitado aos núcleos do processador: com mais
  threads que núcleos, threads que giram atrapalham.
- Testes novos: igualdade com 1 a 4 threads e erro numa thread sem travar o
  programa (57 testes passam).

## Medição (mesma máquina do GitHub, 4 vCPU)

| Caso | V4.0.0 | V4.0.1 | Ganho | V4.0.1: todos os núcleos × 1 thread | Estado igual |
|---|---:|---:|---:|---:|---|
| Realista (100 células) | 4.988 | 6.507 | 1,30× | 1,10× | sim |
| Estresse (300 células) | 917 | 948 | 1,03× | 1,09× | sim |
| Sessão (~5.400 fitoplânctons) | 1.598 | 1.834 | 1,15× | 1,15× | sim |
| Sessão, 200 células iniciais | 1.672 | 2.005 | 1,20× | 1,16× | sim |

Pela primeira vez **usar mais núcleos compensa em todos os cenários**: na
V3.0.3 o mesmo teste dava de 0,87× a 1,07×. O ganho ainda é modesto,
porque boa parte do tick é sequencial (interações, reprodução, nascimento
do fitoplâncton). No seu Ryzen, com 8 núcleos, pode render mais. O padrão
da interface continua em 1 thread: escolha **Auto** na Sessão para usar
todos.

## Como testar

1. Abra `New Gods Unlimited V4.0.1.exe` e rode uma sessão com o oásis cheio.
2. Na Sessão, compare o "Teto de cálculo" com 1 thread, 4, 8 e Auto.
3. Com Auto, o Gerenciador de Tarefas deve mostrar o programa usando vários
   núcleos enquanto roda no máximo.

## Ficou de fora

- Regras mais leves para o fitoplâncton: descartadas.
- Paralelizar as partes sequenciais (interações em duas fases, nascimento
  do fitoplâncton): pouco ganho com a população atual.
