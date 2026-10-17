//! Geometria estática da câmara: máscara de luz solar, paredes e comportas.

use std::io::{self, Read, Write};

use macroquad::prelude::*;

use crate::config::{WORLD_HEIGHT, WORLD_WIDTH};

pub const OASIS_CENTER: Vec2 = Vec2::new(WORLD_WIDTH * 0.53, WORLD_HEIGHT * 0.50);
pub const OASIS_RADII: Vec2 = Vec2::new(WORLD_WIDTH * 0.39, WORLD_HEIGHT * 0.255);
pub const WALL_HALF_THICKNESS: f32 = 18.0;
/// Mantemos pixels lógicos de 40×40 unidades mesmo após ampliar o mundo.
/// Isso dobra também a resolução útil do editor de paredes.
pub const MAP_GRID_WIDTH: usize = 320;
pub const MAP_GRID_HEIGHT: usize = 180;
const LEGACY_MAP_GRID_WIDTH: usize = 160;
const LEGACY_MAP_GRID_HEIGHT: usize = 90;

/// Mapa raster propositalmente grosseiro: cada pixel lógico ocupa 40×40
/// unidades do mundo. Isso mantém desenho e colisão baratos mesmo com muitas
/// células.
#[derive(Clone)]
pub struct MapDefinition {
    walls: Vec<u8>,
    sunlight: Vec<u8>,
}

impl Default for MapDefinition {
    fn default() -> Self {
        let mut map = Self::empty();
        for y in 0..MAP_GRID_HEIGHT {
            for x in 0..MAP_GRID_WIDTH {
                let position = map.cell_center(x, y);
                let index = map.index(x, y);
                if ellipse_level(position) <= 1.0 {
                    map.sunlight[index] = 1;
                }
                let wall_band = (ellipse_level(position) - 1.0).abs()
                    <= WALL_HALF_THICKNESS * 1.7 / OASIS_RADII.y;
                if wall_band && is_solid_wall_at_angle(ellipse_angle(position)) {
                    map.walls[index] = 1;
                }
            }
        }
        map
    }
}

impl MapDefinition {
    pub fn empty() -> Self {
        Self {
            walls: vec![0; MAP_GRID_WIDTH * MAP_GRID_HEIGHT],
            sunlight: vec![0; MAP_GRID_WIDTH * MAP_GRID_HEIGHT],
        }
    }

    pub fn is_sunlit(&self, position: Vec2) -> bool {
        self.cell_at(position)
            .is_some_and(|(x, y)| self.sunlight[self.index(x, y)] != 0)
    }

    /// Camada preparada para valores contínuos futuros. Nesta primeira fase,
    /// cada pixel é 0 (sombra) ou 1 (sol pleno).
    pub fn sunlight_intensity(&self, position: Vec2) -> f32 {
        self.is_sunlit(position) as u8 as f32
    }

    pub fn is_wall(&self, position: Vec2) -> bool {
        self.cell_at(position)
            .is_some_and(|(x, y)| self.walls[self.index(x, y)] != 0)
    }

    /// Consulta barata de linha de visão. A amostragem percorre metade de uma
    /// célula lógica por vez, portanto um muro pintado não deixa passar um
    /// sensor de alimento pelo vão entre dois pixels da grade.
    pub fn has_clear_line(&self, from: Vec2, to: Vec2) -> bool {
        let distance = from.distance(to);
        let step = self.cell_size().x.min(self.cell_size().y) * 0.5;
        let samples = (distance / step).ceil().max(1.0) as usize;
        let delta = to - from;
        for sample_index in 1..=samples {
            // Equivale a `from.lerp(to, fraction)`, mas evita criar Vec2,
            // Option e chamar `is_wall` para cada amostra. Esta rotina é
            // quente tanto para sensores de alimento quanto para a malha
            // química de feromônios.
            let fraction = sample_index as f32 / samples as f32;
            let sample_x = from.x + delta.x * fraction;
            let sample_y = from.y + delta.y * fraction;
            if !(0.0..WORLD_WIDTH).contains(&sample_x) || !(0.0..WORLD_HEIGHT).contains(&sample_y) {
                continue;
            }
            let x = (sample_x / WORLD_WIDTH * MAP_GRID_WIDTH as f32) as usize;
            let y = (sample_y / WORLD_HEIGHT * MAP_GRID_HEIGHT as f32) as usize;
            if self.walls[y * MAP_GRID_WIDTH + x] != 0 {
                return false;
            }
        }
        true
    }

