# Estrutura de arquivos proposta — antes da implementação

> Este documento é uma proposta de organização. Ele ainda não define funções, assinaturas ou valores numéricos. A ideia é primeiro separar responsabilidades e dependências; depois definiremos o que cada arquivo expõe.

## Base já decidida

- Um único programa local em Rust.
- Macroquad serve somente para janela, desenho e entrada de teclado.
- O núcleo da simulação não depende de Macroquad nem de uma janela.
- A interface neural terá 31 slots fixos: 4 sensores de alimento, 2 sensores de corrente, 1 sensor de energia, 1 contexto constante, 16 internos livres, 3 internos temporais e 4 motores.
- O genoma descreve conexões neurais; a fisiologia é igual para todos.

> A documentação preliminar foi atualizada para remover o modelo anterior de 10 slots e direção corporal. Esta estrutura considera o modelo atual de 31 slots.

## Princípio de separação

O programa terá três camadas, com uma única direção de dependência:

```text
app                 monta e controla a execução
  ├── simulation     calcula o ecossistema; funciona com ou sem tela
  ├── metrics        observa resultados da simulação
  └── viewer         lê o estado e o desenha; não altera regras biológicas
```

Isso é o que permite rodar milhões de ticks sem renderizar. O `viewer` não contém organismos, seleção ou rede neural; ele só mostra o estado que a simulação já calculou.

## Árvore proposta

```text
ECOSSYSTEM/
├── Cargo.toml
├── docs/
│   ├── DOCUMENTACAO_PRELIMINAR.md
│   └── ESTRUTURA_DE_ARQUIVOS_PROPOSTA.md
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── config.rs
│   ├── metrics.rs
│   ├── simulation/
│   │   ├── mod.rs
│   │   ├── geometry.rs
│   │   ├── environment/
│   │   │   ├── mod.rs
│   │   │   └── flow.rs
│   │   ├── world.rs
│   │   ├── tick.rs
│   │   ├── organism.rs
│   │   ├── food.rs
│   │   ├── sensing.rs
│   │   ├── brain.rs
│   │   ├── genome.rs
│   │   └── evolution.rs
│   └── viewer/
│       ├── mod.rs
│       ├── render.rs
│       ├── controls.rs
│       └── session_panel.rs
└── tests/
    └── simulation_rules.rs
```

## Responsabilidade de cada parte

| Arquivo ou pasta | Responsabilidade | Não deve fazer |
|---|---|---|
| `Cargo.toml` | Declara que o projeto é Rust e quais bibliotecas são usadas. | Conter regra da simulação. |
| `main.rs` | Ponto de entrada mínimo: inicia a aplicação. | Concentrar loop, mundo ou regras. |
| `app.rs` | Junta configuração, simulação, métricas e — quando houver — visualização. Decide se a execução é visual ou acelerada. | Calcular genoma, neurônio ou regra de alimentação. |
| `config.rs` | Reúne os parâmetros ajustáveis: dimensões do mapa, custos, população inicial, taxa de mutação e afins. | Guardar o estado vivo do mundo. |
| `metrics.rs` | Registra observações: população, energia média, comida consumida, geração e desempenho. | Influenciar decisões dos organismos. |
| `simulation/` | Núcleo determinístico do ecossistema. Deve poder ser testado e executado sem Macroquad. | Desenhar tela ou ler teclado. |
| `geometry.rs` | Tipos e operações geométricas simples e independentes da biblioteca gráfica. | Conhecer organismo, alimento ou genoma. |
| `environment/flow.rs` | Mantém o campo vetorial de corrente e amostra o vetor em uma posição do mapa. | Decidir movimento neural, reprodução ou criação física de calor. |
| `world.rs` | Guarda o estado global em um instante: tick atual, organismos, alimentos e gerador aleatório com semente. | Virar uma coleção de todas as regras detalhadas. |
| `tick.rs` | Define a ordem única das etapas de um tick e coordena os subsistemas. | Armazenar permanentemente entidades. |
| `organism.rs` | Define os dados próprios de um organismo: posição, energia, genoma e estado neural necessário. | Decidir como nasce comida ou como a tela o desenha. |
| `food.rs` | Define a representação de uma unidade de alimento. | Definir percepção ou reprodução. |
| `sensing.rs` | Converte estado espacial do mundo em quatro valores sensoriais para cada organismo. | Escolher movimento. |
| `brain.rs` | Executa a rede decodificada e transforma sensores em quatro ativações motoras. | Alterar energia, posição, comida ou população diretamente. |
| `genome.rs` | Representa o genoma binário, decodifica registros de conexão e valida seus campos. | Escolher quais organismos sobrevivem. |
| `evolution.rs` | Aplica herança e mutação quando a regra de reprodução determina que haverá descendente. | Desenhar ou medir o mundo. |
| `viewer/` | Camada opcional da tela. | Ser necessária para a simulação existir. |
| `render.rs` | Lê uma visão do estado e desenha mapa, organismos, alimento e indicadores. | Escrever diretamente no estado do mundo. |
| `controls.rs` | Converte teclas em comandos da aplicação, como pausar ou alterar a velocidade visual. | Mudar regras internas de seleção a cada tecla. |
| `session_panel.rs` | Mostra e edita a configuração de uma nova sessão, além de criar, carregar e salvar presets locais. | Alterar parâmetros científicos em silêncio durante uma execução já iniciada. |
| `tests/` | Testes de integração das regras observáveis do núcleo. | Depender de janela gráfica. |

