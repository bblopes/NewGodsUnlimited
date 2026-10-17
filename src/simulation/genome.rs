use macroquad::prelude::*;

use crate::{
    config::{DEFAULT_FOOD_SENSOR_RANGE, MAX_FOOD_SENSOR_RANGE, MIN_FOOD_SENSOR_RANGE},
    simulation::brain::{DecodedConnection, INTERNAL_START, MOTOR_START, SLOT_COUNT},
};

pub const CONNECTION_RECORDS: usize = 64;
pub const BITS_PER_RECORD: usize = 19;
/// Bytes gravados por versões anteriores ao gene de alcance sensorial.
pub const PRE_FOOD_SENSOR_RANGE_GENOME_BYTES: usize = 152;
/// Cinco bits extras carregam o alcance do sensor de alimento. Os três bits
/// baixos do último byte são reserva e não entram em mutação/similaridade.
pub const GENOME_BYTES: usize = 153;
pub const LEGACY_V1_GENOME_BYTES: usize = 136;
pub const MUTATION_DENOMINATOR: u32 = 1_024;
const NEURAL_GENOME_BITS: usize = PRE_FOOD_SENSOR_RANGE_GENOME_BYTES * 8;
const FOOD_SENSOR_ALLELE_BITS: usize = 5;
pub const GENOME_BIT_COUNT: usize = NEURAL_GENOME_BITS + FOOD_SENSOR_ALLELE_BITS;
const PHEROMONE_CHANNEL_OFFSET: usize = NEURAL_GENOME_BITS - 9;
const FOOD_SENSOR_ALLELE_OFFSET: usize = NEURAL_GENOME_BITS;
const DEFAULT_FOOD_SENSOR_ALLELE: u32 = 16;

/// Genoma compacto: 64 registros `[origem:6][destino:6][peso:6][ativo:1]`
/// mais um alelo sensorial. O estado neural não está aqui; somente informação
/// hereditária, cuja projeção também define cor, similaridade e PG.
#[derive(Clone)]
pub struct Genome {
    bits: [u8; GENOME_BYTES],
}

impl Genome {
    pub fn random(random_state: &mut u64) -> Self {
        let mut genome = Self {
            bits: [0; GENOME_BYTES],
        };
        let mut used_pairs = Vec::with_capacity(CONNECTION_RECORDS / 2);

        // Parte ativa inicial: conexões legais, sem pares repetidos e pesos
        // discretos de -1,00 a +1,00 em passos de 0,125.
        for record_index in 0..CONNECTION_RECORDS / 2 {
            let (source, destination) = loop {
                let source = next_u32(random_state) as usize % MOTOR_START;
                let destination = INTERNAL_START
                    + next_u32(random_state) as usize % (SLOT_COUNT - INTERNAL_START);
                if !used_pairs.contains(&(source, destination)) {
                    break (source, destination);
                }
            };
            used_pairs.push((source, destination));
            let weight_steps = (next_u32(random_state) % 17) as i8 - 8;
            genome.write_record(record_index, source, destination, weight_steps, true);
        }

        // Parte inativa: bits variam livremente, mas a chave ativo fica em 0.
        // Mutações futuras podem tornar registros inválidos ativos; eles serão
        // ignorados pelo decodificador, sem comprometer o restante da rede.
        for record_index in CONNECTION_RECORDS / 2..CONNECTION_RECORDS {
            genome.write_record(
                record_index,
                (next_u32(random_state) & 63) as usize,
                (next_u32(random_state) & 63) as usize,
                (next_u32(random_state) & 63) as i8,
                false,
            );
        }
        genome.write_bits(
            FOOD_SENSOR_ALLELE_OFFSET,
            FOOD_SENSOR_ALLELE_BITS,
            DEFAULT_FOOD_SENSOR_ALLELE,
        );
        genome
    }

