# Fisiologia neural e escala corporal

Esta versão torna a atividade da rede neural hereditária e faz o tamanho da
célula ter vantagens e limites explícitos. As regras abaixo são aplicadas no
tick de `World`, não são apenas indicadores visuais.

## Genoma e cérebro

O `Genome` agora tem 1.251 bits relevantes: 1.216 de registros de conexão,
cinco do alcance alimentar e 30 de atividade neural. Os 30 bits representam,
na ordem, 19 neurônios internos e 11 motores. Sensores externos continuam
fixos, pois são órgãos de entrada, não neurônios opcionais.

- bit ligado: o nó é avaliado e paga `0,0015` de manutenção por tick;
- bit desligado: o nó produz zero, não é calculado e suas conexões de entrada
  ou saída não são efetivas;
- cada conexão ainda efetiva cobra o custo mental configurável da sessão;
- como os bits estão no `Genome`, mutação, cor, similaridade e assinatura PG
  também mudam quando a arquitetura neural muda.

Runs V16 são migradas com os 30 nós ligados. Isso conserva a fiação já salva.
Na criação de fundadores, Val 1.0 reativa todos os motores e Val 2.0 reativa
os motores, o neurônio de porta e as conexões de navegação/doação garantidas.

## Tamanho corporal

Use `r = raio físico / CELL_RADIUS`. Para uma célula com 2R:

| Processo | Regra | Resultado em 2R |
| --- | --- | --- |
| Velocidade própria máxima | `8 / √r` | `5,66` unidades/tick |
| Custo basal e de motor | `r^0,485` | `1,40×` |
| Alcance alimentar efetivo | `alelo × √r` | `1,41×` |
| Custo do sensor alimentar | quadrado do alcance efetivo | `2×` |
| Emissão PG e química genérica | `r³` | `8×` |
| Custo químico | `r²` | `4×` |
| Tração de fagocitose | vantagem geométrica × `r` | `2×` antes da vantagem |
| Custo de fagocitose | `r^0,485` | `1,40×` |

A corrente ambiental não é reduzida pelo corpo: apenas o nado próprio recebe o
teto de velocidade. Assim, tamanho vira uma estratégia: corpo grande é mais
lento, mas melhor emissor, sensor e fagocitador por energia investida.
