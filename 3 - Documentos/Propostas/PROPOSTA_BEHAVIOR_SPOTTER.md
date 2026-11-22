# Proposta: Behavior Spotter (detector de fagocitose)

**Estado: implementado na V5.0.2 (22/11/2026).**
Base de código lida: `main` @ `6ccbadc` (V4.0.0).
Plano geral: [V5_FUNCOES_NOVAS.md](../Backlog/V5_FUNCOES_NOVAS.md).

## 1. Intenção

Achar e mostrar um comportamento enquanto ele acontece, para demonstrar a
simulação a alguém e para assistir. Nesta primeira versão há **um único
detector: fagocitose em curso**, porque é o comportamento que você ainda não
conseguiu ver direito ao vivo.

Fora desta versão (decidido): outros detectores, salvar o momento,
checkpoint e replay. Ficam para quando o Laboratório estiver dentro do app.

## 2. Onde fica

Uma aba nova na lateral, **Behavior Spotter**, logo abaixo de **Camadas**.

## 3. Comportamento (decidido em 22/11/2026)

A seção **Fagocitose** da aba tem duas partes, que funcionam juntas ou
separadas.

### 3.1 Lista ao vivo (sempre disponível)

- A aba mostra sempre as fagocitoses que estão acontecendo agora, com ou sem
  pausa.
- Cada linha tem **Ir até**: a arena centraliza no ponto de contato, dá zoom e
  passa a **rastrear o evento**. A câmera segue o predador e o círculo
  vermelho acompanha enquanto a puxada durar.
- O botão **Próxima** pula para a ocorrência seguinte da lista.
- O rastreio continua quando a simulação está rodando. Para ver devagar, você
  mesmo baixa a velocidade para 1×. Não há botão de avançar 1 tick.

### 3.2 Opção "Pausar ao encontrar"

É para quando você está rodando e não vê nada acontecer.

1. Com a opção ligada e a simulação rodando, no **primeiro tick** em que uma
   célula começa a puxar outra de verdade, a simulação **pausa sozinha**.
2. Aparece um **círculo vermelho** em cada fagocitose ativa naquele momento.
   O centro do círculo é o **ponto de contato** entre as duas células.
3. A arena já aponta para a primeira ocorrência.
4. Enquanto a opção estiver ligada, a simulação **fica pausada**.
5. Ao **desligar** a opção, a simulação **volta a rodar sozinha** e a câmera
   continua seguindo o predador da ocorrência escolhida.
6. Se você ligar a opção com a simulação já pausada, ela **não despausa**:
   apenas aponta para a primeira ocorrência que já existir naquele tick.

### 3.3 O que conta como "fagocitose em curso"

É o par em que o motor já aplica força de puxar neste tick, com as mesmas
condições do código:
- o motor `Fa` do predador está ativo;
- a presa está dentro do alcance da tentativa (`raio_A + raio_B + 15%` do
  menor raio);
- o predador tem pelo menos 1,10× o raio da presa. Abaixo disso, a tentativa
  custa energia mas não puxa nada, então não conta.

O motor já calcula esses pares em `apply_phagocytosis`. Só falta guardá-los
numa lista do tick (predador, presa, ponto de contato, força). O custo é
praticamente zero.

### 3.4 Ponto de contato

No segmento entre os dois centros, o ponto médio da região onde as duas
células se encostam:

```text
u = direção do predador para a presa
ponto = centro_predador + u × (d + raio_predador − raio_presa) / 2
```

Se as células se sobrepõem, cai no meio da sobreposição. Se ainda há uma
pequena folga (a margem de 15%), cai no meio da folga.

### 3.5 Círculo

- Vermelho, sem preenchimento, raio = raio da presa + 30%, com um mínimo de
  12 pixels na tela para ser visível com zoom afastado.
- Pulsa de leve para chamar atenção.
- Segue o tema da interface (modo noturno incluído).

### 3.6 Velocidade alta e modo headless

A 100.000× ou a 1.000.000×, o motor roda milhares de ticks entre dois quadros
da tela. Para não perder o momento exato, a checagem fica **dentro do laço do
motor**: no tick em que aparece o primeiro par, o motor para ali mesmo e
avisa a interface. Guardar os pares de cada tick para a lista ao vivo custa
praticamente nada, porque o motor já os calcula.

## 4. Linha da lista

```text
●  #4812 puxando #5530 · raio 1,42× · força 0,8 · há 37 ticks     [Ir até]
```

"Há 37 ticks" é há quanto tempo aquela puxada começou. Com a simulação
rodando, a lista se atualiza algumas vezes por segundo (não a cada tick, para
não pesar). Quando uma fagocitose termina, por captura ou porque a presa
escapou, a linha sai da lista. O rastreio mostra como terminou ("capturada" ou
"escapou") antes de soltar a câmera.

## 5. Perguntas

Todas respondidas em 22/11/2026 (seção 3).
