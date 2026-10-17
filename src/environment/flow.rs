use macroquad::prelude::*;

use crate::{
    config::{MAX_FLOW_SPEED, WORLD_HEIGHT, WORLD_WIDTH},
    environment::map::{MapDefinition, OASIS_CENTER, OASIS_RADII, ellipse_level, is_fertile},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowPreset {
    Oasis,
    Classic,
    Left,
}

impl FlowPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Oasis => "Oásis",
            Self::Classic => "Clássico",
            Self::Left => "Esquerda",
        }
    }
}

/// Campo de corrente selecionável; a seed apenas desloca os redemoinhos dos
/// modelos Oásis e Clássico.
pub struct FlowField {
    seed: u64,
    intensity: f32,
    preset: FlowPreset,
}

impl FlowField {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            intensity: 1.0,
            preset: FlowPreset::Classic,
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    pub fn with_preset(mut self, preset: FlowPreset) -> Self {
        self.preset = preset;
        self
    }

    pub fn sample_with_map(&self, position: Vec2, map: Option<&MapDefinition>) -> Vec2 {
        let mut flow = match self.preset {
            FlowPreset::Oasis => self.oasis_flow(position, map),
            FlowPreset::Classic => self.classic_flow(position),
            FlowPreset::Left => vec2(-4.5, 0.0),
        };

        flow *= self.intensity;
        if flow.length() > MAX_FLOW_SPEED {
            flow.normalize() * MAX_FLOW_SPEED
        } else {
            flow
        }
    }

    fn oasis_flow(&self, position: Vec2, map: Option<&MapDefinition>) -> Vec2 {
        let fertile = map.map_or_else(
            || is_fertile(position),
            |definition| definition.is_fertile(position),
        );
        let mut flow = if fertile {
            let normalized = vec2(
                (position.x - OASIS_CENTER.x) / OASIS_RADII.x,
                (position.y - OASIS_CENTER.y) / OASIS_RADII.y,
            );
            // Deriva diagonal reproduz o trajeto de entrada → saída; a parte
            // tangencial cria as curvas suaves indicadas no desenho vermelho.
            let circulation = vec2(-normalized.y * 1.8, normalized.x * 1.2);
            let wall_slowdown = (1.18 - ellipse_level(position)).clamp(0.35, 1.0);
            (vec2(4.4, 1.55) + circulation) * wall_slowdown
        } else {
            // Fora do oásis há apenas uma deriva baixa: alimento do deserto
            // continua vivo, mas o sistema hidráulico dominante é o central.
            vec2(0.25, 0.08)
        };

        let phase = seeded_fraction(self.seed, 0) - 0.5;
        flow += eddy(position, OASIS_CENTER, OASIS_RADII.y * 0.42, phase * 0.45);
        flow
    }

    fn classic_flow(&self, position: Vec2) -> Vec2 {
        let nx = (position.x / WORLD_WIDTH).clamp(0.0, 1.0);
        let ny = ((position.y / WORLD_HEIGHT) * 2.0 - 1.0).clamp(-1.0, 1.0);
        let center_stream = (-(ny / 0.30).powi(2)).exp();
        let mut flow = vec2(3.4 * center_stream - 1.25 * (1.0 - center_stream), 0.0);
        let turns_right = smoothstep(0.70, 0.94, nx);
        let turns_left = 1.0 - smoothstep(0.06, 0.28, nx);
        flow.y += ny.signum() * (2.8 * turns_right - 1.4 * turns_left);
        for index in 0..2 {
            let seed_x = seeded_fraction(self.seed, index * 2);
            let seed_y = seeded_fraction(self.seed, index * 2 + 1);
            let center = vec2(
                WORLD_WIDTH * (0.38 + seed_x * 0.36),
                WORLD_HEIGHT * (0.22 + seed_y * 0.56),
            );
            let radius = 150.0 + seed_x * 90.0;
            let strength = if index == 0 { 1.65 } else { -1.35 };
            flow += eddy(position, center, radius, strength);
        }
        flow
    }
}

fn smoothstep(edge_start: f32, edge_end: f32, value: f32) -> f32 {
    let t = ((value - edge_start) / (edge_end - edge_start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn seeded_fraction(seed: u64, rotation: u32) -> f32 {
    let mixed = seed.rotate_left(rotation * 17) ^ (0x9E37_79B9_7F4A_7C15u64 >> rotation);
    (mixed & 0xFFFF) as f32 / 65_535.0
}

fn eddy(position: Vec2, center: Vec2, radius: f32, strength: f32) -> Vec2 {
    let offset = position - center;
    let distance = offset.length();
    if distance >= radius || distance == 0.0 {
        return Vec2::ZERO;
    }

    let tangent = vec2(-offset.y, offset.x) / distance;
    let influence = (1.0 - distance / radius).powi(2);
    tangent * strength * influence
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{WORLD_HEIGHT, WORLD_WIDTH};

    #[test]
    fn zero_intensity_returns_zero_vector_everywhere() {
        let field = FlowField::new(84).with_intensity(0.0);
        for position in [
            vec2(0.0, 0.0),
            vec2(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5),
            vec2(WORLD_WIDTH, WORLD_HEIGHT),
            vec2(WORLD_WIDTH * 0.72, WORLD_HEIGHT * 0.18),
        ] {
            assert_eq!(field.sample_with_map(position, None), Vec2::ZERO);
        }
    }
}
