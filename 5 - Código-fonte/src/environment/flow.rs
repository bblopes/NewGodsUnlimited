use macroquad::prelude::*;

use crate::{
    config::{MAX_FLOW_SPEED, WORLD_HEIGHT, WORLD_WIDTH},
    environment::map::{MapDefinition, OASIS_CENTER, OASIS_RADII, ellipse_level, is_sunlit},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowPreset {
    Oasis,
    Classic,
    Left,
    /// Redemoinho (V5.1.2): espiral que abre do centro e vira anel na borda.
    Whirlpool,
    /// Circular (V5.1.2): anéis concêntricos girando em torno do centro.
    Circular,
}

impl FlowPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Oasis => "Oásis",
            Self::Classic => "Clássico",
            Self::Left => "Esquerda",
            Self::Whirlpool => "Redemoinho",
            Self::Circular => "Circular",
        }
    }
}

/// Campo de corrente selecionável; a seed apenas desloca os redemoinhos dos
/// modelos Oásis e Clássico.
pub struct FlowField {
    seed: u64,
    intensity: f32,
    preset: FlowPreset,
    /// Centro, raio e força dos dois redemoinhos do Clássico. Dependem só da
    /// seed; antes eram refeitos a cada amostra (milhares por tick).
    classic_eddies: [(Vec2, f32, f32); 2],
}

impl FlowField {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            intensity: 1.0,
            preset: FlowPreset::Classic,
            classic_eddies: std::array::from_fn(|index| {
                let index = index as u32;
                let seed_x = seeded_fraction(seed, index * 2);
                let seed_y = seeded_fraction(seed, index * 2 + 1);
                let center = vec2(
                    WORLD_WIDTH * (0.38 + seed_x * 0.36),
                    WORLD_HEIGHT * (0.22 + seed_y * 0.56),
                );
                let radius = 150.0 + seed_x * 90.0;
                let strength = if index == 0 { 1.65 } else { -1.35 };
                (center, radius, strength)
            }),
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
            FlowPreset::Whirlpool => whirlpool_flow(position),
            FlowPreset::Circular => circular_flow(position),
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
            || is_sunlit(position),
            |definition| definition.is_sunlit(position),
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
        for &(center, radius, strength) in &self.classic_eddies {
            flow += eddy(position, center, radius, strength);
        }
        flow
    }
}

/// Posição em coordenadas da elipse que cabe no mundo: (u, v) vão de −1 a 1
/// nas bordas do meio de cada lado; `r` é 1 na elipse inscrita.
fn ellipse_coordinates(position: Vec2) -> (f32, f32, f32) {
    let a = WORLD_WIDTH * 0.5;
    let b = WORLD_HEIGHT * 0.5;
    let u = (position.x - a) / a;
    let v = (position.y - b) / b;
    (u, v, (u * u + v * v).sqrt())
}

/// Direção tangente à elipse que passa pelo ponto, no sentido anti-horário da
/// tela (no alto vai para a esquerda, à esquerda desce). Vetor unitário.
fn ellipse_tangent(u: f32, v: f32) -> Vec2 {
    let a = WORLD_WIDTH * 0.5;
    let b = WORLD_HEIGHT * 0.5;
    vec2(a * v, -b * u).normalize_or_zero()
}

/// Circular: anéis concêntricos, todos no mesmo sentido. Parado no eixo,
/// acelera até ~30% do raio e mantém a velocidade até os cantos.
fn circular_flow(position: Vec2) -> Vec2 {
    let (u, v, r) = ellipse_coordinates(position);
    ellipse_tangent(u, v) * 4.0 * smoothstep(0.0, 0.3, r)
}

/// Redemoinho: gira no sentido horário da tela e empurra para fora perto do
/// centro, desenhando braços em espiral que se abrem. Perto de 85% do raio o
/// empurro troca de sinal, então a água que chega à borda fica presa num anel
/// circular em vez de se amontoar nas paredes.
fn whirlpool_flow(position: Vec2) -> Vec2 {
    let (u, v, r) = ellipse_coordinates(position);
    if r == 0.0 {
        return Vec2::ZERO;
    }
    const RIM: f32 = 0.85;
    let tangent = -ellipse_tangent(u, v) * 3.6 * smoothstep(0.0, 0.25, r);
    let outward = vec2(u / WORLD_WIDTH, v / WORLD_HEIGHT).normalize_or_zero();
    // Máximo em r = RIM/2; negativo além da borda, com teto no retorno.
    let push = (2.4 * r * (RIM - r) / (RIM * RIM * 0.25)).max(-1.6);
    tangent + outward * push
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
    fn whirlpool_opens_out_and_holds_a_rim() {
        let field = FlowField::new(1).with_preset(FlowPreset::Whirlpool);
        let center = vec2(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5);
        // Perto do centro: sai para fora.
        let near = center + vec2(WORLD_WIDTH * 0.15, 0.0);
        assert!(field.sample_with_map(near, None).x > 0.0);
        // Depois da borda: volta para dentro e gira.
        let out = center + vec2(WORLD_WIDTH * 0.48, 0.0);
        let flow = field.sample_with_map(out, None);
        assert!(flow.x < 0.0);
        assert!(flow.y.abs() > 0.5);
        assert_eq!(field.sample_with_map(center, None), Vec2::ZERO);
    }

    #[test]
    fn circular_is_tangent_everywhere() {
        let field = FlowField::new(1).with_preset(FlowPreset::Circular);
        let center = vec2(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5);
        for offset in [vec2(3_000.0, 0.0), vec2(0.0, 2_000.0), vec2(-4_000.0, 1_000.0)] {
            let flow = field.sample_with_map(center + offset, None);
            let normalized = vec2(offset.x / (WORLD_WIDTH * 0.5), offset.y / (WORLD_HEIGHT * 0.5));
            let radial = vec2(normalized.x / (WORLD_WIDTH * 0.5), normalized.y / (WORLD_HEIGHT * 0.5));
            assert!(flow.length() > 1.0);
            assert!(flow.normalize().dot(radial.normalize()).abs() < 1e-3);
        }
        // No alto do mapa a corrente vai para a esquerda.
        assert!(field.sample_with_map(center - vec2(0.0, 2_000.0), None).x < 0.0);
    }

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
