# Proposta: doação de energia e população Val 2.0

> Estado: **Implementado e publicado** no executável principal.

## Objetivo

Criar a possibilidade física de uma célula transferir parte da própria energia
para outra célula encostada. Não haverá regra de “aliado” ou parentesco no
motor: a célula só percebe contato (`Ct`) e o campo químico `PG`, que traz uma
concentração mais alta quando existem assinaturas geneticamente semelhantes na
vizinhança.

O possível benefício coletivo é emergente: uma célula grande pode acumular
energia e mantê-la viva; outra pode aprender a entregar parte dessa reserva
quando uma estratégia de parentes próximos sobreviver melhor do que guardar
tudo para si.

## Motor e regra física propostos

Adicionar o motor neural `Doar` depois de `EP3`, levando a rede de 52 para 53
slots. Em cada tick, somente o alvo em contato mais próximo é elegível.

```text
energia_transferida = ativação(Doar) × 4,0 energia/tick
energia_transferida × 1,02 ≤ energia_atual - reserva_do_doador
energia_recebida    = min(energia_transferida, capacidade_do_receptor - energia_receptor)
```

- A transferência é conservativa: o que o receptor não pode guardar não é
  criado nem perdido pela regra; simplesmente não é transferido. O custo
  operacional é a única parcela deliberadamente dissipada.
- Sem contato físico, `Doar` não produz efeito nem custo de transferência.
- O doador não pode cair abaixo da reserva inicial de 60 unidades de energia.
- O mecanismo usa **energia**, não cria uma nova partícula de fitoplâncton.
- A transferência cobra 2% operacional sobre a energia entregue; esse custo é
  pago pelo doador e não chega ao receptor.

## Condição que Val 2.0 receberá no nascimento

`Val 2.0` será uma nova origem de população, separada de Clássica, Motor
Mínima e Val 1.0. Ela parte de uma amostra salva de Val 1.0, mas acrescenta uma
amarração inicial de navegação e cooperação. Não muda a física: apenas garante
alguns registros ativos nos fundadores.

Antes do circuito cooperativo, os quatro caminhos abaixo são fixados com peso
positivo: alimento esquerdo→nadar esquerdo, direito→direito, acima→acima e
abaixo→abaixo. Portanto, Val 2.0 não começa sem saber se mover até alimento.

O caminho proposto é um pequeno portão interno:

```text
Ct  ──(+1.0)──┐
PG  ──(+1.0)──┼──> I0 ──(+3 conexões +1.0)──> Doar
+1  ──(-1.0)──┘
```

`Ct` e `PG` altos juntos superam o viés negativo e ativam `I0`; um deles
sozinho tende a não bastar. As três conexões de saída tornam a ativação de
`Doar` visível nos fundadores. Assim, Val 2.0 **tenta doar apenas
quando tocam alguém e estão em uma região com alta presença química de
assinaturas próximas**. A regra não identifica diretamente o receptor como
parente; ela usa o ambiente químico local, como solicitado.

Os pesos e os registros continuam genéticos: filhos sofrem mutação normal e
podem conservar, enfraquecer, inverter ou perder essa estratégia. Val 2.0 é
uma condição inicial experimental, não uma obrigação permanente da linhagem.

## Medidas para validar depois

- energia transferida por tick e cumulativa;
- quantos pares receberam transferência;
- fração de transferências em contato com `PG` alto;
- sobrevivência e reprodução dos doadores versus não doadores;
- diversidade genética antes e depois da estratégia se espalhar.

## Fora desta proposta

- bônus automático por parentesco;
- saldo coletivo, colônia ou inventário compartilhado;
- interpretação semântica dos feromônios;
- Val 2.0 no laboratório antes de validar a transferência visualmente no
  executável principal.
