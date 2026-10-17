# Ecossistema neural evolutivo — documentação preliminar

## Visão

O projeto será uma simulação 2D local de um ecossistema simples. Nele, organismos-célula vivem em um ambiente com alimento, consomem energia, se movem, morrem e se reproduzem.

Cada organismo terá uma pequena rede neural. Essa rede recebe percepções do ambiente e produz movimentos a cada tick da simulação. Não será programada uma estratégia como “vá até a comida”. Serão programadas apenas as regras do mundo, do corpo e da evolução. O comportamento útil deverá surgir por seleção natural.

O conceito central é **neuroevolução**: os genes codificam a estrutura e os pesos das conexões neurais. Ao se reproduzir, o organismo passa uma cópia mutável desses genes ao descendente. Redes que ajudam seus portadores a sobreviver e reproduzir tendem a permanecer na população.

## Princípios do escopo inicial

- Todos os organismos terão a mesma fisiologia e aparência.
- Não haverá evolução de tamanho, eficiência metabólica, órgãos ou fisionomia.
- O que evolui é exclusivamente o cérebro: conexões e pesos neurais.
- O mundo fornece comida e aplica custos de energia.
- A simulação deverá poder rodar acelerada por muitas gerações, além de poder ser visualizada normalmente.
- O programa será local, escrito em Rust; Macroquad será usado apenas para janela, desenho e controles.

## Regras do mundo

Cada organismo possui ao menos posição, direção/movimento e energia.

- Alimento aparece no ambiente.
- Existir e se mover consomem energia.
- Tocar alimento recupera energia.
- Sem energia, o organismo morre.
- Ao ultrapassar uma condição de energia, o organismo se reproduz e transfere parte dessa energia ao descendente.
- O descendente nasce com o genoma do progenitor, sujeito a mutações aleatórias.
- O ambiente terá um campo de corrente: uma célula e uma unidade de fitoplâncton sofrem arrasto conforme o vetor local do mapa.

Com isso, sobreviver por mais tempo e encontrar alimento gera mais oportunidades de reprodução. Essa é a pressão seletiva do sistema.

## Interface entre cérebro e mundo

Nem toda decisão precisa ser fixa, mas o cérebro precisa de uma interface para perceber o mundo e agir nele. Sem sensores, a rede não recebe informação; sem motores, ela não altera o ambiente.

Para o experimento atual com corrente, haverá 32 slots neurais:

| Grupo | Quantidade | Papel |
|---|---:|---|
| Sensores de alimento | 4 | Alimento à esquerda, direita, acima e abaixo |
| Sensores de corrente | 2 | Componentes `corrente_x` e `corrente_y` na posição da célula |
| Sensor de energia | 1 | Energia atual dividida pela capacidade máxima da célula |
| Contexto constante | 1 | Sinal fixo igual a `1`, equivalente a uma referência/bias disponível à rede |
| Internos livres | 16 | Duas camadas lógicas de capacidade interna; processamento e estado persistente, sem comportamento predeterminado |
| Internos temporais | 3 | Recursos de memória/temporização; não escolhem direção nem têm uma estratégia fixa |
| Motores | 5 | Movimento à esquerda, direita, acima e abaixo; decisão de reproduzir |

Os sensores e motores são a interface física fixa. Os dezesseis neurônios internos livres e os três temporais mantêm estado entre ticks; a evolução decide se eles serão usados, como serão conectados e com quais pesos. Os temporais dão à rede espaço explícito para formar atrasos, persistência, oscilação ou contagem, mas não codificam uma regra como “espere X ticks e vá para a direita”.

Cada sensor de alimento cobre um cone exclusivo de 90°. Para cada comida dentro do raio de 240 unidades, o mundo compara `|dx|` e `|dy|`: se o deslocamento horizontal predomina, ela pertence ao cone esquerdo ou direito; caso contrário, pertence ao cone de cima ou baixo. Empates diagonais pertencem ao cone horizontal. Portanto, uma comida alimenta no máximo um sensor no mesmo tick. Cada cone escolhe somente sua comida elegível mais próxima.

