# V4.0.0 — PF em paralelo e uma passada a menos no fitoplâncton

**Data:** 22/11/2026 · **Versão maior:** V4 — Paralelismo

## Pedido

- Paralelizar de verdade as partes pesadas do tick, juntando etapas para
  acordar as threads menos vezes, sem mudar o resultado da simulação.

## Entregue

- **PF (feromônio do alimento) em paralelo.** Era a parte grande do tick que
  ainda rodava num núcleo só. A malha química é dividida em faixas de linhas
  e cada thread espalha o PF só na sua faixa, somando as fontes na mesma
  ordem de antes. Só liga quando há trabalho suficiente (sessão com muito
  fitoplâncton); com pouco alimento fica em uma thread.
- **Uma passada a menos no fitoplâncton.** A conta de "esta partícula já
  venceu?" agora é feita junto com o movimento. A limpeza só junta isso com
  quem foi comido, sem uma passada paralela própria.
- Teste novo: PF com 1, 3 e 4 threads é idêntico ao de referência
  (55 testes passam).
- Medição corrigida: a comparação com a `main` agora é nas mesmas condições
  e mostra também todos os núcleos × 1 thread.

## Medição (mesma máquina do GitHub, 4 vCPU)

| Caso | V3.0.4 | V4.0.0 | Ganho | V4.0.0: todos os núcleos × 1 thread | Estado igual |
|---|---:|---:|---:|---:|---|
| Realista (100 células) | 4.959 | 4.815 | 0,97× | 0,84× | sim |
| Estresse (300 células) | 924 | 924 | 1,00× | 1,07× | sim |
| Sessão (~5.400 fitoplânctons) | 1.455 | 1.587 | 1,09× | 1,02× | sim |
| Sessão, 200 células iniciais | 1.508 | 1.660 | 1,10× | 0,99× | sim |

A sessão ficou **~10% mais rápida**, com estado e comportamento iguais aos
da V3.0.4. Realista e estresse ficaram iguais, dentro da variação. Resultado
honesto: o ganho vem de fazer menos trabalho repetido, não dos núcleos.
Nesta máquina, todos os núcleos ainda empatam com 1 thread na sessão. O tick
é curto demais para dividir. O padrão continua em 1 thread.

## Como testar

1. Abra `New Gods Unlimited V4.0.0.exe` e rode uma sessão longa (oásis cheio
   de fitoplâncton).
2. Na Sessão, compare o "Teto de cálculo" com 1 thread, Auto e 8 threads. No
   seu Ryzen o paralelismo pode render mais que nos 4 núcleos do GitHub.

## Ficou de fora

- Juntar sensores, cérebro e movimento das células numa passada única: com
  menos de 512 células essas etapas já rodam em sequência e custam pouco.
- Regras mais leves para o fitoplâncton (mudam a simulação; decisão sua).
