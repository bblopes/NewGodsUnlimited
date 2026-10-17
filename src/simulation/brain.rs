use macroquad::prelude::*;

use crate::config::MAX_SWIM_SPEED;

/// 23 sensores + 19 internos + 11 motores. Os sensores químicos recebem
/// intensidade e os dois componentes do gradiente para PG, P1, P2 e P3.
pub const SLOT_COUNT: usize = 53;
pub const EXTERNAL_SLOT_COUNT: usize = 23;
pub const INTERNAL_START: usize = 23;
pub const INTERNAL_COUNT: usize = 19;
pub const TIMER_INTERNAL_START: usize = 16;
pub const MOTOR_START: usize = 42;
pub const MOTOR_SLOT_COUNT: usize = 11;
/// Só os 19 internos e os 11 motores são neurônios opcionais. Os 23 slots
/// externos são sensores físicos fixos e continuam disponíveis a toda célula.
pub const NEURAL_NODE_COUNT: usize = INTERNAL_COUNT + MOTOR_SLOT_COUNT;

/// Entradas dos slots 0–22 do cérebro.
#[derive(Clone, Copy)]
pub struct SensorInput {
    pub food_left: f32,
    pub food_right: f32,
    pub food_up: f32,
    pub food_down: f32,
    pub current_x: f32,
    pub current_y: f32,
    pub energy: f32,
    pub constant: f32,
    /// Há outra célula encostando no corpo físico desta.
    pub cell_contact: f32,
    /// Assinatura química genética: intensidade, direção X e direção Y.
    pub pg_intensity: f32,
    pub pg_gradient_x: f32,
    pub pg_gradient_y: f32,
    /// Os três canais genéricos selecionados geneticamente pela célula.
    pub p1_intensity: f32,
    pub p1_gradient_x: f32,
    pub p1_gradient_y: f32,
    pub p2_intensity: f32,
    pub p2_gradient_x: f32,
    pub p2_gradient_y: f32,
    pub p3_intensity: f32,
    pub p3_gradient_x: f32,
    pub p3_gradient_y: f32,
    /// Força de interação celular recebida no tick físico anterior,
    /// normalizada no intervalo [-1, 1]. Corrente ambiental tem sensores
    /// próprios e não entra aqui.
    pub force_x: f32,
    pub force_y: f32,
}

impl Default for SensorInput {
    fn default() -> Self {
        Self {
            food_left: 0.0,
            food_right: 0.0,
            food_up: 0.0,
            food_down: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            energy: 0.0,
            constant: 1.0,
            cell_contact: 0.0,
            pg_intensity: 0.0,
            pg_gradient_x: 0.0,
            pg_gradient_y: 0.0,
            p1_intensity: 0.0,
            p1_gradient_x: 0.0,
            p1_gradient_y: 0.0,
            p2_intensity: 0.0,
            p2_gradient_x: 0.0,
            p2_gradient_y: 0.0,
            p3_intensity: 0.0,
            p3_gradient_x: 0.0,
            p3_gradient_y: 0.0,
            force_x: 0.0,
            force_y: 0.0,
        }
    }
}

impl SensorInput {
    fn slot_values(self) -> [f32; EXTERNAL_SLOT_COUNT] {
        [
            self.food_left,
            self.food_right,
            self.food_up,
            self.food_down,
            self.current_x,
            self.current_y,
            self.energy,
            self.constant,
            self.cell_contact,
            self.pg_intensity,
            self.pg_gradient_x,
            self.pg_gradient_y,
            self.p1_intensity,
            self.p1_gradient_x,
            self.p1_gradient_y,
            self.p2_intensity,
            self.p2_gradient_x,
            self.p2_gradient_y,
            self.p3_intensity,
            self.p3_gradient_x,
            self.p3_gradient_y,
            self.force_x,
            self.force_y,
        ]
    }
}