O alcance radial inicial é fixo em 240 unidades. Para cada sensor, se a menor distância euclidiana elegível for `d`, seu valor é `máximo(0, 1 − d ÷ 240)`; se não houver alimento no cone dentro do alcance, o valor é zero. Distância é medida entre os centros das entidades. O cérebro recebe apenas quatro valores escalares: não recebe ângulo, `dx` nem `dy`. No futuro, o alcance do sensor poderá evoluir por organismo, com custo energético crescente para sensores mais potentes; isso não faz parte da primeira fisiologia fixa.

O campo físico de corrente terá magnitude máxima de 10 unidades por tick. Os sensores de corrente recebem o vetor local normalizado por esse máximo: `corrente_x = fluxo_x ÷ 10` e `corrente_y = fluxo_y ÷ 10`, ambos entre `−1` e `+1`. No mapa, X positivo aponta para a direita e Y positivo aponta para baixo. Assim, um fluxo físico de `(+3, −7)` é entregue à rede como `(+0,3, −0,7)`. O sensor de energia recebe um valor de `0` a `1`, obtido por `energia atual ÷ 120`. Isso permite que a evolução forme comportamentos que nadam contra, a favor ou transversalmente ao fluxo local e ajustem esforço à própria reserva energética. O sinal constante permite que a rede evolua limiares e circuitos temporais sem precisar de uma estratégia comportamental embutida.

### Valores e dinâmica neural

Os quatro sensores de alimento e o sensor de energia variam de `0` a `1`; os dois de corrente variam de `−1` a `+1`; e o slot de contexto emite sempre `+1`. Eles são escritos pelo mundo e não preservam estado próprio.

Internos livres e temporais usam a sigmoide centralizada, evitando que uma rede sem conexões produza movimento ou gasto de energia por padrão:

```text
sinal(z) = 2 × sigmoide(z) − 1
```

Ela produz valores de `−1` a `+1` e vale zero quando a soma recebida `z` é zero. Para internos livres:

```text
estado próximo = sinal(soma dos sinais recebidos × pesos)
```

Os três temporais usam a mesma soma, mas se aproximam do resultado gradualmente:

```text
estado próximo = (1 − taxa) × estado anterior + taxa × sinal(z)
```

As taxas iniciais serão 25%, 8% e 2% por tick, fornecendo memória curta, média e longa sem programar uma estratégia. Todos os 19 internos leem os sensores do tick atual e estados internos salvos do tick anterior; suas próximas ativações são gravadas em buffer separado.

Os quatro motores de nado usam `máximo(0, sinal(z))`, portanto variam de `0` a `1`. Seu vetor de nado é a diferença entre motores opostos. Se a magnitude do vetor exceder 1, ela é normalizada antes de multiplicar pela velocidade própria máxima de 8. O custo energético, porém, continua sendo a soma dos quadrados das quatro ativações individuais. O quinto motor, `reproduzir`, pede divisão quando sua ativação chega a 0,60; o corpo só aceita o pedido acima do mínimo energético configurado e força a divisão no limite energético.

Comer permanece uma consequência física do contato. Reproduzir combina decisão neural e regra física: o motor pede a divisão, o mínimo energético permite e o limite a impõe.

## Genoma neural

O genoma será uma sequência binária composta por registros de conexões neurais. Uma conexão descreve uma sinapse com quatro informações:

- **Origem:** neurônio que envia o sinal.
- **Destino:** neurônio que recebe o sinal.
- **Peso:** influência do sinal; positivo excita e negativo inibe.
- **Ativo:** indica se a ligação existe ou está desligada.

Conceitualmente, um registro é:

```text
[origem] [destino] [peso] [ativo]
```

Os identificadores de slots são fixos e fazem parte do formato do genoma:

| IDs | Slots |
|---:|---|
| 0–3 | Alimento: esquerda, direita, acima, abaixo |
| 4–5 | Corrente: eixo X, eixo Y |
| 6 | Energia normalizada |
| 7 | Contexto constante `+1` |
| 8–23 | Dezesseis internos livres |
| 24–26 | Três internos temporais: rápido, médio, lento |
| 27–31 | Motores: esquerda, direita, cima, baixo e reproduzir |

Os 5 bits de identificador acomodam exatamente os 32 slots. O novo motor `reproduzir` usa o slot 31; portanto, o formato de cada registro e o tamanho de 1.088 bits do genoma não precisam aumentar.

Como existem 32 slots, identificá-los continua exigindo 5 bits por origem e 5 bits por destino. Cada peso usará 6 bits em complemento de dois: o inteiro vai de `−32` a `+31` e é dividido por 8, produzindo pesos de `−4,000` a `+3,875`, com resolução de `0,125`. Um peso positivo excita; um negativo inibe. Com o sinal da origem `a`, a contribuição da conexão é `a × peso`. O bit final informa se a conexão está ativa. Cada registro ocupa 17 bits.

Cada genoma terá 64 registros de conexão: 32 ativos na população inicial e 32 inativos como material genético disponível para mutações futuras. Como cada registro tem 17 bits, o genoma ocupa 1.088 bits, ou 136 bytes por organismo. A ampliação oferece uma segunda camada de capacidade interna sem alterar a interface física. Mutações poderão criar, remover, redirecionar, enfraquecer ou fortalecer ligações.

Na reprodução, o genoma é copiado e cada um de seus 1.088 bits tem probabilidade de `1/1024` de inverter. Isso produz, em média, 1,06 alterações por descendente: a maior parte dos filhos permanece muito parecida com o pai, mas variação contínua cria novos pesos, origens, destinos e estados ativos. Registros com identificadores inexistentes ou conexões proibidas não causam erro; são ignorados durante a execução e podem voltar a ser válidos por mutação futura. O filho herda apenas o genoma: todos os estados internos e temporais começam em zero.

A população inicial tem 20 células sem parentesco. Cada uma recebe 32 conexões ativas válidas, sorteadas sem repetição entre os caminhos permitidos, e 32 registros inativos com bits aleatórios. Pesos de conexões ativas iniciais ficam entre `−1` e `+1`, em passos de 0,125; a faixa completa fica disponível para a evolução. Isso evita que redes aleatórias já nasçam saturando os motores, sem introduzir um comportamento de busca de alimento programado.

Este limite é deliberado. Evoluir livremente o tamanho do genoma e a quantidade de neurônios continua sendo uma extensão desejada, mas a segunda camada interna já aumenta o espaço de busca sem introduzir genomas de comprimento variável.

## Como uma decisão acontece em um tick

Em cada tick:

1. O mundo mede alimento e corrente local e preenche os seis sensores; o contexto constante recebe `1`.
2. Cada conexão ativa transmite o sinal da origem multiplicado pelo seu peso.
3. Os 19 neurônios internos leem tanto os sensores atuais quanto seu próprio estado salvo do tick anterior.
4. Suas próximas ativações são calculadas em um segundo buffer e só substituem o estado anterior ao fim do cálculo. Isso torna conexões recorrentes bem definidas.
5. As ativações dos quatro motores de nado se tornam um vetor de deslocamento; o quinto motor pode pedir reprodução.
6. O deslocamento final combina vetor de nado e arrasto da corrente local.
7. O mundo atualiza posição, alimento, energia, morte e reprodução.

Uma conexão como `sensor de comida à esquerda → motor esquerdo`, com peso positivo alto, favorece movimento para a esquerda quando aquele sensor está ativo. Uma conexão com peso negativo reduz a ativação de seu destino. A evolução encontra essas relações por tentativa, mutação e seleção; elas não são escritas como regras comportamentais.