    /// Variante usada apenas na população inicial do modo "Motor Mínima".
    /// O resto do genoma permanece aleatório; quatro registros são reservados
    /// para garantir o caminho direto sensor de alimento -> motor direcional.
    /// Os pesos continuam aleatórios (inclusive o sinal), como no restante da
    /// população. Filhos não passam por este construtor: mutam normalmente.
    pub fn random_with_motor_minimum(random_state: &mut u64) -> Self {
        let mut genome = Self::random(random_state);
        let direct_paths = [
            (0, MOTOR_START),     // alimento à esquerda -> nadar à esquerda
            (1, MOTOR_START + 1), // alimento à direita -> nadar à direita
            (2, MOTOR_START + 2), // alimento acima -> nadar acima
            (3, MOTOR_START + 3), // alimento abaixo -> nadar abaixo
        ];

        for (record_index, (source, destination)) in direct_paths.into_iter().enumerate() {
            let weight_steps = (next_u32(random_state) % 17) as i8 - 8;
            genome.write_record(record_index, source, destination, weight_steps, true);
        }
        genome
    }

    /// Amarração de fundadores Val 2.0. Ela parte da população validada, mas
    /// reserva quatro registros para navegação direta até alimento e seis
    /// para o portão de doação. Os registros continuam mutáveis nos filhos.
    pub fn with_validated_v2_cooperation(mut self) -> Self {
        let gate = INTERNAL_START;
        let donate = SLOT_COUNT - 1;
        let paths = [
            (0, MOTOR_START, 8),     // alimento esquerda -> nadar esquerda
            (1, MOTOR_START + 1, 8), // alimento direita -> nadar direita
            (2, MOTOR_START + 2, 8), // alimento acima -> nadar acima
            (3, MOTOR_START + 3, 8), // alimento abaixo -> nadar abaixo
            (8, gate, 8),            // Ct positivo
            (9, gate, 8),            // PG positivo
            (7, gate, -8),           // viés: um sinal sozinho não basta
            (gate, donate, 8),
            (gate, donate, 8),
            (gate, donate, 8),
        ];
        for (record, (source, destination, weight)) in paths.into_iter().enumerate() {
            self.write_record(record, source, destination, weight, true);
        }
        self
    }

    /// Migra uma run da topologia química anterior (52 slots). O índice 52
    /// não existia nela: registros que o mencionavam eram inválidos e nunca
    /// influenciaram o cérebro. Ao abrir a run na topologia de 53 slots, eles
    /// precisam continuar inativos para não virarem, por acidente, o motor
    /// novo `Doar`.
    pub fn from_pre_donation_bytes(bits: [u8; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES]) -> Self {
        let mut genome = Self::from_pre_food_sensor_range_bytes(bits);
        for record_index in 0..CONNECTION_RECORDS {
            let (source, destination, weight, active) = genome.read_record(record_index);
            if active && (source >= SLOT_COUNT - 1 || destination >= SLOT_COUNT - 1) {
                genome.write_record(record_index, source, destination, weight, false);
            }
        }
        genome
    }

    pub fn mutated_from(parent: &Self, random_state: &mut u64) -> Self {
        let mut child = parent.clone();
        for bit_index in 0..GENOME_BIT_COUNT {
            if next_u32(random_state) % MUTATION_DENOMINATOR == 0 {
                child.flip_bit(bit_index);
            }
        }
        child
    }

    pub fn decode_connections(&self) -> Vec<DecodedConnection> {
        let mut connections = Vec::with_capacity(CONNECTION_RECORDS / 2);
        for record_index in 0..CONNECTION_RECORDS {
            let (source, destination, weight_steps, active) = self.read_record(record_index);
            if active && is_legal_connection(source, destination) {
                connections.push(DecodedConnection {
                    source,
                    destination,
                    weight: weight_steps as f32 / 8.0,
                });
            }
        }
        connections
    }