    /// Indica se algum pixel de parede pode tocar o disco informado. É uma
    /// consulta conservadora: `true` pode incluir uma parede que a linha não
    /// alcançará, mas `false` garante que nenhuma linha inteiramente dentro
    /// desse disco pode ser bloqueada.
    pub fn has_wall_within_distance(&self, position: Vec2, distance: f32) -> bool {
        let cell_size = self.cell_size();
        let min_x = ((position.x - distance) / cell_size.x).floor().max(0.0) as usize;
        let max_x = ((position.x + distance) / cell_size.x)
            .ceil()
            .min(MAP_GRID_WIDTH as f32 - 1.0) as usize;
        let min_y = ((position.y - distance) / cell_size.y).floor().max(0.0) as usize;
        let max_y = ((position.y + distance) / cell_size.y)
            .ceil()
            .min(MAP_GRID_HEIGHT as f32 - 1.0) as usize;
        let distance_squared = distance * distance;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if self.walls[self.index(x, y)] == 0 {
                    continue;
                }
                let left = x as f32 * cell_size.x;
                let right = left + cell_size.x;
                let top = y as f32 * cell_size.y;
                let bottom = top + cell_size.y;
                let nearest_x = position.x.clamp(left, right);
                let nearest_y = position.y.clamp(top, bottom);
                if position.distance_squared(vec2(nearest_x, nearest_y)) <= distance_squared {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(test)]
    fn has_clear_line_reference(&self, from: Vec2, to: Vec2) -> bool {
        let distance = from.distance(to);
        let step = self.cell_size().x.min(self.cell_size().y) * 0.5;
        let samples = (distance / step).ceil().max(1.0) as usize;
        (1..=samples).all(|index| !self.is_wall(from.lerp(to, index as f32 / samples as f32)))
    }

    pub fn paint_wall(&mut self, position: Vec2, enabled: bool) {
        self.paint_wall_brush(position, enabled, 0);
    }

    /// Pinta um disco de pixels lógicos. `radius_pixels = 0` preserva o
    /// comportamento antigo de um único pixel.
    pub fn paint_wall_brush(&mut self, position: Vec2, enabled: bool, radius_pixels: u8) {
        if let Some((x, y)) = self.cell_at(position) {
            let radius = radius_pixels as i32;
            for offset_y in -radius..=radius {
                for offset_x in -radius..=radius {
                    if offset_x * offset_x + offset_y * offset_y > radius * radius {
                        continue;
                    }
                    let target_x = x as i32 + offset_x;
                    let target_y = y as i32 + offset_y;
                    if target_x >= 0
                        && target_y >= 0
                        && target_x < MAP_GRID_WIDTH as i32
                        && target_y < MAP_GRID_HEIGHT as i32
                    {
                        let index = self.index(target_x as usize, target_y as usize);
                        self.walls[index] = enabled as u8;
                    }
                }
            }
        }
    }

    pub fn logical_pixel_size(&self) -> Vec2 {
        self.cell_size()
    }

    pub fn clear_sunlight(&mut self) {
        self.sunlight.fill(0);
    }

    pub fn set_sunlight_polygon(&mut self, polygon: &[Vec2]) {
        self.clear_sunlight();
        if polygon.len() < 3 {
            return;
        }
        for y in 0..MAP_GRID_HEIGHT {
            for x in 0..MAP_GRID_WIDTH {
                let index = self.index(x, y);
                self.sunlight[index] = point_in_polygon(self.cell_center(x, y), polygon) as u8;
            }
        }
    }

    pub fn has_sunlight_area(&self) -> bool {
        self.sunlight.iter().any(|&value| value != 0)
    }

    pub fn wall_cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.walls.iter().enumerate().filter_map(|(index, value)| {
            (*value != 0).then_some((index % MAP_GRID_WIDTH, index / MAP_GRID_WIDTH))
        })
    }