## Função de ativação

Após somar os sinais recebidos, um neurônio pode usar uma função sigmoide. Ela converte qualquer soma em uma ativação entre 0 e 1:

```text
sigmoide(−5) ≈ 0,007
sigmoide(0)  = 0,5
sigmoide(+5) ≈ 0,993
```

Isso evita forças sem limite e cria uma resposta suave: pesos positivos podem excitar e negativos podem inibir. Para movimento, a diferença entre os motores direito e esquerdo pode formar o deslocamento: se o motor esquerdo está muito mais ativo, o organismo vai para a esquerda.

No futuro, a função `tanh`, que produz valores entre −1 e +1, também pode ser avaliada para comandos motores. Ela não muda a ideia evolutiva.

## Corrente ambiental proposta

O primeiro modelo não será uma engine física de convecção. Será um campo vetorial procedural, contínuo e reproduzível.

O mapa será dividido em uma malha de controle. Cada região armazena um vetor de corrente, como `(2, −1)` em unidades por tick. No início de uma execução, o programa cria poucas regiões amplas de giro e deriva a partir de uma semente aleatória; cada uma tem centro, raio, intensidade e sentido. Os vetores dessas regiões são somados e o vetor resultante é limitado a magnitude 10. Ao consultar uma posição entre células da malha, o programa interpola os vetores vizinhos, evitando mudanças bruscas.

Assim, a corrente é aleatória na criação do mundo, mas suave e persistente ao longo dos ticks. Ela poderá mudar lentamente depois, movendo ou enfraquecendo essas regiões; não será um novo número aleatório independente a cada tick.

O arrasto é uma abstração controlada:

```text
velocidade da célula = nado neural + acoplamento_da_célula × corrente local
velocidade do alimento = acoplamento_do_alimento × corrente local + ruído mínimo
```

Os coeficientes de acoplamento representam quanto cada entidade acompanha o meio. Não vamos interpretá-los inicialmente como uma simulação exata de massa, peso ou densidade.

Temperatura, umidade, fontes de calor, obstáculos e convecção física ficam como extensão. Uma malha de temperatura por difusão poderá, mais adiante, alterar o crescimento e o tamanho do fitoplâncton, sem necessariamente alterar seu nascimento no tubo. Transformar temperatura em convecção fisicamente correta exigiria um solver de fluido com pressão e viscosidade, e não é requisito deste primeiro ambiente.

## Movimento, ticks e energia

O programa separará o tempo de simulação da taxa de desenho. A proposta inicial é executar 10 ticks por segundo (`1 tick = 0,1 segundo`) e desenhar a 60 FPS, interpolando visualmente posições entre ticks. A simulação acelerada executará muitos ticks sem desenho; a taxa de FPS nunca altera a física nem a evolução.

O primeiro exemplo usa um mapa unidimensional de 1000 unidades e velocidade própria máxima de 8 unidades por tick. A rede gera ativações motoras entre 0 e 1. No eixo horizontal:

```text
velocidade própria = 8 × (motor direito − motor esquerdo)
velocidade final   = velocidade própria + acoplamento × corrente local
posição seguinte   = posição atual + velocidade final
```

O acoplamento à corrente é igual para todas as células neste escopo. Não será modelada massa ou inércia física real; o modelo representa arrasto em um meio viscoso.

A energia cobrada pelo movimento depende das ativações motoras, e não do deslocamento final:

```text
energia seguinte = energia atual
  − custo basal
  − 0,20 × soma dos quadrados das ativações motoras
  + energia do alimento consumido
```

Isso cobra desperdício: dois motores opostos podem se anular em velocidade, mas ainda consomem energia. Em água favorável, uma rede pode reduzir a ativação do motor e manter deslocamento com custo menor. Em uma corrente contrária, uma célula pode gastar muito e quase não avançar.

Parâmetros iniciais aprovados:

| Parâmetro | Valor | Significado |
|---|---:|---|
| Capacidade de energia | 120 | Limite máximo armazenável por organismo |
| Energia ao nascer | 60 | Energia inicial de pai e filho após reprodução |
| Custo basal | 0,03/tick | Custo de existir, mesmo parado |
| Custo de motor plenamente ativo | 0,20/tick | Custo de um motor com ativação `1` |
| Energia por fitoplâncton | 50 | Energia recuperada ao consumir uma unidade |
| Reprodução | mínimo → decisão neural; limite → divisão forçada | Entre os dois valores, o motor `reproduzir` precisa atingir 0,60; no limite, pai e filho ficam com metade da energia atual |

Em uma travessia de referência de 213 ticks com um único motor a `1`, o custo de nado é 42,60 e o custo basal é 6,39: total de aproximadamente 49 unidades de energia. Uma célula que nasce com 60 atravessa, mas termina com energia baixa; para reproduzir, precisa obter alimento e evitar desperdício.

Uma faixa de corrente estática adversa cuja magnitude seja maior ou igual à velocidade própria máxima é uma barreira intransponível em 1D. Em 2D, o organismo poderá contorná-la. No primeiro mapa 1D, correntes adversas devem permanecer abaixo de 8 para preservar travessias possíveis.

## Entrada e dispersão de fitoplâncton

O fitoplâncton não nascerá aleatoriamente em qualquer ponto do mapa. O mundo terá um tubo de entrada lateral, visível na simulação, que injeta unidades de fitoplâncton no ambiente.

Cada unidade nasce em uma posição aleatória dentro da abertura do tubo e imediatamente sofre arrasto da corrente local. A corrente ambiental deverá apontar para dentro do mapa na saída do tubo e se dispersar em seguida; assim, a distribuição de alimento é consequência do fluxo, e não de um sorteio global de posições.

Uma unidade de fitoplâncton é uma entidade circular visível, menor que uma célula. Inicialmente, o raio da célula será 14 unidades e o raio do fitoplâncton será 5 unidades. O alimento guarda posição, energia de alimento, raio, idade e acoplamento à corrente. Ela desaparece quando é consumida ou ao atingir sua vida útil. Inicialmente, essa vida útil será de 600 ticks (60 segundos de simulação), tempo suficiente para a corrente dispersá-la pelo ambiente. Ela não desaparece ao alcançar bordas. No futuro, temperatura poderá afetar seu crescimento e tamanho.

O consumo é uma colisão entre círculos:

```text
distância²(centro da célula, centro do fitoplâncton)
  ≤ (raio da célula + raio do fitoplâncton)²
```

O fitoplâncton é removido e a energia é entregue à célula após o movimento daquele tick. Se duas células alcançarem a mesma unidade no mesmo tick, vence a que estiver mais próxima; em empate exato, vence a célula de menor identificador. Essa regra mantém a execução reprodutível.

A taxa de emissão do tubo e as dimensões da abertura ainda precisam ser definidas. A vida útil cria um teto natural aproximado: sem consumo, uma emissão de `r` unidades por tick mantém aproximadamente `600 × r` unidades no mapa depois da estabilização. O consumo pelas células reduz essa quantidade.

## Cena inicial do ecossistema

A referência visual é uma câmara retangular escura, em proporção 16:9, com um tubo vermelho na lateral esquerda aproximadamente no meio da altura. Para não misturar resolução de tela com física, o mundo usará coordenadas lógicas de `1600 × 900` unidades; a janela inicial poderá desenhá-lo em `1280 × 720` pixels, apenas aplicando escala.

| Elemento | Decisão inicial |
|---|---|
| Boca do tubo | Centro em `(120, 450)`, abertura vertical de 80 unidades |
| População inicial | 20 células, distribuídas aleatoriamente pela câmara |
| Fitoplâncton inicial | 12 unidades já saindo da boca do tubo |
| Emissão contínua | 0,12 unidade por tick, ou 1 unidade a cada 8 a 9 ticks em média |
| Vida útil do alimento | 600 ticks |
| Estoque natural sem consumo | Aproximadamente 72 unidades após estabilização |

