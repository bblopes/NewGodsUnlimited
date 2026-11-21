# V3.0.4 — Sensores não lidos deixam de ser calculados; rede neural mais clara

**Data:** 21/11/2026 · **Versão maior:** V3 — Otimização

## Pedido

- Não calcular os sensores que nenhuma ligação ativa do cérebro lê.
- Na rede neural ao vivo, mostrar o que existe no genoma e não funciona, e
  quais sensores não são lidos.

## Entregue

- **Sensores sob demanda.** Cada cérebro guarda quais sensores suas
  ligações efetivas leem. O motor só calcula esses: visão, contato,
  corrente e cada grupo químico (PG, P1, P2, P3, PF). Energia, força e a
  constante são baratos e continuam sempre calculados.
- **Rede neural ao vivo:**
  - sensor que nenhuma ligação lê aparece **vazado** e apagado, com a nota
    "não é calculado";
  - ligação que existe no genoma (registro ligado e válido) mas não
    funciona, porque um dos neurônios está desligado, aparece **tracejada**
    em cinza. Passando o mouse, aparece o número do registro;
  - os neurônios desligados continuam em cinza, como antes.

**O que muda para você:** no painel da célula e no save, os sensores não
lidos agora aparecem como 0. O comportamento das células é exatamente o
mesmo, porque o cérebro nunca usava esses valores.

## Medição (mesma máquina do GitHub, V3.0.3 × V3.0.4)

| Caso | V3.0.3 | V3.0.4 | Ganho | Estado salvo igual* | Comportamento igual |
|---|---:|---:|---:|---|---|
| Realista (100 células) | 4.867 | 4.809 | 0,99× | sim | sim |
| Estresse (300 células) | 902 | 924 | 1,02× | sim | sim |
| Sessão (~5.400 fitoplânctons) | 1.405 | 1.416 | 1,01× | sim | sim |
| Sessão, 200 células iniciais | 1.466 | 1.365 | 0,93× | sim | sim |

\* Com a opção de calcular todos os sensores (`--sensores-completos` no
benchmark), o estado salvo é idêntico ao da V3.0.3. "Comportamento igual"
compara posições, energia, idade, motores e alimento entre o modo novo e o
cálculo completo.

**Resultado honesto:** o ganho de velocidade ficou dentro do ruído da
medição. Como previsto na V3.0.3, o custo dos sensores já tinha caído quase
a zero com a assinatura genética em cache, e os fundadores de referência
leem quase todos os sensores. O ganho cresce numa população cujos cérebros
leem poucos sensores. O valor principal desta versão é a rede neural mais
clara. 54 testes passam, inclusive um novo que roda 400 ticks com e sem o
cálculo completo e confere que tudo o que as células fazem é igual.

## Como testar

Abra `New Gods Unlimited V3.0.4.exe`, selecione uma célula e veja a rede
neural em Células: sensores vazados não são lidos, linhas tracejadas são
ligações inertes. Passe o mouse para ver os detalhes.

## Ficou de fora

Regras mais leves para o fitoplâncton (mudam a simulação; decisão sua). É
onde está o maior ganho restante.