Os dois arquivos que evitam a `main.rs` gigante são deliberados:

- `main.rs` apenas entra no programa;
- `app.rs` é a camada de coordenação. Ela conhece os módulos, mas não carrega a lógica científica deles.

## Fluxo de chamadas pretendido

```text
main.rs
  → app.rs
      → cria Config e World
      → a cada iteração, pede um tick à simulation/tick.rs
          → sensing.rs
          → brain.rs
          → movement / food / energy no estado do mundo
          → evolution.rs, quando houver reprodução
              → genome.rs
      → metrics.rs registra a observação
      → render.rs mostra uma leitura do estado, somente no modo visual
```

`mod.rs` não é um arquivo de dados nem um executável. Em Rust, ele declara os arquivos de uma pasta como módulos e controla quais tipos podem ser usados de fora dela. Ele serve como a porta da pasta `simulation` ou `viewer`.

## Informações que ainda faltam para fechar as funções

Estas lacunas não são detalhes cosméticos; elas determinam as interfaces entre os módulos. Ainda não devemos escondê-las em código.

1. **Escala dos sensores.** Cada sensor de alimento consultará o item mais próximo em sua direção; ainda precisamos fechar raio, fórmula de proximidade e o tratamento de itens no limite direcional. Os sensores de corrente serão vetores assinados normalizados.
2. **Dinâmica temporal.** Conexões entre internos podem formar ciclos. Está definido que internos preservam estado entre ticks e que haverá buffers de estado anterior/próximo; ainda falta definir a dinâmica específica dos três internos temporais.
3. **Combinação dos quatro motores.** Se esquerdo e direito ativarem juntos, eles se anulam, um vence, ou ambos geram custo? O mesmo vale para cima e abaixo. Essa definição é a ponte entre `brain.rs` e movimento.
4. **Geometria e bordas.** Posições contínuas ou células de grade; borda sólida, mundo que envolve, ou colisão; raio para comer. Isso determina `geometry.rs`, `sensing.rs` e custo de busca de alimento.
5. **Economia do mundo.** Energia inicial, custo basal, custo de movimento, ganho ao comer, regra de morte, limiar e custo da reprodução, posição de nascimento e limite de população.
6. **Genoma exato.** Quantidade máxima de registros, distribuição inicial, codificação do peso, como tratar identificadores de slot inválidos e quais mutações são possíveis em cada nascimento.
7. **Ordem física do tick.** Por exemplo: perceber → decidir → mover → comer → gastar energia → morrer → reproduzir → nascer comida. Trocar essa ordem altera a seleção e precisa ser escolha consciente.
8. **Reprodutibilidade.** Uma semente inicial deve permitir repetir uma simulação. Isso é essencial para comparar uma alteração de regra contra outra e para testar bugs.
9. **Métricas de sucesso.** Antes de chamar algo de evolução, definiremos o que observar: sobrevivência, alimento por tick, descendentes, diversidade genômica e desempenho em cenários repetidos.
10. **Corrente ambiental.** O primeiro campo será procedural, suave e persistente. Ainda faltam dimensão da malha, número de regiões de giro, intensidade e a relação entre arrasto e velocidade de nado.

## O que esta estrutura já suporta ao escalar

- **Modo acelerado:** `app.rs` deixa de chamar `viewer` e executa lotes de ticks.
- **Mais organismos:** a percepção poderá trocar sua busca simples por um índice espacial sem reescrever genoma, cérebro ou evolução.
- **Salvar experimentos:** a futura persistência pode ler dados de `world` e `metrics`, sem entrar no loop neural.
- **Mais sensores ou ações:** alteramos a interface declarada do cérebro e seus testes, não a renderização nem o mecanismo de herança.

O primeiro código deve ser deliberadamente pequeno, mas a fronteira entre módulos já precisa estar correta. A partir daqui, o próximo passo é definir as funções públicas e os dados de cada arquivo, começando pela cadeia mínima `Config → World → um tick`.