    pub fn binary_string(&self) -> String {
        let mut binary = String::with_capacity(GENOME_BIT_COUNT);
        for bit_index in 0..GENOME_BIT_COUNT {
            binary.push(if self.bit(bit_index) { '1' } else { '0' });
        }
        binary
    }

    /// Representação compacta usada por telemetria agregada. Não deve ser
    /// modificada fora do genoma.
    pub fn bytes(&self) -> &[u8; GENOME_BYTES] {
        &self.bits
    }

    /// Reconstrói um genoma salvo sem reinterpretar ou mutar seus bits.
    pub fn from_bytes(bits: [u8; GENOME_BYTES]) -> Self {
        Self { bits }
    }

    /// Migra o genoma de 1.216 bits: preserva a fiação e os canais químicos,
    /// preenchendo o novo alelo com o alcance histórico de 240 unidades.
    pub fn from_pre_food_sensor_range_bytes(
        bits: [u8; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES],
    ) -> Self {
        let mut genome = Self::empty_with_default_sensor();
        genome.bits[..PRE_FOOD_SENSOR_RANGE_GENOME_BYTES].copy_from_slice(&bits);
        genome.write_bits(
            FOOD_SENSOR_ALLELE_OFFSET,
            FOOD_SENSOR_ALLELE_BITS,
            DEFAULT_FOOD_SENSOR_ALLELE,
        );
        genome
    }

    /// Fenótipo hereditário do alcance de alimento. A curva logarítmica deixa
    /// aumentos e reduções proporcionais comparáveis e mantém o alelo padrão
    /// exatamente no alcance histórico de 240 unidades.
    pub fn food_sensor_range(&self) -> f32 {
        let allele = self.read_bits(FOOD_SENSOR_ALLELE_OFFSET, FOOD_SENSOR_ALLELE_BITS) as f32;
        let exponent =
            (allele - DEFAULT_FOOD_SENSOR_ALLELE as f32) / DEFAULT_FOOD_SENSOR_ALLELE as f32;
        (DEFAULT_FOOD_SENSOR_RANGE * 2.0_f32.powf(exponent))
            .clamp(MIN_FOOD_SENSOR_RANGE, MAX_FOOD_SENSOR_RANGE)
    }

    /// Migra genomas gravados antes da expansão química. A topologia antiga
    /// é preservada: Sm passa a ler PG; Fx/Fy e todos internos/motores são
    /// deslocados para seus slots novos.
    pub fn from_pre_pheromone_bytes(bits: [u8; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES]) -> Self {
        let mut genome = Self::empty_with_default_sensor();
        for record_index in 0..CONNECTION_RECORDS {
            let offset = record_index * BITS_PER_RECORD;
            let source = read_bits_from(&bits, offset, 6) as usize;
            let destination = read_bits_from(&bits, offset + 6, 6) as usize;
            let encoded_weight = read_bits_from(&bits, offset + 12, 6) as i8;
            let weight_steps = if encoded_weight & 0b10_0000 != 0 {
                encoded_weight - 64
            } else {
                encoded_weight
            };
            let active = read_bits_from(&bits, offset + 18, 1) != 0;
            genome.write_record(
                record_index,
                remap_pre_pheromone_slot(source),
                remap_pre_pheromone_slot(destination),
                weight_steps,
                active,
            );
        }
        genome
    }

    /// Converte o genoma usado pela versão com 10 sensores e 5 motores para
    /// os dois sensores de força e o motor de fagocitose atuais. Os bits da
    /// conexão são preservados; apenas os índices de slots posteriores aos
    /// sensores antigos são deslocados em dois posições.
    pub fn from_legacy_v2_bytes(bits: [u8; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES]) -> Self {
        let mut genome = Self::empty_with_default_sensor();
        for record_index in 0..CONNECTION_RECORDS {
            let offset = record_index * BITS_PER_RECORD;
            let source = read_bits_from(&bits, offset, 6) as usize;
            let destination = read_bits_from(&bits, offset + 6, 6) as usize;
            let encoded_weight = read_bits_from(&bits, offset + 12, 6) as i8;
            let weight_steps = if encoded_weight & 0b10_0000 != 0 {
                encoded_weight - 64
            } else {
                encoded_weight
            };
            let active = read_bits_from(&bits, offset + 18, 1) != 0;
            genome.write_record(
                record_index,
                remap_legacy_v2_slot(source),
                remap_legacy_v2_slot(destination),
                weight_steps,
                active,
            );
        }
        genome
    }

