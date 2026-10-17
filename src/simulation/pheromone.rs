//! Campo químico instantâneo. Ele é reconstruído a cada tick: não há
//! persistência, difusão acumulada nem memória fora das emissões em curso.

use macroquad::prelude::*;

use crate::{
    config::{WORLD_HEIGHT, WORLD_WIDTH},
    environment::map::MapDefinition,
    simulation::world::{CELL_RADIUS, Organism},
};

/// Preserva células químicas de 100×100 unidades após o mundo dobrar nos
/// dois eixos. A malha só é renderizada quando o filtro químico está ligado.
pub const FIELD_WIDTH: usize = 128;
pub const FIELD_HEIGHT: usize = 72;
pub const GENERIC_CHANNEL_COUNT: usize = 8;
pub const PHEROMONE_RANGE: f32 = 500.0;
pub const BASAL_PG_PPM: f32 = 1.0;
pub const GENERIC_PPM: f32 = 0.85;
pub const BASAL_PG_COST: f32 = 0.0004;
pub const GENERIC_PPM_COST: f32 = 0.0010;
const SENSOR_PPM_SCALE: f32 = 1.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PheromoneDisplay {
    Pg,
    Generic(u8),
    All,
}

impl PheromoneDisplay {
    pub fn label(self) -> String {
        match self {
            Self::Pg => "PG · assinatura genética".to_owned(),
            Self::Generic(channel) => format!("C{channel} · químico genérico"),
            Self::All => "Todos os feromônios".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct ChemicalSensors {
    pub intensity: f32,
    pub gradient_x: f32,
    /// Sinal positivo significa concentração maior acima, no mundo visual.
    pub gradient_y: f32,
}

#[derive(Clone)]
pub struct PheromoneField {
    pg: Vec<f32>,
    pg_signature_x: Vec<f32>,
    pg_signature_y: Vec<f32>,
    generic: Vec<Vec<f32>>,
}

impl Default for PheromoneField {
    fn default() -> Self {
        let cells = FIELD_WIDTH * FIELD_HEIGHT;
        Self {
            pg: vec![0.0; cells],
            pg_signature_x: vec![0.0; cells],
            pg_signature_y: vec![0.0; cells],
            generic: (0..GENERIC_CHANNEL_COUNT)
                .map(|_| vec![0.0; cells])
                .collect(),
        }
    }
}

impl PheromoneField {
    /// Reconstrói a fotografia química do tick com as decisões motoras do
    /// tick anterior. O cérebro lê esta fotografia e escolhe a próxima.
    pub fn rebuild(&mut self, organisms: &[Organism], map: &MapDefinition) {
        self.pg.fill(0.0);
        self.pg_signature_x.fill(0.0);
        self.pg_signature_y.fill(0.0);
        for field in &mut self.generic {
            field.fill(0.0);
        }

        for organism in organisms {
            let area_scale = (organism.physical_radius() / CELL_RADIUS).powi(2);
            let signature = organism.genome.pheromone_signature();
            let pg_amount = BASAL_PG_PPM * (1.0 + organism.motors.emit_pg) * area_scale;
            self.deposit_pg(organism.position, pg_amount, signature, map);

            let channels = organism.genome.pheromone_channels();
            for (index, activation) in organism.motors.pheromone_activation()[1..]
                .iter()
                .enumerate()
            {
                if *activation > 0.01 {
                    self.deposit_generic(
                        organism.position,
                        channels[index] as usize,
                        GENERIC_PPM * *activation * area_scale,
                        map,
                    );
                }
            }
        }
    }

    pub fn sensors_for(&self, organism: &Organism) -> [ChemicalSensors; 4] {
        let area_scale = (organism.physical_radius() / CELL_RADIUS).powi(2);
        let own_pg = BASAL_PG_PPM * (1.0 + organism.motors.emit_pg) * area_scale;
        let index = self.nearest_index(organism.position);
        let total_pg = (self.pg[index] - own_pg).max(0.0);
        let signature = organism.genome.pheromone_signature();
        let moment = vec2(self.pg_signature_x[index], self.pg_signature_y[index]);
        let affinity = if moment.length_squared() > f32::EPSILON {
            (signature.dot(moment.normalize()) + 1.0) * 0.5
        } else {
            0.0
        };
        let mut result = [ChemicalSensors::default(); 4];
        result[0] = self.sensors_at(&self.pg, organism.position, total_pg * affinity);
        for (slot, channel) in organism.genome.pheromone_channels().into_iter().enumerate() {
            result[slot + 1] =
                self.sensors_at(&self.generic[channel as usize], organism.position, -1.0);
        }
        result
    }

    pub fn display_value(&self, display: PheromoneDisplay, x: usize, y: usize) -> f32 {
        let index = self.index(x.min(FIELD_WIDTH - 1), y.min(FIELD_HEIGHT - 1));
        match display {
            PheromoneDisplay::Pg => self.pg[index],
            PheromoneDisplay::Generic(channel) => {
                self.generic[channel as usize % GENERIC_CHANNEL_COUNT][index]
            }
            PheromoneDisplay::All => {
                self.pg[index] + self.generic.iter().map(|field| field[index]).sum::<f32>()
            }
        }
    }

    pub fn display_max(&self, display: PheromoneDisplay) -> f32 {
        (0..FIELD_HEIGHT)
            .flat_map(|y| (0..FIELD_WIDTH).map(move |x| self.display_value(display, x, y)))
            .fold(0.0, f32::max)
    }

    fn deposit_pg(&mut self, source: Vec2, amount: f32, signature: Vec2, map: &MapDefinition) {
        for (index, weight) in Self::deposition_samples(source, map) {
            let contribution = amount * weight;
            self.pg[index] += contribution;
            self.pg_signature_x[index] += contribution * signature.x;
            self.pg_signature_y[index] += contribution * signature.y;
        }
    }

    fn deposit_generic(&mut self, source: Vec2, channel: usize, amount: f32, map: &MapDefinition) {
        let field = &mut self.generic[channel % GENERIC_CHANNEL_COUNT];
        for (index, weight) in Self::deposition_samples(source, map) {
            field[index] += amount * weight;
        }
    }

    fn deposition_samples(source: Vec2, map: &MapDefinition) -> Vec<(usize, f32)> {
        let mut samples = Vec::with_capacity(121);
        let step = cell_size();
        let min_x = ((source.x - PHEROMONE_RANGE) / step.x).floor().max(0.0) as usize;
        let max_x = ((source.x + PHEROMONE_RANGE) / step.x)
            .ceil()
            .min(FIELD_WIDTH as f32 - 1.0) as usize;
        let min_y = ((source.y - PHEROMONE_RANGE) / step.y).floor().max(0.0) as usize;
        let max_y = ((source.y + PHEROMONE_RANGE) / step.y)
            .ceil()
            .min(FIELD_HEIGHT as f32 - 1.0) as usize;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let position = cell_center(x, y);
                let distance_squared = source.distance_squared(position);
                if distance_squared > PHEROMONE_RANGE * PHEROMONE_RANGE
                    || !map.has_clear_line(source, position)
                {
                    continue;
                }
                let weight = (-distance_squared / (2.0 * (PHEROMONE_RANGE * 0.38).powi(2))).exp();
                samples.push((y * FIELD_WIDTH + x, weight));
            }
        }
        samples
    }

    fn sensors_at(&self, field: &[f32], position: Vec2, override_center: f32) -> ChemicalSensors {
        let (x, y) = self.cell(position);
        let value = if override_center >= 0.0 {
            override_center
        } else {
            field[self.index(x, y)]
        };
        let left = field[self.index(x.saturating_sub(1), y)];
        let right = field[self.index((x + 1).min(FIELD_WIDTH - 1), y)];
        let below = field[self.index(x, (y + 1).min(FIELD_HEIGHT - 1))];
        let above = field[self.index(x, y.saturating_sub(1))];
        ChemicalSensors {
            intensity: (value / SENSOR_PPM_SCALE).clamp(0.0, 1.0),
            gradient_x: ((right - left) / SENSOR_PPM_SCALE).tanh(),
            gradient_y: ((above - below) / SENSOR_PPM_SCALE).tanh(),
        }
    }

    fn cell(&self, position: Vec2) -> (usize, usize) {
        (
            (position.x / WORLD_WIDTH * FIELD_WIDTH as f32)
                .floor()
                .clamp(0.0, FIELD_WIDTH as f32 - 1.0) as usize,
            (position.y / WORLD_HEIGHT * FIELD_HEIGHT as f32)
                .floor()
                .clamp(0.0, FIELD_HEIGHT as f32 - 1.0) as usize,
        )
    }

    fn nearest_index(&self, position: Vec2) -> usize {
        let (x, y) = self.cell(position);
        self.index(x, y)
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * FIELD_WIDTH + x
    }
}

fn cell_size() -> Vec2 {
    vec2(
        WORLD_WIDTH / FIELD_WIDTH as f32,
        WORLD_HEIGHT / FIELD_HEIGHT as f32,
    )
}
fn cell_center(x: usize, y: usize) -> Vec2 {
    vec2(
        (x as f32 + 0.5) * cell_size().x,
        (y as f32 + 0.5) * cell_size().y,
    )
}

pub fn pheromone_energy_cost(organism: &Organism) -> f32 {
    let area_scale = (organism.physical_radius() / CELL_RADIUS).powi(2);
    let motors = organism.motors.pheromone_activation();
    (BASAL_PG_COST * (1.0 + motors[0]) + GENERIC_PPM_COST * (motors[1] + motors[2] + motors[3]))
        * area_scale
}
