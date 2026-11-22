# V5.0.2 — Behavior Spotter: fagocitose ao vivo

**Data:** 22/11/2026 · **Versão maior:** V5 — Funções novas

## Pedido

- Uma aba nova, abaixo de Camadas, para achar um comportamento acontecendo e
  assistir. Nesta versão, só a fagocitose, que você nunca tinha visto direito
  ao vivo.
- Lista ao vivo das ocorrências, com ir até e pular para a próxima, e o evento
  destacado por um círculo vermelho centrado no ponto de contato.
- Opção de pausar ao encontrar: fica pausado enquanto ligada; ao desligar, a
  simulação volta sozinha e a câmera continua seguindo. Ligar com a
  simulação pausada não despausa.
- Sem salvar momento, replay ou botão de avançar 1 tick.

## Entregue

- Aba **Behavior Spotter** com a seção **Fagocitose**:
  - **lista ao vivo**: `#predador puxando #presa · raio · força · há N ticks`;
  - **Ir até** e **Próxima**: a câmera centraliza no ponto de contato, dá zoom
    e segue o predador. Quando a puxada acaba, aparece "capturada" ou
    "escapou" e a câmera é solta;
  - **Pausar ao encontrar**: o motor para no tick exato em que uma célula
    começa a puxar outra, mesmo a 1.000.000×, e a arena aponta para a primeira
    ocorrência;
  - **Terminaram há pouco**: as últimas puxadas encerradas, com o desfecho.
- **Círculo vermelho** pulsando em cada fagocitose ativa (com a aba aberta ou
  a pausa ligada), centrado no ponto de contato, com o raio da presa + 30%.
- Só conta a puxada real: motor de fagocitose ligado, presa ao alcance e
  predador com pelo menos 1,10× o raio dela.

**O que muda para você:** nada na simulação; é observação.

## Medição

Mesma máquina do GitHub, main (V5.0.1) × V5.0.2:

| Caso | main (ticks/s) | V5.0.2 (ticks/s) | Ganho | Estado igual | Comportamento igual |
|---|---:|---:|---:|---|---|
| Realista (100 células) | 12.123 | 12.408 | 1,02× | sim | sim |
| Estresse (300 células) | 1.634 | 1.612 | 0,99× | sim | sim |
| Sessão (100 células) | 3.226 | 3.388 | 1,05× | sim | sim |
| Sessão (200 células) | 3.473 | 3.328 | 0,96× | sim | sim |

**Resultado honesto:** a simulação ficou exatamente igual (mesmo estado
salvo) e a velocidade dentro do ruído. Guardar as puxadas de cada tick não
custa nada perceptível. 63 testes passam, inclusive um novo que confere a
lista de puxadas e o ponto de contato.

## Como testar

1. Abra a aba **Behavior Spotter** e ligue **Pausar ao encontrar**.
2. Deixe rodar em alta velocidade até pausar sozinho: aparecem os círculos e
   a câmera vai até a primeira fagocitose.
3. Baixe a velocidade para 1× e desligue a opção: a simulação volta e a câmera
   segue o predador até "capturada" ou "escapou".

## Ficou de fora

Outros detectores, salvar o momento e replay.