    /// Converte o genoma salvo antes dos sensores de contato. Os slots
    /// internos e motores antigos são deslocados em dois índices, sem mudar
    /// a topologia funcional herdada.
    pub fn from_legacy_v1_bytes(bits: [u8; LEGACY_V1_GENOME_BYTES]) -> Self {
        let mut genome = Self::empty_with_default_sensor();
        for record_index in 0..CONNECTION_RECORDS {
            let offset = record_index * 17;
            let source = read_bits_from(&bits, offset, 5) as usize;
            let destination = read_bits_from(&bits, offset + 5, 5) as usize;
            let encoded_weight = read_bits_from(&bits, offset + 10, 6) as i8;
            let weight_steps = if encoded_weight & 0b10_0000 != 0 {
                encoded_weight - 64
            } else {
                encoded_weight
            };
            let active = read_bits_from(&bits, offset + 16, 1) != 0;
            genome.write_record(
                record_index,
                remap_legacy_v1_slot(source),
                remap_legacy_v1_slot(destination),
                weight_steps,
                active,
            );
        }
        genome
    }

    /// Similaridade exata de Hamming entre dois genomas, no intervalo [0, 1].
    pub fn similarity_to(&self, other: &Self) -> f32 {
        let different_bits = (0..GENOME_BIT_COUNT)
            .filter(|&bit_index| self.bit(bit_index) != other.bit(bit_index))
            .count();
        1.0 - different_bits as f32 / GENOME_BIT_COUNT as f32
    }

    /// Cor de linhagem derivada continuamente de todos os bits hereditários.
    /// cada bit tem influência pequena, portanto mutações leves preservam a cor.
    pub fn genetic_rgb(&self) -> [u8; 3] {
        let projection_x = self.color_projection(0xA24B_AED4_963E_E407);
        let projection_y = self.color_projection(0x9FB2_1C65_1E98_DF25);
        let projection_light = self.color_projection(0xC13F_A9A9_02A6_328F);

        let hue = projection_y
            .atan2(projection_x)
            .to_degrees()
            .rem_euclid(360.0);
        let chroma = (projection_x * projection_x + projection_y * projection_y).sqrt();
        let saturation = 0.43 + 0.34 * (15.0 * chroma).tanh();
        let lightness = 0.60 + 0.12 * (13.0 * projection_light).tanh();
        hsl_to_rgb(hue, saturation, lightness)
    }

    pub fn genetic_hex(&self) -> String {
        let [red, green, blue] = self.genetic_rgb();
        format!("#{red:02X}{green:02X}{blue:02X}")
    }

    /// Três IDs químicos globais escolhidos por uma região regulatória do
    /// genoma. Colisões são normalizadas para que P1/P2/P3 sempre apontem a
    /// canais distintos.
    pub fn pheromone_channels(&self) -> [u8; 3] {
        let mut channels = [
            self.read_bits(PHEROMONE_CHANNEL_OFFSET, 3) as u8,
            self.read_bits(PHEROMONE_CHANNEL_OFFSET + 3, 3) as u8,
            self.read_bits(PHEROMONE_CHANNEL_OFFSET + 6, 3) as u8,
        ];
        for index in 1..channels.len() {
            while channels[..index].contains(&channels[index]) {
                channels[index] = (channels[index] + 1) % 8;
            }
        }
        channels
    }

