# Guia de leitura do tick do mundo

Este guia acompanha a função `World::tick()` em
`src/simulation/world.rs`. Leia-o com o arquivo aberto: os comentários
`Etapa 1` a `Etapa 13` foram inseridos diretamente no código para que a
documentação e a implementação tenham a mesma ordem.

## Quem chama quem

```text
main.rs
  → app::run()                         interface e desenho
      → simulation_worker()            controla a execução em segundo plano
          → World::tick()              coordena um instante biológico
              → brain.rs               decide motores a partir dos sensores
              → flow.rs / map.rs       corrente, terreno e paredes
              → pheromone.rs           química e custo de emissão
              → genome.rs              mutação na criação de descendentes
```

`World` guarda o estado do ecossistema. Ele não se atualiza sozinho:
`simulation_worker()` em `app.rs` chama `World::tick()` repetidas vezes. A
interface recebe apenas cópias do mundo para desenhá-las.

## Como ler a assinatura da função

```rust
pub fn tick(&mut self, flow: &FlowField, ..., reproduction_limit: f32)
```

- `&mut self`: o tick modifica o próprio mundo: organismos, alimento,
  feromônios, contadores e energia.
- `flow`: corrente ambiental já configurada pelo aplicativo.
- parâmetros de emissão, energia, custo e reprodução: regras escolhidas para
  a sessão atual, vindas de `EngineSettings` em `app.rs`.

## As treze etapas

| Etapa no código | Funções importantes | Resultado |
|---|---|---|
| 1. Tempo | `self.tick += 1` | Um novo instante lógico começa; limites de reprodução são normalizados. |
| 2. Emissão | `spawn_food_anywhere` | Um único crédito de emissão gera fitoplâncton uniformemente no mapa. |
| 3. Química | `PheromoneField::rebuild` | Campo químico é reconstruído com as emissões decididas no tick anterior. |
| 4. Sensores | `update_sensors` | Cada organismo recebe alimento direcional, corrente, energia, contato, química e força externa. |
| 5. Decisão | `Brain::tick` | Sensores e estado interno anterior viram motores de nado, reprodução, fagocitose, emissão e doação. |
| 6. Alimento | `FlowField::sample_with_map`, `reproduce_sunlit_phytoplankton` | Corrente desloca o fito; sombra o envelhece e luz permite duplicação local. |
| 7. Células | `motors.swim_velocity` | Corrente mais nado deslocam organismos; o mapa bloqueia paredes. |
| 8. Interações | `apply_phagocytosis`, `apply_anti_cramming`, `apply_donations` | Células podem puxar/consumir presas, separar-se e transferir energia. |
| 9. Custos | `mental_energy_cost`, `pheromone_energy_cost` | A célula paga metabolismo, movimento, cérebro e química. |
| 10. Mortes | `organisms.retain` | Saem células sem energia ou além de sua idade máxima. |
| 11. Alimentação | `closest_organism_containing` | Cada alimento escolhe uma única célula vencedora, que ganha energia. |
| 12. Limpeza | filtro sobre `self.food` | Alimento comido ou velho é removido. |
| 13. Reprodução | `reproduce_ready_organisms` → `Genome::mutated_from` | Pai e filho dividem energia; o filho recebe genoma e longevidade mutados. |

## Duas funções com nomes parecidos

```text
set_anti_cramming(config)    guarda a configuração vinda da interface
apply_anti_cramming(...)     executa a separação física no tick
```

`set_anti_cramming()` aparece depois de `tick()` no arquivo porque é uma
função de configuração, não uma etapa do ciclo de vida. A execução real está
na etapa 8.

## Detalhe que merece atenção

O código cobra custos e remove mortes nas etapas 9 e 10 **antes** de conceder
energia de alimento na etapa 11. Ao estudar ou mudar a economia do mundo,
essa ordem é decisiva: uma célula sem energia não consegue ser salva por uma
partícula que também alcançaria naquele mesmo tick.
