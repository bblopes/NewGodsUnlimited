# Sessões reproduzíveis e telemetria manual

## Objetivo

Uma sessão manual boa não pode virar apenas uma memória visual. Ao encerrar e salvar, o Ecossistema Neural deverá gerar um arquivo capaz de responder:

- com quais condições o mundo começou;
- em qual tick cada ajuste foi efetivamente aplicado;
- como população, alimento, energia e diversidade mudaram;
- como retomar exatamente daquele ponto;
- como rodar novamente a mesma experiência desde o tick zero.

O Laboratório não será alterado nesta etapa. O arquivo será preparado para que, no futuro, ele possa abrir uma sessão salva e criar variações controladas dela.

## Unidade salva: uma run

Uma **run** começa ao criar ou reiniciar a sessão e termina quando o usuário a salva/encerra. Um reinício não deve sobrescrever uma run anterior: ele inicia outra run com identificador próprio.

O arquivo terá extensão `.ecosrun`. Internamente, será um pacote versionado com estes blocos lógicos:

| Bloco | Conteúdo | Função |
|---|---|---|
| Manifesto | versão do formato, versão do motor e resumo | identifica como ler a run |
| Condições iniciais | seed e todos os parâmetros no tick 0 | reproduz o começo |
| Eventos | alterações aplicadas, com tick exato | reproduz o fine-tuning manual |
| Telemetria | séries de dados ao longo da run | análise posterior |
| Checkpoints | estados completos em pontos do tempo | retoma a run sem recalcular tudo |
| Encerramento | tick final e resumo agregado | identifica o resultado salvo |

## Condições iniciais

O manifesto inicial registra tudo que influencia a ecologia:

- seed do mundo;
- número inicial de células;
- emissão de fitoplâncton por tick;
- vida do fitoplâncton;
- energia entregue por fitoplâncton;
- mínimo de energia para reprodução voluntária;
- limite obrigatório de reprodução;
- custo mental por conexão/tick;
- corrente ligada/desligada e intensidade;
- anti-cramming, força e espaço mínimo;
- alcance e geometria dos sensores (`raio 240`, cones de 90°);
- versão do formato do genoma, número de slots e regras de mutação;
- versão exata do motor de simulação.

Velocidade visual e pausa também podem ser registradas como eventos de interface, mas não alteram o resultado ecológico. Elas não são necessárias para reproduzir o mundo.

## Eventos: o diário do fine-tuning

Uma edição de campo não é um evento. O evento acontece apenas quando o usuário pressiona **Aplicar** ou **Reiniciar sessão** e o worker do motor aceita a mudança.

O tick salvo é o tick do mundo no instante em que o comando é drenado pelo motor, não o instante em que o usuário começou a digitar. Isso elimina ambiguidade entre interface e simulação.

Cada evento terá esta forma conceitual:

```text
tick: 184_250
tipo: parametros_aplicados
origem: manual
antes: { emissao: 0.80, vida_fito: 1800, custo_mental: 0.0025 }
depois: { emissao: 0.12, vida_fito: 900, custo_mental: 0.0040 }
```

Tipos necessários:

| Tipo | Quando ocorre | Dados |
|---|---|---|
| `parametros_aplicados` | botão Aplicar | apenas valores que mudaram, antes e depois |
| `sessao_reiniciada` | botão Reiniciar sessão | novas condições iniciais e tick da run anterior |
| `pausada` / `retomada` | controle de execução | tick e estado da interface |
| `velocidade_alterada` | 1× a 100000× | tick e multiplicador; apenas auditoria |
| `checkpoint_criado` | automático ou manual | tick e identificador do checkpoint |
| `run_salva` | usuário encerra/salva | tick final e nome do arquivo |

O evento deve registrar, entre outros, emissão, vida/energia do fitoplâncton, mínimo/limite reprodutivo, custo mental, corrente e anti-cramming. Assim será possível localizar frases como: “a diversidade caiu logo depois que reduzi a emissão no tick 184250”.

## Telemetria

Há duas cadências, para equilibrar leitura analítica e tamanho de arquivo.

### Série por tick

Estes dados devem ser registrados em todo tick, pois são pequenos e permitem correlacionar mortes e escassez sem interpolação:

| Dado | Uso |
|---|---|
| tick | eixo de tempo exato |
| células vivas | população instantânea |
| nascimentos no tick | taxa real de nascimento |
| mortes no tick | taxa real de morte |
| fitoplânctons disponíveis | estoque de comida no mapa |
| fitoplânctons emitidos | oferta criada naquele tick |
| fitoplânctons consumidos | pressão alimentar efetiva |
| fitoplânctons expirados | perda por vida útil |
| energia potencial por alimento | energia configurada naquele tick |

Com isso, quando houver um pico de mortes, será possível ver a disponibilidade de alimento exatamente naquele tick e nos ticks anteriores.

### Amostra analítica

Os dados mais caros são agregados a cada 250 ticks por padrão — intervalo que poderá ser configurável. Cada amostra contém:

- tick;
- população viva, nascimentos e mortes acumulados;
- idade média e distribuição de idade;
- energia média e histograma de energia;
- geração média, máxima e histograma das gerações vivas;
- conexões neurais médias;
- diversidade genética global;
- tamanho da linhagem dominante e participação dela na população;
- frequência de cada neurônio e conexão nas células vivas;
- estoque de fitoplâncton e saldo alimentar no intervalo;
- parâmetros ecológicos efetivos naquele momento.

A telemetria por tick responde “o que havia no momento da morte”. A amostra analítica responde “qual linhagem dominou e quando a diversidade desabou”.

## Checkpoint para continuar uma run

Reproduzir desde o início e continuar de onde parou são necessidades diferentes. Para continuar uma sessão, o checkpoint deve guardar o estado completo do mundo:

- tick, contadores de nascimentos e mortes e crédito fracionário de emissão;
- estado do gerador pseudoaleatório;
- lista de fitoplânctons: posição e idade;
- lista de células: id, posição, energia, idade, geração, filhos produzidos e genoma;
- estado dinâmico de cada cérebro: internos, temporizadores, últimas ativações e sinais de conexões;
- configuração ambiental em vigor;
- estado do anti-cramming;
- próximo identificador de organismo.

Checkpoints automáticos a cada 50 mil ticks são um ponto inicial razoável. Também haverá um checkpoint final obrigatório ao salvar. O usuário poderá retomar o checkpoint final sem perder a continuidade genética nem o estado dos timers neurais.

## Dois modos de reprodução

| Modo | Entrada | Resultado |
|---|---|---|
| Reexecutar | condições iniciais + sequência de eventos | roda do tick 0 e refaz a história ecológica |
| Retomar | checkpoint + eventos posteriores | continua do tick salvo sem recalcular o passado |

A reexecução é determinística dentro da mesma versão do motor e mesma arquitetura de execução. O manifesto precisa validar a versão antes de prometer igualdade de resultado. Checkpoints continuam úteis mesmo se versões futuras mudarem a física, porque preservam uma fotografia explícita do mundo antigo.

## Dados que não devem ser salvos por organismo em todo tick

Não será salvo um histórico completo de posição, energia e cérebro de cada célula em cada tick. Isso explode o arquivo e não é necessário para reproduzir a run.

O estado por organismo existe nos checkpoints; entre eles, a simulação é reconstruída deterministicamente. Para análise, ficam as séries agregadas e os eventos.

## Fluxo de interface futuro

Na sessão normal:

1. O usuário conduz a ecologia e aplica alterações.
2. Cada aplicação cria um evento com tick exato.
3. A telemetria e checkpoints são acumulados em memória/arquivo temporário.
4. Ao clicar em **Salvar run**, o aplicativo pede nome e grava um `.ecosrun`.
5. O resumo final mostra tick final, população, diversidade e quantidade de eventos manuais.

No futuro, o Laboratório poderá oferecer “Importar `.ecosrun`”, escolher um checkpoint ou reexecutar a agenda de eventos. Só depois acrescentaremos variações controladas, como executar dez cópias mudando levemente emissão, corrente ou seed.

## Critérios de aceite da futura implementação

- Uma alteração de emissão aplicada no tick `T` aparece no arquivo com valor anterior e posterior.
- Um gráfico de disponibilidade de fitoplâncton permite localizar o valor do tick de uma morte coletiva.
- Uma run salva pode ser reexecutada do tick zero com a mesma seed, eventos e versão de motor.
- Um checkpoint final pode ser aberto e continuar sem reiniciar os timers neurais ou a sequência aleatória.
- O arquivo deixa explícito quando uma versão do motor é incompatível com reprodução bit a bit.