A emissão equivale a cerca de 6 unidades de energia por tick (`0,12 × 50`). Para 20 células, é suficiente para manter um ecossistema inicial, mas insuficiente para que toda célula nade no máximo, coma facilmente e se reproduza sem eficiência. Os valores serão parâmetros de configuração, não constantes enterradas no código.

Como alimento não sai do mundo ao tocar uma borda, a corrente inicial será recirculante: sai da boca do tubo para a direita pelo centro da câmara, curva nas proximidades da lateral direita e retorna mais lentamente pelas regiões superior e inferior. Giros locais podem perturbar esse circuito. Assim, o alimento forma uma pluma ao sair do tubo, percorre o ambiente e continua circulando até ser consumido ou completar sua vida útil.

As bordas da câmara serão paredes sólidas. Células e fitoplânctons permanecem dentro do mapa; se um deslocamento tentá-los levar para fora, sua posição é limitada à borda. Uma célula ainda paga o custo dos motores que ativou, mesmo que a parede impeça seu avanço.

## Ordem completa de um tick

Cada tick representa um instante atômico do mundo. Células não ganham vantagem por serem calculadas antes de outras.

1. Atualizar o relógio e, no futuro, qualquer variação lenta da corrente ambiental.
2. Atualizar o crédito do tubo e emitir fitoplâncton quando houver crédito suficiente.
3. Tirar uma fotografia lógica do estado atual: posições, alimento e corrente.
4. Calcular os sensores de todas as células a partir dessa fotografia.
5. Executar as redes neurais de todas as células usando seus estados internos anteriores.
6. Calcular intenções de deslocamento: nado neural mais arrasto para células; arrasto para fitoplâncton.
7. Aplicar todos os deslocamentos simultaneamente e limitar entidades às paredes da câmara.
8. Detectar colisões célula–fitoplâncton após o movimento e atribuir cada alimento a uma única vencedora.
9. Calcular, para cada célula, energia anterior menos custos mais alimento consumido. Remover as que terminarem com energia zero ou menor.
10. Reproduzir células que chegarem à capacidade energética: pai e filho recebem 60 de energia; o filho recebe genoma mutado e entra somente no próximo tick.
11. Aumentar a idade do fitoplâncton não consumido e remover o que atingir 600 ticks.
12. Registrar métricas do estado final do tick.

O uso de fotografia no passo 3 e de aplicação simultânea no passo 7 torna a simulação determinística para uma mesma semente aleatória. Em particular, uma célula que encontra alimento no mesmo tick em que sua energia ficaria baixa pode recebê-lo antes de a morte ser verificada.

## Interface de configuração da sessão

A visualização terá uma aba lateral de configuração. Ela permitirá ajustar os parâmetros de uma sessão antes de iniciá-la e criar, carregar, renomear e salvar presets locais.

Parâmetros de mundo — como semente aleatória, população inicial, emissão e vida útil do fitoplâncton, intensidade da corrente, custos de energia e velocidade de execução — serão editados no painel e aplicados por uma ação explícita de reiniciar a sessão. Isso impede que uma experiência perca reprodutibilidade por alterações silenciosas no meio da execução. Pausa, velocidade visual e exibição de camadas serão controles imediatos da interface.

## O que não será decidido agora

- Aprendizado por backpropagation: não é necessário para esta proposta.
- Evolução fisiológica ou visual dos organismos.
- Genoma de tamanho variável e criação de novos neurônios.
- Temperatura, umidade, fontes de calor, obstáculos e convecção física.

## Próxima etapa

Fechar o formato exato do genoma, regras de mutação, dinâmica dos neurônios temporais, ordem do tick, modelo de energia, reprodução e limites do ambiente.