    /// Assinatura contínua da linhagem usada pelo feromônio PG. Ela é uma
    /// projeção dos bits, por isso mutações pequenas só deslocam levemente o
    /// sinal químico, como já acontece com a cor genética.
    pub fn pheromone_signature(&self) -> Vec2 {
        let signature = vec2(
            self.color_projection(0xE1A3_7C5D_92F0_4B16),
            self.color_projection(0x7B10_CE34_6D8A_215F),
        );
        if signature.length_squared() < 0.000_001 {
            vec2(1.0, 0.0)
        } else {
            signature.normalize()
        }
    }

    fn color_projection(&self, salt: u64) -> f32 {
        let mut sum = 0i32;
        for bit_index in 0..GENOME_BIT_COUNT {
            let gene_sign = if self.bit(bit_index) { 1 } else { -1 };
            let weight_sign = if mix_bit(bit_index as u64 ^ salt) & 1 == 0 {
                1
            } else {
                -1
            };
            sum += gene_sign * weight_sign;
        }
        sum as f32 / GENOME_BIT_COUNT as f32
    }

    fn empty_with_default_sensor() -> Self {
        let mut genome = Self {
            bits: [0; GENOME_BYTES],
        };
        genome.write_bits(
            FOOD_SENSOR_ALLELE_OFFSET,
            FOOD_SENSOR_ALLELE_BITS,
            DEFAULT_FOOD_SENSOR_ALLELE,
        );
        genome
    }

    fn read_record(&self, record_index: usize) -> (usize, usize, i8, bool) {
        let offset = record_index * BITS_PER_RECORD;
        let source = self.read_bits(offset, 6) as usize;
        let destination = self.read_bits(offset + 6, 6) as usize;
        let encoded_weight = self.read_bits(offset + 12, 6) as i8;
        let weight_steps = if encoded_weight & 0b10_0000 != 0 {
            encoded_weight - 64
        } else {
            encoded_weight
        };
        let active = self.read_bits(offset + 18, 1) != 0;
        (source, destination, weight_steps, active)
    }

    fn write_record(
        &mut self,
        record_index: usize,
        source: usize,
        destination: usize,
        weight_steps: i8,
        active: bool,
    ) {
        let offset = record_index * BITS_PER_RECORD;
        self.write_bits(offset, 6, source as u32);
        self.write_bits(offset + 6, 6, destination as u32);
        self.write_bits(offset + 12, 6, (weight_steps as u8 & 0b11_1111) as u32);
        self.write_bits(offset + 18, 1, active as u32);
    }

    fn read_bits(&self, offset: usize, length: usize) -> u32 {
        let mut value = 0;
        for bit_index in offset..offset + length {
            value = (value << 1) | self.bit(bit_index) as u32;
        }
        value
    }

    fn write_bits(&mut self, offset: usize, length: usize, value: u32) {
        for relative_index in 0..length {
            let shift = length - 1 - relative_index;
            self.set_bit(offset + relative_index, (value >> shift) & 1 != 0);
        }
    }

    fn bit(&self, bit_index: usize) -> bool {
        let byte_index = bit_index / 8;
        let mask = 1 << (7 - bit_index % 8);
        self.bits[byte_index] & mask != 0
    }

    fn set_bit(&mut self, bit_index: usize, value: bool) {
        let byte_index = bit_index / 8;
        let mask = 1 << (7 - bit_index % 8);
        if value {
            self.bits[byte_index] |= mask;
        } else {
            self.bits[byte_index] &= !mask;
        }
    }

    fn flip_bit(&mut self, bit_index: usize) {
        let byte_index = bit_index / 8;
        self.bits[byte_index] ^= 1 << (7 - bit_index % 8);
    }
}

fn is_legal_connection(source: usize, destination: usize) -> bool {
    source < MOTOR_START && (INTERNAL_START..SLOT_COUNT).contains(&destination)
}

fn remap_legacy_v1_slot(slot: usize) -> usize {
    remap_legacy_v2_slot(if slot >= 8 { slot + 2 } else { slot })
}

