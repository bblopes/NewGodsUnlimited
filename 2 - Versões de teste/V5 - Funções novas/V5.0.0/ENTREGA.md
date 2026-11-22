# V5.0.0 — Sensor de idade e indivíduo Baseline

**Data:** 22/11/2026 · **Versão maior:** V5 — Funções novas

## Pedido

- Um neurônio de sobrevivência: a célula sabe a própria idade e, perto de
  morrer de velhice, se reproduz ("vai morrer de idade, se reproduz de última
  hora").
- Acabar com a configuração neural Clássica/Motor/PF guia: ficam só os
  indivíduos do banco.
- Um indivíduo **Baseline**: o PF guia sem os internos (internos desligados,
  motores ligados) com a amarração da idade. O PF guia e o "Rede de
  espaçamento" ganham a mesma amarração.
- Mínimo de energia continua valendo; fundadores com uma rodada de mutação;
  sem série própria na Demografia (conta como decisão neural).
- Na rede ao vivo, ver o nome de qualquer nó ao passar o mouse, com
  "inativo" quando for o caso.

## Entregue

- **Sensor de idade `Id`** (slot 26, logo abaixo de `PFy`): quanto da
  própria vida a célula já viveu, de 0 a 1. Não custa energia. A rede passa
  de 56 para 57 slots.
- **População inicial** no lugar de "Configuração neural": **Baseline** e
  **PF guia** embutidos (não podem ser apagados) e os seus indivíduos
  salvos. As células iniciais são cópias do escolhido, cada uma com uma
  rodada de mutação. Inserir célula insere uma cópia exata.
- **Baseline:** `PFx`/`PFy` direto nos motores de nado e
  `8 × Id − 5,75` no motor Reproduzir, que passa do limiar a partir de
  ~89% da vida. Sem internos, ele nada na diagonal quando há cheiro nos dois
  eixos.
- **PF guia** embutido agora tem a mesma amarração. Para os seus indivíduos
  (como o "Rede de espaçamento"), abra o olho no banco e clique em
  **Adicionar reprodução de última hora**: é criada uma cópia "+ idade".
- **Rede ao vivo:** tooltip na hora em qualquer nó, com área de mouse maior.

**O que muda para você:** saves e indivíduos antigos são migrados ao abrir,
com as mesmas ligações de antes (o banco guarda uma cópia
`banco_de_individuos.v1.json`). Runs salvas pela V5 não abrem em versões
anteriores.

## Medição

Mesma máquina do GitHub, main (V4.0.1) × V5.0.0:

| Caso | main (ticks/s) | V5.0.0 (ticks/s) | Ganho | Estado igual | Comportamento igual |
|---|---:|---:|---:|---|---|
| Realista (100 células) | 6.463 | 6.602 | 1,02× | não | sim |
| Estresse (300 células) | 952 | 956 | 1,00× | não | não |
| Sessão (100 células) | 1.853 | 1.661 | 0,90× | sim | sim |
| Sessão (200 células) | 1.962 | 2.005 | 1,02× | sim | sim |

**Resultado honesto:** a velocidade fica igual, dentro do ruído das máquinas
do GitHub. O 0,90× da sessão com 100 células não se repete com 200, e o
sensor é só uma divisão por célula. **O estado mudou de propósito:** o save
agora guarda 27 sensores, e a rede de 57 slots muda o que uma mutação pode
produzir (um registro que antes apontava para um slot inexistente pode cair
num slot válido). Por isso o comportamento só diverge onde há muitos
nascimentos e mutações (estresse). 62 testes passam, inclusive os novos:
Baseline decide reproduzir a partir de Id ≈ 0,89 e não antes, segue o PF
(inclusive na diagonal), a migração de 56 para 57 slots preserva todas as
ligações e a amarração nova não mexe nas ligações que já existiam.

## Como testar

1. Em **Sessão**, veja a População inicial com o Baseline selecionado e
   reinicie.
2. Em **Células › Indivíduo**, passe o mouse no nó `Id` e num interno
   cinza: aparecem o nome e "inativo".
3. Siga uma célula perto do limite de idade: a partir de ~89% da vida ela se
   divide, se tiver energia acima do mínimo.
4. Abra o "Rede de espaçamento" no banco e clique em **Adicionar reprodução
   de última hora**.

## Ficou de fora

Behavior Spotter (V5.0.1), abas Física/Mapa e tema claro (V5.0.2). Campanhas
antigas do Laboratório não foram migradas para os 57 slots.