/// Saídas dos slots 42–52. Cada uma é limitada ao intervalo [0, 1].
#[derive(Clone, Copy, Default)]
pub struct MotorOutput {
    pub left: f32,
    pub right: f32,
    pub up: f32,
    pub down: f32,
    /// Pedido comportamental de divisão. O corpo ainda valida as faixas de
    /// energia; ele não pode dividir abaixo do mínimo configurado.
    pub reproduce: f32,
    /// Quando há uma célula próxima menor, pede força de fagocitose.
    pub phagocytose: f32,
    /// Reforça a assinatura genética basal, que já é emitida continuamente.
    pub emit_pg: f32,
    /// Emissões químicas genéricas P1, P2 e P3.
    pub emit_p1: f32,
    pub emit_p2: f32,
    pub emit_p3: f32,
    /// Transfere energia para a célula fisicamente encostada mais próxima.
    pub donate: f32,
}

impl MotorOutput {
    pub fn swim_velocity(self, maximum_speed: f32) -> Vec2 {
        let raw_direction = vec2(self.right - self.left, self.down - self.up);
        raw_direction.normalize_or_zero()
            * raw_direction.length().min(1.0)
            * maximum_speed.clamp(0.0, MAX_SWIM_SPEED)
    }

    pub fn energy_cost(self, max_cost: f32) -> f32 {
        max_cost
            * (self.left * self.left
                + self.right * self.right
                + self.up * self.up
                + self.down * self.down)
    }

    pub fn pheromone_activation(self) -> [f32; 4] {
        [self.emit_pg, self.emit_p1, self.emit_p2, self.emit_p3]
    }
}

/// Uma conexão já interpretada. O genoma binário produzirá esses registros
/// na etapa seguinte; o cérebro não precisará reler bits em cada tick.
#[derive(Clone, Copy)]
pub struct DecodedConnection {
    pub source: usize,
    pub destination: usize,
    pub weight: f32,
}

/// Estado dinâmico de uma rede. Os valores de conexão serão herdados pelo
/// genoma, não escritos como comportamento manual.
#[derive(Clone)]
pub struct Brain {
    internal_state: [f32; INTERNAL_COUNT],
    /// Bits hereditários, já decodificados uma vez no nascimento/carregamento.
    /// Nó desligado não é avaliado e não deixa conexões chegarem ao cérebro.
    active_nodes: [bool; NEURAL_NODE_COUNT],
    connections: Vec<DecodedConnection>,
    last_slots: [f32; SLOT_COUNT],
    connection_signals: Vec<f32>,
}

impl Default for Brain {
    fn default() -> Self {
        Self {
            internal_state: [0.0; INTERNAL_COUNT],
            active_nodes: [false; NEURAL_NODE_COUNT],
            connections: Vec::new(),
            last_slots: [0.0; SLOT_COUNT],
            connection_signals: Vec::new(),
        }
    }
}

impl Brain {
    pub fn tick(&mut self, sensors: SensorInput) -> MotorOutput {
        let external = sensors.slot_values();
        let mut internal_sums = [0.0; INTERNAL_COUNT];
        let mut motor_sums = [0.0; MOTOR_SLOT_COUNT];

        // Conexões internas leem o estado do tick anterior. Isso evita ordem
        // arbitrária de avaliação e dá significado aos neurônios temporais.
        if self.connection_signals.len() != self.connections.len() {
            self.connection_signals.resize(self.connections.len(), 0.0);
        }
        for (index, connection) in self.connections.iter().enumerate() {
            let signal = source_signal(connection.source, &external, &self.internal_state);
            let transmitted_signal = signal * connection.weight;
            self.connection_signals[index] = transmitted_signal;
            if (INTERNAL_START..MOTOR_START).contains(&connection.destination) {
                internal_sums[connection.destination - INTERNAL_START] += transmitted_signal;
            } else if (MOTOR_START..SLOT_COUNT).contains(&connection.destination) {
                motor_sums[connection.destination - MOTOR_START] += transmitted_signal;
            }
        }

        let mut next_internal_state = [0.0; INTERNAL_COUNT];
        for index in 0..INTERNAL_COUNT {
            if !self.active_nodes[index] {
                continue;
            }
            let activated = centered_sigmoid(internal_sums[index]);
            next_internal_state[index] = if index < TIMER_INTERNAL_START {
                activated
            } else {
                let rate = [0.25, 0.08, 0.02][index - TIMER_INTERNAL_START];
                (1.0 - rate) * self.internal_state[index] + rate * activated
            };
        }
        self.internal_state = next_internal_state;

        let motor = |index: usize| {
            self.active_nodes[INTERNAL_COUNT + index]
                .then(|| centered_sigmoid(motor_sums[index]).max(0.0))
                .unwrap_or(0.0)
        };
        let motors = MotorOutput {
            left: motor(0),
            right: motor(1),
            up: motor(2),
            down: motor(3),
            reproduce: motor(4),
            phagocytose: motor(5),
            emit_pg: motor(6),
            emit_p1: motor(7),
            emit_p2: motor(8),
            emit_p3: motor(9),
            donate: motor(10),
        };
        self.last_slots[..EXTERNAL_SLOT_COUNT].copy_from_slice(&external);
        self.last_slots[INTERNAL_START..MOTOR_START].copy_from_slice(&self.internal_state);
        self.last_slots[MOTOR_START..SLOT_COUNT].copy_from_slice(&[
            motors.left,
            motors.right,
            motors.up,
            motors.down,
            motors.reproduce,
            motors.phagocytose,
            motors.emit_pg,
            motors.emit_p1,
            motors.emit_p2,
            motors.emit_p3,
            motors.donate,
        ]);
        motors
    }