fn remap_legacy_v2_slot(slot: usize) -> usize {
    // A versão anterior tinha sensores 0..9 e internos a partir de 10.
    // Agora Fcx/Fcy ocupam 10/11, deslocando internos e motores.
    if slot >= 10 { slot + 2 } else { slot }
}

fn remap_pre_pheromone_slot(slot: usize) -> usize {
    match slot {
        0..=9 => slot,
        10 => 21,
        11 => 22,
        _ => slot + 11,
    }
}

fn read_bits_from(bits: &[u8], offset: usize, length: usize) -> u32 {
    let mut value = 0;
    for bit_index in offset..offset + length {
        let byte_index = bit_index / 8;
        let mask = 1 << (7 - bit_index % 8);
        value = (value << 1) | (bits[byte_index] & mask != 0) as u32;
    }
    value
}

fn next_u32(random_state: &mut u64) -> u32 {
    *random_state = random_state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    (*random_state >> 32) as u32
}

fn mix_bit(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> [u8; 3] {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_section = hue / 60.0;
    let secondary = chroma * (1.0 - (hue_section.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match hue_section as u32 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let offset = lightness - chroma / 2.0;
    [red, green, blue].map(|channel| ((channel + offset) * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_genome_decodes_to_half_of_its_records_as_active_connections() {
        let mut state = 42;
        let genome = Genome::random(&mut state);
        assert_eq!(genome.decode_connections().len(), CONNECTION_RECORDS / 2);
    }

    #[test]
    fn mutation_keeps_the_compact_genome_size() {
        let mut state = 42;
        let parent = Genome::random(&mut state);
        let child = Genome::mutated_from(&parent, &mut state);
        assert_eq!(child.bits.len(), GENOME_BYTES);
    }

    #[test]
    fn motor_minimum_keeps_the_four_direct_food_paths_active() {
        let mut state = 910;
        let genome = Genome::random_with_motor_minimum(&mut state);
        let connections = genome.decode_connections();

        for (source, destination) in [
            (0, MOTOR_START),
            (1, MOTOR_START + 1),
            (2, MOTOR_START + 2),
            (3, MOTOR_START + 3),
        ] {
            assert!(connections.iter().any(|connection| {
                connection.source == source && connection.destination == destination
            }));
        }
    }

    #[test]
    fn food_sensor_allele_is_hereditary_and_changes_genetic_readings() {
        let mut state = 911;
        let baseline = Genome::random(&mut state);
        let mut short_range = baseline.clone();
        let mut long_range = baseline.clone();
        short_range.write_bits(FOOD_SENSOR_ALLELE_OFFSET, FOOD_SENSOR_ALLELE_BITS, 0);
        long_range.write_bits(FOOD_SENSOR_ALLELE_OFFSET, FOOD_SENSOR_ALLELE_BITS, 31);

        assert!((baseline.food_sensor_range() - DEFAULT_FOOD_SENSOR_RANGE).abs() < 0.001);
        assert!(short_range.food_sensor_range() < DEFAULT_FOOD_SENSOR_RANGE);
        assert!(long_range.food_sensor_range() > DEFAULT_FOOD_SENSOR_RANGE);
        assert!(short_range.similarity_to(&long_range) < 1.0);
        assert_ne!(
            short_range.pheromone_signature(),
            long_range.pheromone_signature()
        );
    }

    #[test]
    fn old_genome_migrates_to_the_historical_sensor_range() {
        let legacy = [0b1010_0101; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES];
        let migrated = Genome::from_pre_food_sensor_range_bytes(legacy);

        assert_eq!(
            &migrated.bytes()[..PRE_FOOD_SENSOR_RANGE_GENOME_BYTES],
            &legacy
        );
        assert!((migrated.food_sensor_range() - DEFAULT_FOOD_SENSOR_RANGE).abs() < 0.001);
    }
}