    pub fn sunlight_cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.sunlight
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                (*value != 0).then_some((index % MAP_GRID_WIDTH, index / MAP_GRID_WIDTH))
            })
    }

    pub fn constrain_to_walls(&self, position: Vec2, previous: Vec2, radius: f32) -> Vec2 {
        if !self.overlaps_wall(position, radius) {
            return position;
        }
        if !self.overlaps_wall(previous, radius) {
            return previous;
        }
        // Edição pode criar uma parede sobre uma célula existente. Nesse caso,
        // deslocamos para a primeira direção livre, em vez de prendê-la.
        let step = self.cell_size().x.min(self.cell_size().y) * 0.55;
        for direction in [
            vec2(1.0, 0.0),
            vec2(-1.0, 0.0),
            vec2(0.0, 1.0),
            vec2(0.0, -1.0),
            vec2(0.7, 0.7),
            vec2(-0.7, 0.7),
            vec2(0.7, -0.7),
            vec2(-0.7, -0.7),
        ] {
            let candidate = position + direction * step;
            if !self.overlaps_wall(candidate, radius) {
                return candidate;
            }
        }
        previous
    }

    /// Alternativa neutra para fundadores: evita concentrá-los no centro se
    /// uma categoria de terreno não tiver área disponível.
    pub fn random_free_point(&self, mut random: impl FnMut() -> f32, radius: f32) -> Vec2 {
        for _ in 0..480 {
            let position = vec2(
                radius + random() * (WORLD_WIDTH - 2.0 * radius),
                radius + random() * (WORLD_HEIGHT - 2.0 * radius),
            );
            if !self.overlaps_wall(position, radius) {
                return position;
            }
        }
        for y in 0..MAP_GRID_HEIGHT {
            for x in 0..MAP_GRID_WIDTH {
                let position = self.cell_center(x, y);
                if !self.overlaps_wall(position, radius) {
                    return position;
                }
            }
        }
        vec2(radius, radius)
    }

    pub fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(&self.walls)?;
        writer.write_all(&self.sunlight)
    }

    pub fn read_from(reader: &mut impl Read) -> io::Result<Self> {
        let mut map = Self::empty();
        reader.read_exact(&mut map.walls)?;
        reader.read_exact(&mut map.sunlight)?;
        if map.walls.iter().any(|value| *value > 1) || map.sunlight.iter().any(|value| *value > 1) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "máscara de mapa inválida",
            ));
        }
        Ok(map)
    }

    /// Lê mapas gravados quando o mundo tinha metade da largura e da altura.
    /// Cada pixel antigo é expandido para uma área 2×2, preservando a forma
    /// relativa ao migrar a câmera para a malha atual.
    pub fn read_legacy_from(reader: &mut impl Read) -> io::Result<Self> {
        let legacy_len = LEGACY_MAP_GRID_WIDTH * LEGACY_MAP_GRID_HEIGHT;
        let mut legacy_walls = vec![0; legacy_len];
        let mut legacy_sunlight = vec![0; legacy_len];
        reader.read_exact(&mut legacy_walls)?;
        reader.read_exact(&mut legacy_sunlight)?;
        if legacy_walls.iter().any(|value| *value > 1)
            || legacy_sunlight.iter().any(|value| *value > 1)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "máscara de mapa legado inválida",
            ));
        }
        let mut map = Self::empty();
        for legacy_y in 0..LEGACY_MAP_GRID_HEIGHT {
            for legacy_x in 0..LEGACY_MAP_GRID_WIDTH {
                let legacy_index = legacy_y * LEGACY_MAP_GRID_WIDTH + legacy_x;
                for y in 0..2 {
                    for x in 0..2 {
                        let target_x = legacy_x * 2 + x;
                        let target_y = legacy_y * 2 + y;
                        let target_index = map.index(target_x, target_y);
                        map.walls[target_index] = legacy_walls[legacy_index];
                        map.sunlight[target_index] = legacy_sunlight[legacy_index];
                    }
                }
            }
        }
        Ok(map)
    }

    pub fn cell_center(&self, x: usize, y: usize) -> Vec2 {
        vec2(
            (x as f32 + 0.5) / MAP_GRID_WIDTH as f32 * WORLD_WIDTH,
            (y as f32 + 0.5) / MAP_GRID_HEIGHT as f32 * WORLD_HEIGHT,
        )
    }

    fn overlaps_wall(&self, position: Vec2, radius: f32) -> bool {
        if self.is_wall(position) {
            return true;
        }
        for direction in [
            vec2(1.0, 0.0),
            vec2(-1.0, 0.0),
            vec2(0.0, 1.0),
            vec2(0.0, -1.0),
            vec2(0.7, 0.7),
            vec2(-0.7, 0.7),
            vec2(0.7, -0.7),
            vec2(-0.7, -0.7),
        ] {
            if self.is_wall(position + direction * radius) {
                return true;
            }
        }
        false
    }

    fn cell_at(&self, position: Vec2) -> Option<(usize, usize)> {
        if !(0.0..WORLD_WIDTH).contains(&position.x) || !(0.0..WORLD_HEIGHT).contains(&position.y) {
            return None;
        }
        Some((
            (position.x / WORLD_WIDTH * MAP_GRID_WIDTH as f32) as usize,
            (position.y / WORLD_HEIGHT * MAP_GRID_HEIGHT as f32) as usize,
        ))
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * MAP_GRID_WIDTH + x
    }

    fn cell_size(&self) -> Vec2 {
        vec2(
            WORLD_WIDTH / MAP_GRID_WIDTH as f32,
            WORLD_HEIGHT / MAP_GRID_HEIGHT as f32,
        )
    }
}

