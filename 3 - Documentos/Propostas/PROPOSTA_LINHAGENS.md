# Proposta: genealogia real e aba Linhagens

**Estado: genealogia aprovada em 22/11/2026; desenho da aba Linhagens
aguardando aprovação.**
Base de código lida: `main` @ `3f41f7a` (V3.0.4 oficial).
Plano geral: [V5_FUNCOES_NOVAS.md](../Backlog/V5_FUNCOES_NOVAS.md).

## 1. Intenção

Hoje **Células › Genética › Grupos** junta células por semelhança do genoma
(98,5% dos bits iguais). Isso parece família, mas não é. A V5 grava quem é
filho de quem, e a aba **Grupos** é substituída por **Linhagens**: famílias de
verdade, com história.

## 2. O que já foi aprovado

- Cada célula guarda `parent_id` (a mãe), `root_lineage_id` (o fundador
  original) e `birth_tick`.
- Fundadores e células inseridas pelo mapa: sem mãe; são fundadoras de si
  mesmas.
- Um diário compacto de nascimentos (`mãe, filha, tick`) fica com a
  telemetria da sessão, sem pesar na simulação.

## 3. O problema que o desenho precisa resolver

Só a linhagem fundadora não basta. Depois de algumas horas, quase sempre
**um fundador vira ancestral de toda a população**. A aba mostraria "Linhagem
#17: 100%" e nada mais.

É preciso dividir uma linhagem em **ramos**, do jeito que a biologia faz com
espécies: um ramo novo nasce quando os descendentes se afastaram o
suficiente do ancestral do ramo.

### 3.1 Regra do ramo (proposta)

- Cada célula guarda também o seu `ramo`.
- Cada ramo guarda o genoma do seu **fundador de ramo**.
- Ao nascer, a filha herda o ramo da mãe. Mas, se o genoma dela já difere do
  genoma do fundador do ramo em mais de **18 bits** (o mesmo 98,5% usado
  hoje), ela **abre um ramo novo**, filho do ramo da mãe.
- O teste é feito só no nascimento, contra um único genoma, e custa uma
  comparação de 1.251 bits por nascimento. É desprezível.

Resultado: uma árvore de ramos que cresce devagar. Mutações isoladas não
abrem ramos; ramos novos nascem só quando o acúmulo de mutações num caminho
de descendência passa do limiar. Juntamos genealogia (quem descende de quem)
e genética (quanto mudou).

### 3.2 O que fica guardado

| Dado | Onde | Tamanho |
|---|---|---|
| `parent_id`, `root_lineage_id`, `ramo`, `birth_tick` | em cada célula viva | ~28 bytes por célula |
| Tabela de ramos: id, ramo-pai, tick de nascimento, genoma do fundador, tick de extinção, pico de população | no mundo, salva no checkpoint | ~200 bytes por ramo; mil ramos = 200 KB |
| Diário de nascimentos | telemetria da sessão | opcional, cresce com a run |

Os ramos extintos continuam na tabela (é a história), mas a tabela cresce só
quando nasce um ramo, não a cada nascimento.

## 4. Como apresentar: aba Células › Genética › Linhagens

Os seletores passam a ser **Resumo | Linhagens | Circuitos**.

### 4.1 Lista de ramos vivos (visão padrão)

Uma linha por ramo vivo, ordenada por população:

```text
●  Ramo 17.4  ·  312 células (41%)  ·  desde o tick 84.200
   linhagem 17 (Clássica) · ramo de 17.1 · geração 52–61 · ↑ crescendo
```

- **Cor:** a cor genética do fundador do ramo, então combina com a arena.
- **Nome:** `linhagem.ramo`, por exemplo `17.4` = quarto ramo da linhagem 17.
- **Tendência:** crescendo, estável ou encolhendo (comparando com a amostra
  anterior).
- **Clicar** abre o detalhe (4.3).

### 4.2 Árvore

Um botão alterna para a **árvore de ramos**:
- cada ramo é um nó, ligado ao ramo-pai;
- a espessura do nó é a população viva; ramos extintos ficam em cinza, finos,
  com o tick da extinção;
- ramos podem ser recolhidos, e por padrão aparecem só os ramos com
  descendentes vivos (com um botão "mostrar extintos").

### 4.3 Detalhe de um ramo

- População, participação, idade do ramo, gerações, nascimentos e mortes na
  janela.
- **O que mudou:** os circuitos (origem → destino) que este ramo tem e o
  ramo-pai não tinha, e os que perdeu. É a pergunta mais útil: "o que esse
  ramo inventou?".
- Botões:
  - **Destacar na arena:** apaga as outras células e mostra só as do ramo;
  - **Seguir uma célula do ramo:** abre uma no modo Indivíduo;
  - **Ver fundador do ramo:** mostra a rede do genoma fundador, mesmo que ele
    já tenha morrido.

### 4.4 Gráfico de rio na Demografia

Um gráfico de áreas empilhadas (*Muller plot*): o eixo X é o tempo, e cada
faixa colorida é um ramo, com a altura igual à participação dele na
população. Mostra de forma direta um ramo surgindo, tomando conta e outro
sumindo. É a visualização clássica de evolução.

Ele usa a mesma amostragem da Demografia (a cada 500 ticks). Ramos com menos
de 1% entram somados numa faixa "outros".

### 4.5 Modo Indivíduo

Na seção Resumo da célula:
- mãe (com botão **ir para a mãe**, se ela estiver viva), fundador e ramo;
- filhos vivos (lista curta, clicável).

## 5. O que sai

- A aba **Grupos** (agrupamento por semelhança) sai da interface.
- A similaridade de Hamming continua existindo no motor: ela é usada pela
  regra do ramo, pelo "mais parecida" do Indivíduo e pela coesão.
- O Resumo troca "grupos genéticos" por "ramos vivos" e "ramo dominante".

## 6. Saves

- O checkpoint sobe de versão com os campos novos e a tabela de ramos.
- Runs antigas: cada célula vira fundadora da própria linhagem, com o ramo
  `id.0`, e a genealogia começa a contar a partir do carregamento.

## 7. Testes

- A filha herda o ramo da mãe enquanto difere do fundador do ramo em até 18
  bits; com 19, abre um ramo novo, filho do ramo da mãe.
- A tabela de ramos marca a extinção quando o último membro morre.
- Ida e volta no save preserva genealogia e ramos.
- O mesmo hash para a mesma seed.

## 8. Perguntas em aberto

| # | Pergunta | Recomendação |
|---|---|---|
| L1 | Limiar para abrir ramo: 18 bits (98,5%), o mesmo dos grupos de hoje? | Sim, para começar. Depois de ver as árvores, dá para ajustar. |
| L2 | Tirar a aba Grupos de vez, ou deixar escondida por um tempo para comparar? | Tirar. A lógica continua no motor, se for preciso voltar. |
| L3 | O gráfico de rio entra na mesma entrega da aba ou depois? | Logo depois (entrega própria), porque depende de guardar a participação dos ramos em cada amostra. |