    pub fn configure_network(
        &mut self,
        connections: Vec<DecodedConnection>,
        active_nodes: [bool; NEURAL_NODE_COUNT],
    ) {
        self.connections = connections;
        self.active_nodes = active_nodes;
        self.connection_signals = vec![0.0; self.connections.len()];
    }

    pub fn internal_state(&self) -> &[f32; INTERNAL_COUNT] {
        &self.internal_state
    }

    pub fn restore_internal_state(&mut self, state: [f32; INTERNAL_COUNT]) {
        self.internal_state = state;
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// O custo neural tem duas partes: manter nós habilitados e avaliar as
    /// conexões que ainda existem entre eles. Nós desligados e fios ligados a
    /// eles não chegam aqui, portanto não custam nem são calculados.
    pub fn mental_energy_cost(&self, cost_per_connection: f32, cost_per_active_node: f32) -> f32 {
        self.connections.len() as f32 * cost_per_connection.max(0.0)
            + self.active_node_count() as f32 * cost_per_active_node.max(0.0)
    }

    pub fn active_node_count(&self) -> usize {
        self.active_nodes.iter().filter(|active| **active).count()
    }

    pub fn slot_value(&self, slot: usize) -> f32 {
        self.last_slots[slot]
    }

    pub fn visual_connections(&self) -> impl Iterator<Item = (DecodedConnection, f32)> + '_ {
        self.connections
            .iter()
            .copied()
            .zip(self.connection_signals.iter().copied())
    }
}

fn source_signal(
    source: usize,
    external: &[f32; EXTERNAL_SLOT_COUNT],
    internal_state: &[f32; INTERNAL_COUNT],
) -> f32 {
    if source < EXTERNAL_SLOT_COUNT {
        external[source]
    } else if (INTERNAL_START..MOTOR_START).contains(&source) {
        internal_state[source - INTERNAL_START]
    } else {
        0.0
    }
}

fn centered_sigmoid(value: f32) -> f32 {
    2.0 / (1.0 + (-value).exp()) - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_motor_is_not_evaluated_and_active_nodes_have_upkeep() {
        let mut brain = Brain::default();
        let mut nodes = [false; NEURAL_NODE_COUNT];
        nodes[INTERNAL_COUNT] = true;
        brain.configure_network(
            vec![DecodedConnection {
                source: 7,
                destination: MOTOR_START,
                weight: 1.0,
            }],
            nodes,
        );

        let motors = brain.tick(SensorInput::default());
        assert!(motors.left > 0.0);
        assert_eq!(brain.active_node_count(), 1);
        assert!((brain.mental_energy_cost(0.0025, 0.0015) - 0.004).abs() < 0.000_001);

        brain.configure_network(Vec::new(), [false; NEURAL_NODE_COUNT]);
        assert_eq!(brain.tick(SensorInput::default()).left, 0.0);
        assert_eq!(brain.mental_energy_cost(0.0025, 0.0015), 0.0);
    }
}
