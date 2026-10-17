# Próxima subida de produção — escopo aprovado e backlog histórico

Este documento separa o que já está autorizado para a próxima entrega do que
permanece como proposta. Um item só é marcado como publicado depois de passar
por implementação, testes e geração do executável principal.

## Subida de produção em preparo

### Sensor de alimento com teto suave de alcance

- Aplicar a curva validada: mantém o crescimento forte para células pequenas e
  médias, mas troca o crescimento cúbico ilimitado por uma aproximação suave a
  **600 unidades**.
- O gene continua hereditário e o tamanho continua contribuindo para o alcance;
  o teto impede que uma célula enxergue todo o mapa.
- Atualizar os testes e os valores exibidos no painel do indivíduo.

**Estado:** implementado e validado por testes; aguardando a geração do
executável e a publicação.

### Vida do fitoplâncton sob iluminação

- O slider de vida do fitoplâncton passa a significar a sua vida-base, fora do
  sol.
- Cada partícula envelhece em todos os ticks. Sob sol, ela recebe inicialmente
  um multiplicador fixo de longevidade de **1,5×**; portanto, **não existe mais
  imortalidade em região ensolarada**.
- A reprodução sob luz continua sendo regulada pelo slider próprio de
  multiplicação. Vida e reprodução permanecem controles independentes.
- O texto da interface deixará explícito: vida-base sem sol e bônus de vida
  sob sol.

**Estado:** implementado e validado por testes; aguardando a geração do
executável e a publicação.

### Inserir indivíduo pelo mapa

**Objetivo:** criar um indivíduo controlado pelo usuário sem recomeçar a run.

Na aba de configurações, abaixo do seletor de **configuração neural**, haverá
um botão **Inserir célula**. A configuração selecionada — **Clássico**,
**Motor V1** ou **Motor V2** — determina o grupo e o genoma inicial do novo
indivíduo.

Fluxo proposto:

1. O usuário escolhe a configuração neural desejada.
2. Clica em **Inserir célula**; a aplicação entra em modo de posicionamento.
3. Um clique válido no mapa cria uma única célula naquele ponto, pertencente ao
   grupo selecionado.
4. A inserção retorna ao modo normal, sem alterar os demais organismos da run.

Critérios de implementação:

- não permitir criação dentro de parede, fora do mundo ou sobre uma posição
  fisicamente ocupada;
- indicar visualmente que o mapa está aguardando o ponto de inserção;
- preservar o grupo selecionado no momento do clique, mesmo se a simulação
  estiver pausada;
- salvar o mundo resultante no checkpoint, para que a população inserida seja
  restaurada ao carregar a sessão.

**Estado:** implementado e validado por testes; aguardando a geração do
executável e a publicação.

### Controles de sessão já solicitados

- Concluir o checkbox para manter o Windows ativo enquanto a simulação roda.
- Restaurar o modal de salvar o estado da sessão no executável padrão, para que
  uma run possa ser enviada para análise.

**Estado:** implementado; a validação final ocorrerá na entrega Windows.

## Backlog após esta subida

### Reorganização da aba de indivíduo

Reorganizar a aba **Células / indivíduo** em grupos expansíveis, para mostrar
mais informações sem esconder o essencial. A nova visão deve incluir ao menos
raio e custo do sensor de alimento, energia, tamanho, velocidade, fisiologia,
genoma e topologia neural. Este item foi explicitamente adiado nesta subida.

### Visão de alimento e quimiorrecepção

Refatorar os quatro sensores atualmente chamados de alimento para que passem a
ser **visão direcional genérica**: cada direção produz sinal positivo ao detectar
qualquer entidade visível, não apenas fitoplâncton. A iluminação local limitará
o alcance dessa visão.

A identificação e a navegação até alimento deixam de depender da visão. O
fitoplâncton deverá emitir um feromônio/campo químico alimentar próprio, e novos
sensores neurais deverão expor à célula a concentração e os vetores de gradiente
desse campo. Assim, ela poderá seguir o gradiente até a comida inclusive no
escuro, enquanto a visão continuará servindo para perceber presença e direção
de entidades no ambiente.

Antes de implementar, é preciso definir os novos slots neurais, custos de
emissão e leitura, intensidade/difusão/decaimento do campo, a regra para
combinar concentração com vetor gradiente e a migração dos genomas existentes.
A especificação de iluminação correlata permanece em
[Luz solar e fitoplâncton](LUZ_SOLAR_E_FITOPLANCTON.md#backlog-visão-de-alimento-e-quimiorrecepção).

### Iluminação contínua, temperatura e correntes

Evoluir a máscara solar binária para intensidade contínua; depois derivar a
temperatura do mapa, o custo térmico das células e, por fim, os efeitos nas
correntes ambientais.

### Telemetria demográfica de mudanças

Além da linha de mínimo reprodutivo, registrar no gráfico os instantes em que
uma configuração foi aplicada, de modo que uma mudança possa ser comparada à
resposta da população.

## Histórico relevante já incorporado

- Genoma hereditário passou a carregar o alcance do sensor e a atividade dos
  neurônios.
- Tamanho corporal já afeta raio físico, velocidade, captura, emissão química,
  custo basal e eficiência do sensor.
- Luz solar já existe como camada binária do mapa e habilita reprodução local
  do fitoplâncton.
- O executável já oferece nascimento por abiogênese, multiplicação sob luz e
  energia por unidade de fitoplâncton como controles separados.
- A próxima subida substitui a exceção de longevidade infinita sob sol pela
  regra de bônus de vida descrita acima.