// As duas entradas desenhadas no mapa: uma no noroeste e outra no sudeste.
const NORTHWEST_GATE_ANGLE: f32 = -2.02;
const SOUTHEAST_GATE_ANGLE: f32 = 0.82;
const GATE_HALF_ANGLE: f32 = 0.23;

pub fn is_sunlit(position: Vec2) -> bool {
    ellipse_level(position) <= 1.0
}

pub fn ellipse_level(position: Vec2) -> f32 {
    let offset = position - OASIS_CENTER;
    ((offset.x / OASIS_RADII.x).powi(2) + (offset.y / OASIS_RADII.y).powi(2)).sqrt()
}

pub fn ellipse_angle(position: Vec2) -> f32 {
    let offset = position - OASIS_CENTER;
    (offset.y / OASIS_RADII.y).atan2(offset.x / OASIS_RADII.x)
}

pub fn is_solid_wall_at_angle(angle: f32) -> bool {
    angular_distance(angle, NORTHWEST_GATE_ANGLE) > GATE_HALF_ANGLE
        && angular_distance(angle, SOUTHEAST_GATE_ANGLE) > GATE_HALF_ANGLE
}

/// Impede que um centro atravesse a parede oval; nas duas comportas ele passa.
/// O raio do corpo participa da margem, portanto células grandes não vazam pelo muro.
#[cfg(test)]
fn constrain_to_walls(position: Vec2, previous: Vec2, radius: f32) -> Vec2 {
    let previous_level = ellipse_level(previous);
    let current_level = ellipse_level(position);
    let angle = ellipse_angle(position);
    if !is_solid_wall_at_angle(angle) {
        return position;
    }

    let clearance = (WALL_HALF_THICKNESS + radius) / OASIS_RADII.y;
    let inside_limit = 1.0 - clearance;
    let outside_limit = 1.0 + clearance;
    let crossed_from_inside = previous_level <= 1.0 && current_level > inside_limit;
    let crossed_from_outside = previous_level > 1.0 && current_level < outside_limit;
    let started_in_wall = current_level >= inside_limit && current_level <= outside_limit;
    if !(crossed_from_inside || crossed_from_outside || started_in_wall) {
        return position;
    }

    let keep_inside = if (previous_level - 1.0).abs() < 0.001 {
        current_level < 1.0
    } else {
        previous_level < 1.0
    };
    point_on_ellipse(
        angle,
        if keep_inside {
            inside_limit
        } else {
            outside_limit
        },
    )
}

#[cfg(test)]
fn point_on_ellipse(angle: f32, scale: f32) -> Vec2 {
    OASIS_CENTER
        + vec2(
            angle.cos() * OASIS_RADII.x * scale,
            angle.sin() * OASIS_RADII.y * scale,
        )
}

fn angular_distance(left: f32, right: f32) -> f32 {
    let difference = (left - right + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    difference.abs()
}

fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let mut inside = false;
    for index in 0..polygon.len() {
        let current = polygon[index];
        let previous = polygon[(index + polygon.len() - 1) % polygon.len()];
        let crosses = (current.y > point.y) != (previous.y > point.y)
            && point.x
                < (previous.x - current.x) * (point.y - current.y) / (previous.y - current.y)
                    + current.x;
        if crosses {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oasis_center_is_sunlit_and_corner_is_unlit() {
        assert!(is_sunlit(OASIS_CENTER));
        assert!(!is_sunlit(Vec2::ZERO));
    }

    #[test]
    fn wall_blocks_crossing_but_gate_allows_it() {
        let wall_angle = 0.0;
        let blocked = constrain_to_walls(
            point_on_ellipse(wall_angle, 1.1),
            point_on_ellipse(wall_angle, 0.9),
            5.0,
        );
        assert!(ellipse_level(blocked) < 1.0);

        let allowed = constrain_to_walls(
            point_on_ellipse(NORTHWEST_GATE_ANGLE, 1.1),
            point_on_ellipse(NORTHWEST_GATE_ANGLE, 0.9),
            5.0,
        );
        assert!(ellipse_level(allowed) > 1.0);
    }

    #[test]
    fn lasso_replaces_the_sunlight_mask() {
        let mut map = MapDefinition::empty();
        let polygon = [
            vec2(1_000.0, 1_000.0),
            vec2(2_000.0, 1_000.0),
            vec2(2_000.0, 2_000.0),
            vec2(1_000.0, 2_000.0),
        ];
        map.set_sunlight_polygon(&polygon);

        assert!(map.is_sunlit(vec2(1_500.0, 1_500.0)));
        assert!(!map.is_sunlit(vec2(3_000.0, 1_500.0)));
    }

    #[test]
    fn optimized_line_of_sight_keeps_the_original_sampling_decision() {
        let map = MapDefinition::default();
        let mut state = 0x4d59_5df4_d0f3_3173u64;
        let next_unit = |state: &mut u64| {
            *state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (*state >> 40) as f32 / (1u32 << 24) as f32
        };
        for _ in 0..1_024 {
            let from = vec2(
                next_unit(&mut state) * WORLD_WIDTH,
                next_unit(&mut state) * WORLD_HEIGHT,
            );
            let to = vec2(
                next_unit(&mut state) * WORLD_WIDTH,
                next_unit(&mut state) * WORLD_HEIGHT,
            );
            assert_eq!(
                map.has_clear_line(from, to),
                map.has_clear_line_reference(from, to),
                "a linha entre {from:?} e {to:?} mudou de resultado"
            );
        }
    }
}
