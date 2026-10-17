use std::{
    collections::{HashMap, HashSet},
    io::{self, Read, Write},
    time::{Duration, Instant},
};

use macroquad::prelude::*;

use crate::{
    config::{
        DEFAULT_FOOD_SENSOR_RANGE, MAX_FLOW_SPEED, MAX_FOOD_SENSOR_RANGE, WORLD_HEIGHT, WORLD_WIDTH,
    },
    environment::{
        flow::FlowField,
        map::{MapDefinition, Terrain},
    },
    simulation::brain::{Brain, MotorOutput, SensorInput},
    simulation::genome::{
        Genome, PRE_FOOD_SENSOR_RANGE_GENOME_BYTES, PRE_NEURAL_NODE_ACTIVITY_GENOME_BYTES,
    },
    simulation::pheromone::{PheromoneField, pheromone_energy_cost},
};

pub const FOOD_RADIUS: f32 = 5.0;
pub const CELL_RADIUS: f32 = 14.0;
/// Referência da curva corporal: com esta energia o raio chega a 2R.
/// Acima desse ponto o corpo continua crescendo, mas pela raiz da área.
pub const SIZE_GROWTH_ENERGY: f32 = 2_000.0;
/// Um corpo com 2R paga 1,40× de custo físico. A curva segue sem teto para
/// corpos maiores, mas cresce de modo sublinear para não penalizá-los demais.
pub const MAX_BODY_COST_MULTIPLIER: f32 = 1.40;
const BODY_COST_RADIUS_EXPONENT: f32 = 0.485_427;
pub const STARTING_ENERGY: f32 = 60.0;
pub const MAX_ENERGY: f32 = 120.0;
pub const FOOD_ENERGY: f32 = 50.0;
pub const BASAL_ENERGY_COST: f32 = 0.03;
pub const MAX_MOTOR_ENERGY_COST: f32 = 0.20;
pub const DEFAULT_MENTAL_ENERGY_COST: f32 = 0.0025;
/// Manter um neurônio interno ou motor habilitado custa por tick, mesmo sem
/// disparo. A fiação efetiva continua usando o slider mental da sessão.
pub const ACTIVE_NEURON_ENERGY_COST: f32 = 0.0015;
/// Manter o sensor de alimento padrão (240 unidades) ativo custa 0,006 por
/// tick. Alcances hereditários maiores escalam pelo quadrado da distância.
pub const DEFAULT_FOOD_SENSOR_ENERGY_COST: f32 = 0.006;
/// A longevidade é herdável, mas fica fora dos bits de fiação neural para não
/// contaminar cor, similaridade ou assinatura química de parentesco.
/// Janela curta para tornar a pressão de envelhecimento observável numa run.
/// O alelo ainda varia e é herdado, mas nenhum organismo ultrapassa 5 mil ticks.
pub const MIN_LIFESPAN_TICKS: u64 = 1_000;
pub const DEFAULT_LIFESPAN_TICKS: u64 = 3_000;
/// O alelo comporta linhagens longevas. Só os fundadores Val 2.0 começam
/// nessa ponta; as demais configurações continuam no valor-base curto.
pub const MAX_LIFESPAN_TICKS: u64 = 11_000;
const VALIDATED_V2_FOUNDER_LIFESPAN_MIN_TICKS: u64 = 10_000;
const VALIDATED_V2_FOUNDER_LIFESPAN_MAX_TICKS: u64 = 11_000;
/// Quanto maior, mais tardio é o colapso: a perda fica quase invisível no
/// começo e se intensifica muito perto do limite hereditário.
const SENESCENCE_CURVATURE: f32 = 8.0;
/// Acima deste nível a decisão da rede neural é convertida em reprodução.
pub const REPRODUCTION_ACTIVATION_THRESHOLD: f32 = 0.60;
const FOOD_FLOW_COUPLING: f32 = 0.42;
const CELL_FLOW_COUPLING: f32 = 1.0;
/// Vantagem geométrica mínima para uma célula conseguir fagocitar outra.
const PHAGOCYTOSIS_MIN_RADIUS_RATIO: f32 = 1.10;
/// Margem em que uma célula já pode começar a puxar outra, antes da
/// sobreposição visual. O engolfamento ainda exige contenção total do núcleo.
const PHAGOCYTOSIS_INTERACTION_MARGIN: f32 = 0.15;
const MAX_PHAGOCYTOSIS_PULL_PER_TICK: f32 = 2.0;
const MAX_PHAGOCYTOSIS_ENERGY_COST: f32 = 0.04;
/// Transferência máxima de energia por tick quando o motor Doar está em 1.
const MAX_DONATION_PER_TICK: f32 = 4.0;
/// O doador não pode transferir energia que o deixaria abaixo deste piso.
const DONATION_RESERVE_ENERGY: f32 = STARTING_ENERGY;
/// Perda operacional da transferência; não chega ao receptor.
const DONATION_OPERATIONAL_COST: f32 = 0.02;
/// Deslocamento celular correspondente a sinal ±1 dos sensores de força.
const FORCE_SENSOR_DISPLACEMENT_SCALE: f32 = 4.0;
pub const MIN_EXCLUSION_DISTANCE: f32 = CELL_RADIUS * 2.0;
pub const MAX_EXCLUSION_DISTANCE: f32 = CELL_RADIUS * 6.0;
const MAX_CRAMMING_RELAXATION_PASSES: usize = 6;

#[derive(Clone, Copy, PartialEq)]
pub struct AntiCrammingConfig {
    pub enabled: bool,
    pub force: f32,
    pub exclusion_distance: f32,
}

impl Default for AntiCrammingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            force: 1.0,
            exclusion_distance: MIN_EXCLUSION_DISTANCE,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct CrowdingMetrics {
    pub violating_pairs: usize,
    pub minimum_violation_distance: Option<f32>,
}

/// Tempos acumulados das fases de um tick. É preenchido somente por
/// `World::tick_profiled()`, usado pelo benchmark headless; `World::tick()`
/// normal não consulta o relógio nem altera o custo da simulação publicada.
#[derive(Clone, Copy, Default)]
pub struct TickProfile {
    pub clock: Duration,
    pub food_emission: Duration,
    pub pheromones: Duration,
    pub sensors: Duration,
    pub brain: Duration,
    pub food_movement: Duration,
    pub organism_movement: Duration,
    pub interactions: Duration,
    pub energy: Duration,
    pub deaths: Duration,
    pub feeding: Duration,
    pub food_cleanup: Duration,
    pub reproduction: Duration,
}

impl TickProfile {
    pub fn merge(&mut self, other: &Self) {
        self.clock += other.clock;
        self.food_emission += other.food_emission;
        self.pheromones += other.pheromones;
        self.sensors += other.sensors;
        self.brain += other.brain;
        self.food_movement += other.food_movement;
        self.organism_movement += other.organism_movement;
        self.interactions += other.interactions;
        self.energy += other.energy;
        self.deaths += other.deaths;
        self.feeding += other.feeding;
        self.food_cleanup += other.food_cleanup;
        self.reproduction += other.reproduction;
    }

    pub fn total(&self) -> Duration {
        self.clock
            + self.food_emission
            + self.pheromones
            + self.sensors
            + self.brain
            + self.food_movement
            + self.organism_movement
            + self.interactions
            + self.energy
            + self.deaths
            + self.feeding
            + self.food_cleanup
            + self.reproduction
    }

    pub fn phases(&self) -> [(&'static str, Duration); 13] {
        [
            ("tempo", self.clock),
            ("emissão de alimento", self.food_emission),
            ("feromônios", self.pheromones),
            ("sensores", self.sensors),
            ("cérebro", self.brain),
            ("movimento do alimento", self.food_movement),
            ("movimento das células", self.organism_movement),
            ("interações", self.interactions),
            ("energia", self.energy),
            ("mortes", self.deaths),
            ("alimentação", self.feeding),
            ("limpeza de alimento", self.food_cleanup),
            ("reprodução", self.reproduction),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TickPhase {
    Clock,
    FoodEmission,
    Pheromones,
    Sensors,
    Brain,
    FoodMovement,
    OrganismMovement,
    Interactions,
    Energy,
    Deaths,
    Feeding,
    FoodCleanup,
    Reproduction,
}

trait TickObserver {
    fn begin(&mut self, phase: TickPhase);
    fn end(&mut self, phase: TickPhase);
}

struct NoopTickObserver;

impl TickObserver for NoopTickObserver {
    #[inline(always)]
    fn begin(&mut self, _: TickPhase) {}

    #[inline(always)]
    fn end(&mut self, _: TickPhase) {}
}

#[derive(Default)]
struct ProfileTickObserver {
    profile: TickProfile,
    started: Option<(TickPhase, Instant)>,
}

impl ProfileTickObserver {
    fn into_profile(mut self) -> TickProfile {
        debug_assert!(self.started.is_none());
        std::mem::take(&mut self.profile)
    }
}

impl TickObserver for ProfileTickObserver {
    fn begin(&mut self, phase: TickPhase) {
        debug_assert!(self.started.is_none());
        self.started = Some((phase, Instant::now()));
    }

    fn end(&mut self, phase: TickPhase) {
        let Some((started_phase, started_at)) = self.started.take() else {
            debug_assert!(false, "fase de benchmark encerrada sem início");
            return;
        };
        debug_assert_eq!(started_phase, phase);
        let elapsed = started_at.elapsed();
        match phase {
            TickPhase::Clock => self.profile.clock += elapsed,
            TickPhase::FoodEmission => self.profile.food_emission += elapsed,
            TickPhase::Pheromones => self.profile.pheromones += elapsed,
            TickPhase::Sensors => self.profile.sensors += elapsed,
            TickPhase::Brain => self.profile.brain += elapsed,
            TickPhase::FoodMovement => self.profile.food_movement += elapsed,
            TickPhase::OrganismMovement => self.profile.organism_movement += elapsed,
            TickPhase::Interactions => self.profile.interactions += elapsed,
            TickPhase::Energy => self.profile.energy += elapsed,
            TickPhase::Deaths => self.profile.deaths += elapsed,
            TickPhase::Feeding => self.profile.feeding += elapsed,
            TickPhase::FoodCleanup => self.profile.food_cleanup += elapsed,
            TickPhase::Reproduction => self.profile.reproduction += elapsed,
        }
    }
}

/// Regra usada somente quando a população inicial é construída. Não é uma
/// propriedade hereditária: após o primeiro nascimento, todos os descendentes
/// usam mutação normal sobre o genoma recebido.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialNeuralConfiguration {
    Classic,
    MotorMinimum,
    ValidatedV1,
    ValidatedV2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CheckpointFormat {
    V1,
    V2,
    V3,
    V4,
    PrePheromone,
    PreDonation,
    PreWorldExpansion,
    PreLongevity,
    /// V13 antes dos contadores separados de nascimento e morte.
    PreEventCounters,
    /// V14 antes do contador separado de morte por idade.
    PreAgeCounters,
    /// V15 antes do alelo hereditário de alcance do sensor de alimento.
    PreFoodSensorRange,
    /// V16 antes dos bits hereditários de atividade neural.
    PreNeuralNodeActivity,
    Current,
}

#[derive(Default)]
struct PhagocytosisOutcome {
    protected_pairs: HashSet<(usize, usize)>,
    capture_candidates: Vec<(usize, usize)>,
}

/// Uma única partícula de fitoplâncton. Ela só desaparece ao ser comida
/// (quando as células existirem) ou quando alcançar sua vida máxima.
#[derive(Clone, Copy)]
pub struct Phytoplankton {
    pub position: Vec2,
    pub age_ticks: u32,
}

/// Corpo físico da célula e os seus estados de decisão.
#[derive(Clone)]
pub struct Organism {
    pub id: u64,
    pub position: Vec2,
    pub energy: f32,
    pub age_ticks: u64,
    /// Alelo de longevidade: codifica a idade máxima independente do cérebro.
    pub longevity_gene: LongevityGene,
    pub generation: u32,
    pub children_produced: u32,
    pub crowding_violation: bool,
    /// Deslocamento causado por outras células no tick físico anterior.
    /// Não inclui nado próprio nem corrente ambiental.
    pub external_force: Vec2,
    pub motors: MotorOutput,
    pub sensors: SensorInput,
    pub brain: Brain,
    pub genome: Genome,
}

impl Organism {
    /// O corpo inicia em 1R e cresce sem teto. A energia cresce em área, logo
    /// o raio usa raiz quadrada: dobrar o raio exige quadruplicar a área.
    pub fn physical_radius(&self) -> f32 {
        physical_radius_for_energy(self.energy)
    }

    pub fn maximum_age_ticks(&self) -> u64 {
        self.longevity_gene.maximum_age_ticks()
    }

    /// Fração de eficiência metabólica restante. Chega a 0 no tick final,
    /// enquanto o multiplicador de custo usa um piso apenas para não dividir
    /// por zero antes de a remoção por idade ocorrer no mesmo tick.
    pub fn energy_efficiency(&self) -> f32 {
        let progress = self.age_ticks as f32 / self.maximum_age_ticks().max(1) as f32;
        let denominator = SENESCENCE_CURVATURE.exp() - 1.0;
        (1.0 - ((SENESCENCE_CURVATURE * progress.clamp(0.0, 1.0)).exp() - 1.0) / denominator)
            .clamp(0.0, 1.0)
    }

    fn energy_cost_multiplier(&self) -> f32 {
        1.0 / self.energy_efficiency().max(0.02)
    }

    fn physical_area_scale(&self) -> f32 {
        let radius_ratio = self.physical_radius() / CELL_RADIUS;
        radius_ratio.powf(BODY_COST_RADIUS_EXPONENT)
    }

    /// O corpo maior capta alimento com eficiência volumétrica, como na
    /// emissão química. O alelo hereditário multiplica essa capacidade.
    fn food_sensor_size_scale(&self) -> f32 {
        (self.physical_radius() / CELL_RADIUS).powi(3)
    }

    pub fn food_sensor_range(&self) -> f32 {
        self.genome.food_sensor_range() * self.food_sensor_size_scale()
    }

    /// O gene mais amplo cobra mais por órgão sensorial; crescer o corpo só
    /// cobra a área (r²), enquanto o alcance ganha o volume (r³). Isso torna
    /// o sensor mais eficiente por energia em células grandes.
    pub fn food_sensor_energy_cost(&self) -> f32 {
        let genetic_scale = self.genome.food_sensor_range() / DEFAULT_FOOD_SENSOR_RANGE;
        let body_area_scale = (self.physical_radius() / CELL_RADIUS).powi(2);
        DEFAULT_FOOD_SENSOR_ENERGY_COST * genetic_scale.powi(2) * body_area_scale
    }

    /// Corpos maiores têm mais massa/inércia: sua velocidade própria máxima
    /// diminui pela raiz do raio relativo, sem alterar a corrente ambiental.
    fn maximum_swim_speed(&self) -> f32 {
        crate::config::MAX_SWIM_SPEED / (self.physical_radius() / CELL_RADIUS).sqrt()
    }
}

/// Registro hereditário auxiliar de 16 bits. Ele é explicitamente excluído
/// das métricas de `Genome`: trata-se de fisiologia temporal, não de cérebro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LongevityGene {
    allele: u16,
}

impl Default for LongevityGene {
    fn default() -> Self {
        Self::from_age_ticks(DEFAULT_LIFESPAN_TICKS)
    }
}

impl LongevityGene {
    fn founder(random: f32, configuration: InitialNeuralConfiguration) -> Self {
        if configuration == InitialNeuralConfiguration::ValidatedV2 {
            let span =
                VALIDATED_V2_FOUNDER_LIFESPAN_MAX_TICKS - VALIDATED_V2_FOUNDER_LIFESPAN_MIN_TICKS;
            let age = VALIDATED_V2_FOUNDER_LIFESPAN_MIN_TICKS
                + (span as f32 * random.clamp(0.0, 1.0)).round() as u64;
            return Self::from_age_ticks(age);
        }
        // Fundadores já têm uma variação leve (±15%); evolução posterior
        // acontece por mutação binária deste alelo.
        let variation = 0.85 + random.clamp(0.0, 1.0) * 0.30;
        Self::from_age_ticks((DEFAULT_LIFESPAN_TICKS as f32 * variation).round() as u64)
    }

    fn mutated_from(parent: Self, chance: f32, bit_choice: f32) -> Self {
        // A probabilidade é propositalmente separada da mutação dos 1.216
        // bits neurais: um único bit de longevidade muda em média a cada 64
        // nascimentos, e não muda os indicadores do genoma neural.
        if chance < 1.0 / 64.0 {
            let bit = (bit_choice.clamp(0.0, 0.999_9) * 16.0) as u32;
            Self {
                allele: parent.allele ^ (1u16 << bit),
            }
        } else {
            parent
        }
    }

    pub fn maximum_age_ticks(self) -> u64 {
        let range = MAX_LIFESPAN_TICKS - MIN_LIFESPAN_TICKS;
        MIN_LIFESPAN_TICKS + (range * self.allele as u64) / u16::MAX as u64
    }

    fn from_age_ticks(age_ticks: u64) -> Self {
        let clamped = age_ticks.clamp(MIN_LIFESPAN_TICKS, MAX_LIFESPAN_TICKS);
        let range = MAX_LIFESPAN_TICKS - MIN_LIFESPAN_TICKS;
        Self {
            allele: (((clamped - MIN_LIFESPAN_TICKS) * u16::MAX as u64) / range) as u16,
        }
    }

    fn from_legacy_age(age_ticks: u64) -> Self {
        // Uma run antiga não tinha telômero. Ao restaurá-la, damos uma janela
        // de reprodução razoável acima da idade que ela já alcançou.
        Self::from_age_ticks(age_ticks.saturating_add(DEFAULT_LIFESPAN_TICKS))
    }

    fn allele(self) -> u16 {
        self.allele
    }

    fn from_allele(allele: u16) -> Self {
        Self { allele }
    }
}

/// Estado mutável da câmara. Não sabe desenhar nem toma decisões neurais.
#[derive(Clone)]
pub struct World {
    pub tick: u64,
    pub births_total: u64,
    pub deaths_total: u64,
    /// Nascimentos disparados pelo motor Reproduzir dentro da faixa viável.
    pub births_by_neural_decision_total: u64,
    /// Nascimentos disparados pelo limite físico obrigatório de energia.
    pub births_by_energy_limit_total: u64,
    /// Mortes cujo motivo foi energia zerada durante o metabolismo.
    pub deaths_by_starvation_total: u64,
    /// Mortes por fagocitose concluída por outra célula.
    pub deaths_by_phagocytosis_total: u64,
    /// Mortes ao alcançar o limite hereditário de longevidade.
    pub deaths_by_age_total: u64,
    pub energy_capacity: f32,
    pub food: Vec<Phytoplankton>,
    pub organisms: Vec<Organism>,
    pub map: MapDefinition,
    /// Fotografia química apenas do tick atual; não é persistida em saves.
    pub pheromones: PheromoneField,
    desert_spawn_credit: f32,
    fertile_spawn_credit: f32,
    next_organism_id: u64,
    random_state: u64,
    anti_cramming: AntiCrammingConfig,
    pub crowding: CrowdingMetrics,
}

impl World {
    pub fn new(seed: u64, initial_cells: usize) -> Self {
        Self::new_with_map(seed, initial_cells, MapDefinition::default())
    }

    pub fn new_with_map(seed: u64, initial_cells: usize, map: MapDefinition) -> Self {
        Self::new_with_initial_neural_configuration(
            seed,
            initial_cells,
            map,
            InitialNeuralConfiguration::Classic,
        )
    }

    pub fn new_with_initial_neural_configuration(
        seed: u64,
        initial_cells: usize,
        map: MapDefinition,
        initial_neural_configuration: InitialNeuralConfiguration,
    ) -> Self {
        Self::new_with_initial_population(
            seed,
            initial_cells,
            map,
            initial_neural_configuration,
            &[],
        )
    }

    /// Cria um mundo a partir de uma população fundadora opcional. A piscina
    /// é usada só no tick zero: os indivíduos começam em posições e energia
    /// novas, mas carregam genomas reais extraídos de uma run validada.
    pub fn new_with_initial_population(
        seed: u64,
        initial_cells: usize,
        map: MapDefinition,
        initial_neural_configuration: InitialNeuralConfiguration,
        validated_genomes: &[Genome],
    ) -> Self {
        let mut world = Self {
            tick: 0,
            births_total: 0,
            deaths_total: 0,
            births_by_neural_decision_total: 0,
            births_by_energy_limit_total: 0,
            deaths_by_starvation_total: 0,
            deaths_by_phagocytosis_total: 0,
            deaths_by_age_total: 0,
            energy_capacity: MAX_ENERGY,
            food: Vec::new(),
            organisms: Vec::new(),
            map,
            pheromones: PheromoneField::default(),
            desert_spawn_credit: 0.0,
            fertile_spawn_credit: 0.0,
            next_organism_id: 0,
            random_state: seed,
            anti_cramming: AntiCrammingConfig::default(),
            crowding: CrowdingMetrics::default(),
        };

        let mut validated_order: Vec<usize> = (0..validated_genomes.len()).collect();
        for index in (1..validated_order.len()).rev() {
            let swap_index = (world.next_random_unit() * (index + 1) as f32) as usize;
            validated_order.swap(index, swap_index.min(index));
        }
        for initial_index in 0..initial_cells {
            let inherited_genome = if matches!(
                initial_neural_configuration,
                InitialNeuralConfiguration::ValidatedV1 | InitialNeuralConfiguration::ValidatedV2
            ) {
                validated_order
                    .get(initial_index % validated_order.len().max(1))
                    .map(|index| validated_genomes[*index].clone())
            } else {
                None
            };
            world.spawn_organism(initial_neural_configuration, inherited_genome);
        }
        for index in 0..12 {
            let terrain = if index < 8 && world.map.has_fertile_area() {
                Terrain::Fertile
            } else {
                Terrain::Desert
            };
            world.spawn_food_in(terrain);
        }
        world
    }

    /// Modo Amadeus: uma única amarração neural aleatória, posicionada no
    /// centro geométrico da câmara. A linhagem posterior segue as regras
    /// normais de reprodução e mutação.
    pub fn new_amadeus_with_map(seed: u64, map: MapDefinition) -> Self {
        Self::new_amadeus_with_initial_neural_configuration(
            seed,
            map,
            InitialNeuralConfiguration::Classic,
        )
    }

    pub fn new_amadeus_with_initial_neural_configuration(
        seed: u64,
        map: MapDefinition,
        initial_neural_configuration: InitialNeuralConfiguration,
    ) -> Self {
        Self::new_amadeus_with_initial_population(seed, map, initial_neural_configuration, &[])
    }

    pub fn new_amadeus_with_initial_population(
        seed: u64,
        map: MapDefinition,
        initial_neural_configuration: InitialNeuralConfiguration,
        validated_genomes: &[Genome],
    ) -> Self {
        // Amadeus usa o medóide genético da população validada: o fundador
        // mais parecido, em média, com o grupo. Isso é determinístico e evita
        // escolher um outlier aleatório como demonstrador da linhagem.
        let representative: Vec<Genome> = validated_genomes
            .iter()
            .enumerate()
            .max_by(|(left_index, left), (right_index, right)| {
                let score = |candidate: &Genome| {
                    validated_genomes
                        .iter()
                        .map(|other| candidate.similarity_to(other))
                        .sum::<f32>()
                };
                score(left)
                    .total_cmp(&score(right))
                    .then_with(|| left_index.cmp(right_index))
            })
            .map(|(_, genome)| vec![genome.clone()])
            .unwrap_or_default();
        let mut world = Self::new_with_initial_population(
            seed,
            1,
            map,
            initial_neural_configuration,
            &representative,
        );
        if let Some(organism) = world.organisms.first_mut() {
            organism.position = vec2(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5);
        }
        world
    }

    /// Avança um tick lógico completo do ecossistema.
    ///
    /// Este é o orquestrador das regras biológicas: ele não desenha a tela e
    /// não decide quantos ticks por segundo serão calculados. `simulation_worker`
    /// em `app.rs` o chama; aqui definimos somente a ordem dos fenômenos de um
    /// único instante do mundo.
    ///
    /// Roteiro de leitura, na mesma ordem do código:
    /// 1. relógio e limites de reprodução; 2. emissão de alimento;
    /// 3. campo de feromônios; 4. sensores; 5. cérebro e motores;
    /// 6. movimento do alimento; 7. movimento das células;
    /// 8. interações físicas; 9. custo energético; 10. mortes;
    /// 11. alimentação; 12. remoção de alimento; 13. reprodução.
    ///
    /// A taxa de emissão é acumulada: 0,12/tick produz uma nova partícula a
    /// cada 8 ou 9 ticks, sem depender do FPS da tela.
    pub fn tick(
        &mut self,
        flow: &FlowField,
        desert_emission_per_tick: f32,
        fertile_emission_per_tick: f32,
        lifetime_ticks: u32,
        food_energy: f32,
        mental_energy_cost: f32,
        reproduction_minimum: f32,
        reproduction_limit: f32,
    ) {
        let mut observer = NoopTickObserver;
        self.tick_with_observer(
            flow,
            desert_emission_per_tick,
            fertile_emission_per_tick,
            lifetime_ticks,
            food_energy,
            mental_energy_cost,
            reproduction_minimum,
            reproduction_limit,
            &mut observer,
        );
    }

    /// Executa exatamente as mesmas regras de [`Self::tick`], medindo o
    /// tempo gasto em cada etapa. Destinado ao benchmark headless, nunca ao
    /// loop normal da interface.
    pub fn tick_profiled(
        &mut self,
        flow: &FlowField,
        desert_emission_per_tick: f32,
        fertile_emission_per_tick: f32,
        lifetime_ticks: u32,
        food_energy: f32,
        mental_energy_cost: f32,
        reproduction_minimum: f32,
        reproduction_limit: f32,
    ) -> TickProfile {
        let mut observer = ProfileTickObserver::default();
        self.tick_with_observer(
            flow,
            desert_emission_per_tick,
            fertile_emission_per_tick,
            lifetime_ticks,
            food_energy,
            mental_energy_cost,
            reproduction_minimum,
            reproduction_limit,
            &mut observer,
        );
        observer.into_profile()
    }

    fn tick_with_observer<Observer: TickObserver>(
        &mut self,
        flow: &FlowField,
        desert_emission_per_tick: f32,
        fertile_emission_per_tick: f32,
        lifetime_ticks: u32,
        food_energy: f32,
        mental_energy_cost: f32,
        reproduction_minimum: f32,
        reproduction_limit: f32,
        observer: &mut Observer,
    ) {
        // ── Etapa 1: tempo e limites desta rodada ────────────────────────
        observer.begin(TickPhase::Clock);
        self.tick += 1;
        let reproduction_minimum = reproduction_minimum.max(1.0);
        let reproduction_limit = reproduction_limit.max(reproduction_minimum);
        self.energy_capacity = reproduction_limit;
        observer.end(TickPhase::Clock);
        // ── Etapa 2: emissão de fitoplâncton ─────────────────────────────
        observer.begin(TickPhase::FoodEmission);
        // O crédito fracionário permite uma taxa média estável de emissão.
        self.desert_spawn_credit += desert_emission_per_tick.max(0.0);
        while self.desert_spawn_credit >= 1.0 {
            self.desert_spawn_credit -= 1.0;
            self.spawn_food_in(Terrain::Desert);
        }
        if self.map.has_fertile_area() {
            self.fertile_spawn_credit += fertile_emission_per_tick.max(0.0);
            while self.fertile_spawn_credit >= 1.0 {
                self.fertile_spawn_credit -= 1.0;
                self.spawn_food_in(Terrain::Fertile);
            }
        } else {
            self.fertile_spawn_credit = 0.0;
        }
        observer.end(TickPhase::FoodEmission);

        // ── Etapa 3: fotografia química ──────────────────────────────────
        // Primeiro construímos o campo químico a partir da emissão basal e
        // dos motores decididos no tick anterior. Assim, não há persistência
        // química, mas existe uma defasagem causal de um tick.
        observer.begin(TickPhase::Pheromones);
        self.pheromones.rebuild(&self.organisms, &self.map);
        observer.end(TickPhase::Pheromones);

        // ── Etapas 4 e 5: perceber e decidir ─────────────────────────────
        // Sensores veem o retrato do mundo no início do tick. O cérebro
        // decide usando este mesmo retrato antes do movimento.
        observer.begin(TickPhase::Sensors);
        self.update_sensors(flow, reproduction_limit);
        observer.end(TickPhase::Sensors);
        observer.begin(TickPhase::Brain);
        for organism in &mut self.organisms {
            organism.motors = organism.brain.tick(organism.sensors);
            organism.age_ticks += 1;
            // O sensor já leu a força do tick anterior. A partir daqui
            // acumulamos a força que ele perceberá no próximo tick.
            organism.external_force = Vec2::ZERO;
        }
        observer.end(TickPhase::Brain);

        // ── Etapa 6: deslocar e envelhecer o alimento ────────────────────
        observer.begin(TickPhase::FoodMovement);
        for particle in &mut self.food {
            let previous_position = particle.position;
            particle.position +=
                flow.sample_with_map(particle.position, Some(&self.map)) * FOOD_FLOW_COUPLING;
            particle.position.x = particle
                .position
                .x
                .clamp(FOOD_RADIUS, WORLD_WIDTH - FOOD_RADIUS);
            particle.position.y = particle
                .position
                .y
                .clamp(FOOD_RADIUS, WORLD_HEIGHT - FOOD_RADIUS);
            particle.position =
                self.map
                    .constrain_to_walls(particle.position, previous_position, FOOD_RADIUS);
            particle.age_ticks += 1;
        }
        observer.end(TickPhase::FoodMovement);

        // ── Etapa 7: deslocar as células ─────────────────────────────────
        // Posição nova = corrente ambiental + nado escolhido pelo cérebro.
        // O teto do nado próprio cai com o raio; a corrente segue física externa.
        observer.begin(TickPhase::OrganismMovement);
        for organism in &mut self.organisms {
            let previous_position = organism.position;
            let environment_velocity =
                flow.sample_with_map(organism.position, Some(&self.map)) * CELL_FLOW_COUPLING;
            organism.position +=
                environment_velocity + organism.motors.swim_velocity(organism.maximum_swim_speed());
            let radius = organism.physical_radius();
            organism.position.x =
                clamp_coordinate_to_world(organism.position.x, radius, WORLD_WIDTH);
            organism.position.y =
                clamp_coordinate_to_world(organism.position.y, radius, WORLD_HEIGHT);
            organism.position =
                self.map
                    .constrain_to_walls(organism.position, previous_position, radius);
        }
        observer.end(TickPhase::OrganismMovement);

        // ── Etapa 8: interações entre células ────────────────────────────
        // A fagocitose pode relaxar a repulsão somente para o par que está
        // em sucção válida. Todos os outros pares continuam anti-cramming.
        observer.begin(TickPhase::Interactions);
        let phagocytosis_pairs = self.apply_phagocytosis();
        self.apply_anti_cramming(&phagocytosis_pairs.protected_pairs);
        self.consume_phagocytosed(&phagocytosis_pairs.capture_candidates, reproduction_limit);
        self.apply_donations(reproduction_limit);
        observer.end(TickPhase::Interactions);

        // ── Etapa 9: cobrar o custo de existir e agir ────────────────────
        // O custo de existir e de nadar é pago antes do ganho alimentar.
        // Assim, alcançar o teto de energia após comer realmente dispara a divisão.
        observer.begin(TickPhase::Energy);
        for organism in &mut self.organisms {
            let body_cost_scale = organism.physical_area_scale();
            organism.energy -= ((BASAL_ENERGY_COST
                + organism.motors.energy_cost(MAX_MOTOR_ENERGY_COST))
                * body_cost_scale
                + organism
                    .brain
                    .mental_energy_cost(mental_energy_cost, ACTIVE_NEURON_ENERGY_COST)
                + pheromone_energy_cost(organism)
                + organism.food_sensor_energy_cost())
                * organism.energy_cost_multiplier();
        }
        observer.end(TickPhase::Energy);
        // ── Etapa 10: remover mortes por energia ou idade ────────────────
        observer.begin(TickPhase::Deaths);
        let population_before_deaths = self.organisms.len();
        let mut starvation_deaths = 0u64;
        let mut age_deaths = 0u64;
        self.organisms.retain(|organism| {
            let alive = organism.energy > 0.0 && organism.age_ticks < organism.maximum_age_ticks();
            if !alive {
                if organism.age_ticks >= organism.maximum_age_ticks() {
                    age_deaths += 1;
                } else if organism.energy <= 0.0 {
                    starvation_deaths += 1;
                }
            }
            alive
        });
        self.deaths_total += (population_before_deaths - self.organisms.len()) as u64;
        self.deaths_by_starvation_total += starvation_deaths;
        self.deaths_by_age_total += age_deaths;
        observer.end(TickPhase::Deaths);

        // ── Etapa 11: encontrar a vencedora de cada alimento ─────────────
        // Para cada comida, só a célula mais próxima que a contém vence.
        // A grade espacial evita testar cada partícula contra toda a população.
        // Como a interação tem alcance CELL_RADIUS, bastam os nove quadrados
        // vizinhos à partícula.
        observer.begin(TickPhase::Feeding);
        let organism_grid = self.organism_grid();
        // A regra de contato é deliberadamente pelo centro da comida, não pela
        // sobreposição de dois raios: distância(centro comida, centro célula) ≤ raio célula.
        let winners: Vec<Option<usize>> = self
            .food
            .iter()
            .map(|particle| self.closest_organism_containing(particle.position, &organism_grid))
            .collect();

        for index in winners.iter().flatten() {
            let organism = &mut self.organisms[*index];
            organism.energy = (organism.energy + food_energy).min(reproduction_limit);
        }
        self.clamp_organisms_to_physical_bounds();
        observer.end(TickPhase::Feeding);

        // ── Etapa 12: remover alimento consumido ou vencido ──────────────
        observer.begin(TickPhase::FoodCleanup);
        self.food = self
            .food
            .drain(..)
            .zip(winners)
            .filter_map(|(particle, winner)| {
                (winner.is_none() && particle.age_ticks < lifetime_ticks).then_some(particle)
            })
            .collect();
        observer.end(TickPhase::FoodCleanup);

        // ── Etapa 13: dividir células aptas e criar descendentes ─────────
        observer.begin(TickPhase::Reproduction);
        self.reproduce_ready_organisms(reproduction_minimum, reproduction_limit);
        observer.end(TickPhase::Reproduction);
    }

    /// Atualiza a configuração; não executa uma etapa biológica por si só.
    /// A regra é aplicada durante a etapa 8 de `tick()`, por
    /// `apply_anti_cramming`. A interface chama este método quando a pessoa
    /// altera força, distância ou estado do controle anti-cramming.
    pub fn set_anti_cramming(&mut self, config: AntiCrammingConfig) {
        self.anti_cramming = AntiCrammingConfig {
            enabled: config.enabled,
            force: config.force.clamp(0.0, 1.0),
            exclusion_distance: config
                .exclusion_distance
                .clamp(MIN_EXCLUSION_DISTANCE, MAX_EXCLUSION_DISTANCE),
        };
    }

    /// Fotografia binária completa do mundo para uma run continuar sem perder
    /// genomas, estados temporais dos cérebros ou sequência aleatória.
    pub fn write_checkpoint(&self, writer: &mut impl Write) -> io::Result<()> {
        write_u64(writer, self.tick)?;
        write_u64(writer, self.births_total)?;
        write_u64(writer, self.deaths_total)?;
        write_u64(writer, self.births_by_neural_decision_total)?;
        write_u64(writer, self.births_by_energy_limit_total)?;
        write_u64(writer, self.deaths_by_starvation_total)?;
        write_u64(writer, self.deaths_by_phagocytosis_total)?;
        write_u64(writer, self.deaths_by_age_total)?;
        write_f32(writer, self.energy_capacity)?;
        write_f32(writer, self.desert_spawn_credit)?;
        write_f32(writer, self.fertile_spawn_credit)?;
        self.map.write_to(writer)?;
        write_u64(writer, self.next_organism_id)?;
        write_u64(writer, self.random_state)?;
        write_bool(writer, self.anti_cramming.enabled)?;
        write_f32(writer, self.anti_cramming.force)?;
        write_f32(writer, self.anti_cramming.exclusion_distance)?;

        write_u32(writer, self.food.len() as u32)?;
        for food in &self.food {
            write_f32(writer, food.position.x)?;
            write_f32(writer, food.position.y)?;
            write_u32(writer, food.age_ticks)?;
        }
        write_u32(writer, self.organisms.len() as u32)?;
        for organism in &self.organisms {
            write_u64(writer, organism.id)?;
            write_f32(writer, organism.position.x)?;
            write_f32(writer, organism.position.y)?;
            write_f32(writer, organism.energy)?;
            write_u64(writer, organism.age_ticks)?;
            write_u16(writer, organism.longevity_gene.allele())?;
            write_u32(writer, organism.generation)?;
            write_u32(writer, organism.children_produced)?;
            write_bool(writer, organism.crowding_violation)?;
            write_f32(writer, organism.external_force.x)?;
            write_f32(writer, organism.external_force.y)?;
            for motor in [
                organism.motors.left,
                organism.motors.right,
                organism.motors.up,
                organism.motors.down,
                organism.motors.reproduce,
                organism.motors.phagocytose,
                organism.motors.emit_pg,
                organism.motors.emit_p1,
                organism.motors.emit_p2,
                organism.motors.emit_p3,
                organism.motors.donate,
            ] {
                write_f32(writer, motor)?;
            }
            for sensor in [
                organism.sensors.food_left,
                organism.sensors.food_right,
                organism.sensors.food_up,
                organism.sensors.food_down,
                organism.sensors.current_x,
                organism.sensors.current_y,
                organism.sensors.energy,
                organism.sensors.constant,
                organism.sensors.cell_contact,
                organism.sensors.pg_intensity,
                organism.sensors.pg_gradient_x,
                organism.sensors.pg_gradient_y,
                organism.sensors.p1_intensity,
                organism.sensors.p1_gradient_x,
                organism.sensors.p1_gradient_y,
                organism.sensors.p2_intensity,
                organism.sensors.p2_gradient_x,
                organism.sensors.p2_gradient_y,
                organism.sensors.p3_intensity,
                organism.sensors.p3_gradient_x,
                organism.sensors.p3_gradient_y,
                organism.sensors.force_x,
                organism.sensors.force_y,
            ] {
                write_f32(writer, sensor)?;
            }
            writer.write_all(organism.genome.bytes())?;
            for value in organism.brain.internal_state() {
                write_f32(writer, *value)?;
            }
        }
        Ok(())
    }

    pub fn read_checkpoint(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::Current)
    }

    /// Restaura runs 6–9: elas tinham mapa e força física, mas ainda usavam
    /// 12 sensores/6 motores. A topologia genética é deslocada para a rede
    /// química expandida durante a leitura.
    pub fn read_checkpoint_pre_pheromone(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::PrePheromone)
    }

    /// Restaura a versão química anterior ao motor Doar.
    pub fn read_checkpoint_pre_donation(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::PreDonation)
    }

    /// Restaura a versão imediatamente anterior à ampliação 2×2 do mundo.
    pub fn read_checkpoint_pre_world_expansion(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::PreWorldExpansion)
    }

    /// Restaura a versão de mundo ampliado anterior ao gene de longevidade.
    pub fn read_checkpoint_pre_longevity(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::PreLongevity)
    }

    /// Restaura a V13, anterior à telemetria separada de eventos.
    pub fn read_checkpoint_pre_event_counters(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::PreEventCounters)
    }

    /// Restaura a V14, anterior ao contador de morte por idade.
    pub fn read_checkpoint_pre_age_counters(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::PreAgeCounters)
    }

    /// Restaura a V15, cujo genoma ainda tinha 1.216 bits e alcance fixo.
    pub fn read_checkpoint_pre_food_sensor_range(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::PreFoodSensorRange)
    }

    /// Restaura a V16, cujo genoma já tinha alcance hereditário, mas ainda
    /// não carregava os bits de atividade de neurônios.
    pub fn read_checkpoint_pre_neural_node_activity(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::PreNeuralNodeActivity)
    }

    /// Restaura sessões gravadas antes de fagocitose e sensores de força.
    pub fn read_checkpoint_v2(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::V2)
    }

    pub fn read_checkpoint_v3(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::V3)
    }

    pub fn read_checkpoint_v4(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::V4)
    }

    /// Restaura sessões salvas antes dos sensores de contato e migra seus
    /// genomas de 5 para 6 bits de identificador por conexão.
    pub fn read_checkpoint_v1(reader: &mut impl Read) -> io::Result<Self> {
        Self::read_checkpoint_with_format(reader, CheckpointFormat::V1)
    }

    fn read_checkpoint_with_format(
        reader: &mut impl Read,
        checkpoint_format: CheckpointFormat,
    ) -> io::Result<Self> {
        let tick = read_u64(reader)?;
        let births_total = read_u64(reader)?;
        let deaths_total = read_u64(reader)?;
        let (
            births_by_neural_decision_total,
            births_by_energy_limit_total,
            deaths_by_starvation_total,
            deaths_by_phagocytosis_total,
            deaths_by_age_total,
        ) = match checkpoint_format {
            CheckpointFormat::PreFoodSensorRange
            | CheckpointFormat::PreNeuralNodeActivity
            | CheckpointFormat::Current => (
                read_u64(reader)?,
                read_u64(reader)?,
                read_u64(reader)?,
                read_u64(reader)?,
                read_u64(reader)?,
            ),
            CheckpointFormat::PreAgeCounters => (
                read_u64(reader)?,
                read_u64(reader)?,
                read_u64(reader)?,
                read_u64(reader)?,
                0,
            ),
            _ => (0, 0, 0, 0, 0),
        };
        let energy_capacity = read_f32(reader)?;
        let desert_spawn_credit = read_f32(reader)?;
        let fertile_spawn_credit = if matches!(
            checkpoint_format,
            CheckpointFormat::V4
                | CheckpointFormat::PrePheromone
                | CheckpointFormat::PreDonation
                | CheckpointFormat::PreWorldExpansion
                | CheckpointFormat::PreLongevity
                | CheckpointFormat::PreEventCounters
                | CheckpointFormat::PreAgeCounters
                | CheckpointFormat::PreFoodSensorRange
                | CheckpointFormat::PreNeuralNodeActivity
                | CheckpointFormat::Current
        ) {
            read_f32(reader)?
        } else {
            0.0
        };
        let map = if matches!(
            checkpoint_format,
            CheckpointFormat::PreLongevity
                | CheckpointFormat::PreEventCounters
                | CheckpointFormat::PreAgeCounters
                | CheckpointFormat::PreFoodSensorRange
                | CheckpointFormat::PreNeuralNodeActivity
                | CheckpointFormat::Current
        ) {
            MapDefinition::read_from(reader)?
        } else if matches!(
            checkpoint_format,
            CheckpointFormat::PrePheromone
                | CheckpointFormat::PreDonation
                | CheckpointFormat::PreWorldExpansion
        ) {
            MapDefinition::read_legacy_from(reader)?
        } else {
            MapDefinition::default()
        };
        let next_organism_id = read_u64(reader)?;
        let random_state = read_u64(reader)?;
        let anti_cramming = AntiCrammingConfig {
            enabled: read_bool(reader)?,
            force: read_f32(reader)?,
            exclusion_distance: read_f32(reader)?,
        };
        let food_count = read_count(reader, 2_000_000)?;
        let mut food = Vec::with_capacity(food_count);
        for _ in 0..food_count {
            food.push(Phytoplankton {
                position: vec2(read_f32(reader)?, read_f32(reader)?)
                    * if matches!(
                        checkpoint_format,
                        CheckpointFormat::PreLongevity
                            | CheckpointFormat::PreEventCounters
                            | CheckpointFormat::PreAgeCounters
                            | CheckpointFormat::PreFoodSensorRange
                            | CheckpointFormat::PreNeuralNodeActivity
                            | CheckpointFormat::Current
                    ) {
                        1.0
                    } else {
                        2.0
                    },
                age_ticks: read_u32(reader)?,
            });
        }
        let organism_count = read_count(reader, 1_000_000)?;
        let mut organisms = Vec::with_capacity(organism_count);
        for _ in 0..organism_count {
            let id = read_u64(reader)?;
            let position = vec2(read_f32(reader)?, read_f32(reader)?)
                * if matches!(
                    checkpoint_format,
                    CheckpointFormat::PreLongevity
                        | CheckpointFormat::PreEventCounters
                        | CheckpointFormat::PreAgeCounters
                        | CheckpointFormat::PreFoodSensorRange
                        | CheckpointFormat::PreNeuralNodeActivity
                        | CheckpointFormat::Current
                ) {
                    1.0
                } else {
                    2.0
                };
            let energy = read_f32(reader)?;
            let age_ticks = read_u64(reader)?;
            let longevity_gene = if matches!(
                checkpoint_format,
                CheckpointFormat::PreEventCounters
                    | CheckpointFormat::PreAgeCounters
                    | CheckpointFormat::PreFoodSensorRange
                    | CheckpointFormat::PreNeuralNodeActivity
                    | CheckpointFormat::Current
            ) {
                LongevityGene::from_allele(read_u16(reader)?)
            } else {
                LongevityGene::from_legacy_age(age_ticks)
            };
            let generation = read_u32(reader)?;
            let children_produced = read_u32(reader)?;
            let crowding_violation = read_bool(reader)?;
            let external_force = if matches!(
                checkpoint_format,
                CheckpointFormat::V3
                    | CheckpointFormat::PrePheromone
                    | CheckpointFormat::PreDonation
                    | CheckpointFormat::PreWorldExpansion
                    | CheckpointFormat::PreLongevity
                    | CheckpointFormat::PreEventCounters
                    | CheckpointFormat::PreAgeCounters
                    | CheckpointFormat::PreFoodSensorRange
                    | CheckpointFormat::PreNeuralNodeActivity
                    | CheckpointFormat::Current
            ) {
                vec2(read_f32(reader)?, read_f32(reader)?)
            } else {
                Vec2::ZERO
            };
            let motors = MotorOutput {
                left: read_f32(reader)?,
                right: read_f32(reader)?,
                up: read_f32(reader)?,
                down: read_f32(reader)?,
                reproduce: read_f32(reader)?,
                phagocytose: if matches!(
                    checkpoint_format,
                    CheckpointFormat::V3
                        | CheckpointFormat::PrePheromone
                        | CheckpointFormat::PreDonation
                        | CheckpointFormat::PreWorldExpansion
                        | CheckpointFormat::PreLongevity
                        | CheckpointFormat::PreEventCounters
                        | CheckpointFormat::PreAgeCounters
                        | CheckpointFormat::PreFoodSensorRange
                        | CheckpointFormat::PreNeuralNodeActivity
                        | CheckpointFormat::Current
                ) {
                    read_f32(reader)?
                } else {
                    0.0
                },
                emit_pg: if matches!(
                    checkpoint_format,
                    CheckpointFormat::PreDonation
                        | CheckpointFormat::PreWorldExpansion
                        | CheckpointFormat::PreLongevity
                        | CheckpointFormat::PreEventCounters
                        | CheckpointFormat::PreAgeCounters
                        | CheckpointFormat::PreFoodSensorRange
                        | CheckpointFormat::PreNeuralNodeActivity
                        | CheckpointFormat::Current
                ) {
                    read_f32(reader)?
                } else {
                    0.0
                },
                emit_p1: if matches!(
                    checkpoint_format,
                    CheckpointFormat::PreDonation
                        | CheckpointFormat::PreWorldExpansion
                        | CheckpointFormat::PreLongevity
                        | CheckpointFormat::PreEventCounters
                        | CheckpointFormat::PreAgeCounters
                        | CheckpointFormat::PreFoodSensorRange
                        | CheckpointFormat::PreNeuralNodeActivity
                        | CheckpointFormat::Current
                ) {
                    read_f32(reader)?
                } else {
                    0.0
                },
                emit_p2: if matches!(
                    checkpoint_format,
                    CheckpointFormat::PreDonation
                        | CheckpointFormat::PreWorldExpansion
                        | CheckpointFormat::PreLongevity
                        | CheckpointFormat::PreEventCounters
                        | CheckpointFormat::PreAgeCounters
                        | CheckpointFormat::PreFoodSensorRange
                        | CheckpointFormat::PreNeuralNodeActivity
                        | CheckpointFormat::Current
                ) {
                    read_f32(reader)?
                } else {
                    0.0
                },
                emit_p3: if matches!(
                    checkpoint_format,
                    CheckpointFormat::PreDonation
                        | CheckpointFormat::PreWorldExpansion
                        | CheckpointFormat::PreLongevity
                        | CheckpointFormat::PreEventCounters
                        | CheckpointFormat::PreAgeCounters
                        | CheckpointFormat::PreFoodSensorRange
                        | CheckpointFormat::PreNeuralNodeActivity
                        | CheckpointFormat::Current
                ) {
                    read_f32(reader)?
                } else {
                    0.0
                },
                donate: if matches!(
                    checkpoint_format,
                    CheckpointFormat::PreLongevity
                        | CheckpointFormat::PreEventCounters
                        | CheckpointFormat::PreAgeCounters
                        | CheckpointFormat::PreFoodSensorRange
                        | CheckpointFormat::PreNeuralNodeActivity
                        | CheckpointFormat::Current
                ) {
                    read_f32(reader)?
                } else {
                    0.0
                },
            };
            let sensors = read_sensors(reader, checkpoint_format)?;
            let genome = if checkpoint_format == CheckpointFormat::V1 {
                let mut bits = [0u8; crate::simulation::genome::LEGACY_V1_GENOME_BYTES];
                reader.read_exact(&mut bits)?;
                Genome::from_legacy_v1_bytes(bits)
            } else if checkpoint_format == CheckpointFormat::Current {
                let mut bits = [0u8; crate::simulation::genome::GENOME_BYTES];
                reader.read_exact(&mut bits)?;
                Genome::from_bytes(bits)
            } else if checkpoint_format == CheckpointFormat::PreNeuralNodeActivity {
                let mut bits = [0u8; PRE_NEURAL_NODE_ACTIVITY_GENOME_BYTES];
                reader.read_exact(&mut bits)?;
                Genome::from_pre_neural_node_activity_bytes(bits)
            } else {
                let mut bits = [0u8; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES];
                reader.read_exact(&mut bits)?;
                if checkpoint_format == CheckpointFormat::V2 {
                    Genome::from_legacy_v2_bytes(bits)
                } else if checkpoint_format == CheckpointFormat::PrePheromone {
                    Genome::from_pre_pheromone_bytes(bits)
                } else if matches!(
                    checkpoint_format,
                    CheckpointFormat::PreDonation | CheckpointFormat::PreWorldExpansion
                ) {
                    Genome::from_pre_donation_bytes(bits)
                } else if checkpoint_format == CheckpointFormat::PreFoodSensorRange {
                    Genome::from_pre_food_sensor_range_bytes(bits)
                } else {
                    Genome::from_pre_food_sensor_range_bytes(bits)
                }
            };
            let mut internal_state = [0.0; crate::simulation::brain::INTERNAL_COUNT];
            for value in &mut internal_state {
                *value = read_f32(reader)?;
            }
            let mut brain = Brain::default();
            brain.configure_network(genome.decode_connections(), genome.neural_node_activity());
            brain.restore_internal_state(internal_state);
            organisms.push(Organism {
                id,
                position,
                energy,
                age_ticks,
                longevity_gene,
                generation,
                children_produced,
                crowding_violation,
                external_force,
                motors,
                sensors,
                brain,
                genome,
            });
        }
        Ok(Self {
            tick,
            births_total,
            deaths_total,
            births_by_neural_decision_total,
            births_by_energy_limit_total,
            deaths_by_starvation_total,
            deaths_by_phagocytosis_total,
            deaths_by_age_total,
            energy_capacity,
            food,
            organisms,
            map,
            pheromones: PheromoneField::default(),
            desert_spawn_credit,
            fertile_spawn_credit,
            next_organism_id,
            random_state,
            anti_cramming,
            crowding: CrowdingMetrics::default(),
        })
    }

    pub fn average_energy(&self) -> f32 {
        if self.organisms.is_empty() {
            0.0
        } else {
            self.organisms
                .iter()
                .map(|organism| organism.energy)
                .sum::<f32>()
                / self.organisms.len() as f32
        }
    }

    pub fn energy_distribution(&self, bucket_count: usize) -> (Vec<usize>, f32) {
        let bucket_count = bucket_count.max(1);
        let bucket_width = self.energy_capacity / bucket_count as f32;
        let mut counts = vec![0; bucket_count];
        for organism in &self.organisms {
            let index = (organism.energy / bucket_width).floor() as usize;
            counts[index.min(bucket_count - 1)] += 1;
        }
        (counts, bucket_width)
    }

    #[allow(dead_code)] // usado pelo executável independente do laboratório
    pub fn average_connection_count(&self) -> f32 {
        if self.organisms.is_empty() {
            0.0
        } else {
            self.organisms
                .iter()
                .map(|organism| organism.brain.connection_count() as f32)
                .sum::<f32>()
                / self.organisms.len() as f32
        }
    }

    pub fn average_age_ticks(&self) -> f32 {
        if self.organisms.is_empty() {
            0.0
        } else {
            self.organisms
                .iter()
                .map(|organism| organism.age_ticks as f32)
                .sum::<f32>()
                / self.organisms.len() as f32
        }
    }

    pub fn average_generation(&self) -> f32 {
        if self.organisms.is_empty() {
            0.0
        } else {
            self.organisms
                .iter()
                .map(|organism| organism.generation as f32)
                .sum::<f32>()
                / self.organisms.len() as f32
        }
    }

    pub fn most_similar_living_cell(&self, organism_id: u64) -> Option<(u64, f32)> {
        let observed = self
            .organisms
            .iter()
            .find(|organism| organism.id == organism_id)?;
        self.organisms
            .iter()
            .filter(|organism| organism.id != organism_id)
            .map(|organism| (organism.id, observed.genome.similarity_to(&organism.genome)))
            .max_by(|left, right| left.1.total_cmp(&right.1))
    }

    fn update_sensors(&mut self, flow: &FlowField, reproduction_limit: f32) {
        // A grade espacial limita cada consulta aos alimentos próximos. Depois
        // de encontrados, cada alimento é classificado em um único cone de
        // 90°: esquerda, direita, cima ou baixo.
        let mut food_grid: HashMap<(i32, i32), Vec<Vec2>> = HashMap::new();
        for particle in &self.food {
            food_grid
                .entry(sensor_grid_cell(particle.position))
                .or_default()
                .push(particle.position);
        }
        // A mesma grade que acelera o anti-cramming encontra contatos físicos
        // sem comparar toda a população. O retrato é calculado antes de
        // alterar qualquer SensorInput, para todos verem o mesmo tick.
        let organism_grid = self.organism_grid();
        let contacts: Vec<bool> = (0..self.organisms.len())
            .map(|index| self.has_contact(index, &organism_grid))
            .collect();
        for (index, organism) in self.organisms.iter_mut().enumerate() {
            let current = flow.sample_with_map(organism.position, Some(&self.map)) / MAX_FLOW_SPEED;
            let food_sensor_range = organism.food_sensor_range();
            let nearest =
                nearest_food_in_cones(&food_grid, organism.position, food_sensor_range, &self.map);
            let chemical = self.pheromones.sensors_for(organism);
            organism.sensors = SensorInput {
                food_left: cone_signal(nearest[0], food_sensor_range),
                food_right: cone_signal(nearest[1], food_sensor_range),
                food_up: cone_signal(nearest[2], food_sensor_range),
                food_down: cone_signal(nearest[3], food_sensor_range),
                current_x: current.x,
                current_y: current.y,
                // A célula percebe a fração da energia que falta para a
                // divisão obrigatória, e não uma capacidade abstrata.
                energy: (organism.energy / reproduction_limit.max(1.0)).clamp(0.0, 1.0),
                constant: 1.0,
                cell_contact: contacts[index] as u8 as f32,
                pg_intensity: chemical[0].intensity,
                pg_gradient_x: chemical[0].gradient_x,
                pg_gradient_y: chemical[0].gradient_y,
                p1_intensity: chemical[1].intensity,
                p1_gradient_x: chemical[1].gradient_x,
                p1_gradient_y: chemical[1].gradient_y,
                p2_intensity: chemical[2].intensity,
                p2_gradient_x: chemical[2].gradient_x,
                p2_gradient_y: chemical[2].gradient_y,
                p3_intensity: chemical[3].intensity,
                p3_gradient_x: chemical[3].gradient_x,
                p3_gradient_y: chemical[3].gradient_y,
                force_x: (organism.external_force.x / FORCE_SENSOR_DISPLACEMENT_SCALE)
                    .clamp(-1.0, 1.0),
                force_y: (organism.external_force.y / FORCE_SENSOR_DISPLACEMENT_SCALE)
                    .clamp(-1.0, 1.0),
            };
        }
    }

    fn organism_grid(&self) -> HashMap<(i32, i32), Vec<usize>> {
        let cell_size = self.contact_grid_cell_size();
        let mut grid = HashMap::with_capacity(self.organisms.len() * 2);
        for (index, organism) in self.organisms.iter().enumerate() {
            grid.entry(grid_cell_with_size(organism.position, cell_size))
                .or_insert_with(Vec::new)
                .push(index);
        }
        grid
    }

    /// Similaridade da célula em contato com núcleo mais próximo. Empate por
    /// ID torna o sinal estável entre execuções com a mesma seed.
    fn has_contact(&self, index: usize, grid: &HashMap<(i32, i32), Vec<usize>>) -> bool {
        let organism = &self.organisms[index];
        let origin = grid_cell_with_size(organism.position, self.contact_grid_cell_size());
        (-1..=1)
            .flat_map(|offset_y| {
                (-1..=1).filter_map(move |offset_x| {
                    grid.get(&(origin.0 + offset_x, origin.1 + offset_y))
                })
            })
            .flatten()
            .filter(|&&neighbor_index| neighbor_index != index)
            .any(|&neighbor_index| {
                let neighbor = &self.organisms[neighbor_index];
                let distance_squared = organism.position.distance_squared(neighbor.position);
                let contact_distance = organism.physical_radius() + neighbor.physical_radius();
                distance_squared <= contact_distance * contact_distance
            })
    }

    /// Executa a fase física da fagocitose. O motor só gera sucção quando o
    /// atacante possui ao menos 10% de vantagem de raio. Uma tentativa contra
    /// alvo maior ainda custa energia, mas não puxa nem causa dano.
    fn apply_phagocytosis(&mut self) -> PhagocytosisOutcome {
        let cell_size = self.contact_grid_cell_size() * (1.0 + PHAGOCYTOSIS_INTERACTION_MARGIN);
        let grid = self.organism_grid_for_distance(cell_size);
        let attempts: Vec<(usize, usize, f32)> = self
            .organisms
            .iter()
            .enumerate()
            .filter_map(|(index, organism)| {
                (organism.motors.phagocytose > 0.0)
                    .then(|| self.closest_phagocytosis_target(index, &grid))
                    .flatten()
                    .map(|target| (index, target, organism.motors.phagocytose))
            })
            .collect();

        let mut outcome = PhagocytosisOutcome::default();
        for (predator_index, prey_index, activation) in attempts {
            // Há contato/aproximação real: ativar fagocitose sempre cobra um
            // pouco. Isso permite que a seleção elimine ataques inúteis.
            let cost_multiplier = self.organisms[predator_index].energy_cost_multiplier()
                * self.organisms[predator_index].physical_area_scale();
            self.organisms[predator_index].energy -=
                MAX_PHAGOCYTOSIS_ENERGY_COST * activation * cost_multiplier;

            let predator_radius = self.organisms[predator_index].physical_radius();
            let prey_radius = self.organisms[prey_index].physical_radius();
            if predator_radius < prey_radius * PHAGOCYTOSIS_MIN_RADIUS_RATIO {
                continue;
            }

            let delta =
                self.organisms[prey_index].position - self.organisms[predator_index].position;
            let distance = delta.length();
            let direction = if distance > 0.001 {
                delta / distance
            } else {
                spread_direction(self.organisms[predator_index].id ^ self.organisms[prey_index].id)
            };
            let size_advantage = (predator_radius / prey_radius - 1.0).clamp(0.0, 1.0);
            // Como a emissão química e o alcance alimentar, a atração ganha
            // escala volumétrica (r³). O custo segue o expoente corporal
            // sublinear, portanto uma célula grande captura com mais
            // eficiência por energia investida.
            let size_capture_scale = (predator_radius / CELL_RADIUS).powi(3);
            let pull =
                MAX_PHAGOCYTOSIS_PULL_PER_TICK * activation * size_advantage * size_capture_scale;
            let predator_force = direction * pull * 0.20;
            let prey_force = -direction * pull * 0.80;
            {
                let (predator, prey) = if predator_index < prey_index {
                    two_mut(&mut self.organisms, predator_index, prey_index)
                } else {
                    let (prey, predator) = two_mut(&mut self.organisms, prey_index, predator_index);
                    (predator, prey)
                };
                predator.position += predator_force;
                predator.external_force += predator_force;
                prey.position += prey_force;
                prey.external_force += prey_force;
            }

            outcome
                .protected_pairs
                .insert(pair_key(predator_index, prey_index));
            if distance <= predator_radius - prey_radius {
                outcome
                    .capture_candidates
                    .push((predator_index, prey_index));
            }
        }
        self.clamp_organisms_to_physical_bounds();
        outcome
    }

    fn closest_phagocytosis_target(
        &self,
        index: usize,
        grid: &HashMap<(i32, i32), Vec<usize>>,
    ) -> Option<usize> {
        let organism = &self.organisms[index];
        let cell_size = self.contact_grid_cell_size() * (1.0 + PHAGOCYTOSIS_INTERACTION_MARGIN);
        let origin = grid_cell_with_size(organism.position, cell_size);
        (-1..=1)
            .flat_map(|offset_y| {
                (-1..=1).filter_map(move |offset_x| {
                    grid.get(&(origin.0 + offset_x, origin.1 + offset_y))
                })
            })
            .flatten()
            .filter(|&&neighbor_index| neighbor_index != index)
            .filter_map(|&neighbor_index| {
                let neighbor = &self.organisms[neighbor_index];
                let distance_squared = organism.position.distance_squared(neighbor.position);
                let interaction_distance = organism.physical_radius()
                    + neighbor.physical_radius()
                    + organism.physical_radius().min(neighbor.physical_radius())
                        * PHAGOCYTOSIS_INTERACTION_MARGIN;
                (distance_squared <= interaction_distance * interaction_distance).then_some((
                    neighbor_index,
                    distance_squared,
                    neighbor.id,
                ))
            })
            .min_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.2.cmp(&right.2))
            })
            .map(|target| target.0)
    }

    fn consume_phagocytosed(
        &mut self,
        capture_candidates: &[(usize, usize)],
        reproduction_limit: f32,
    ) {
        let mut winner_by_prey: HashMap<usize, usize> = HashMap::new();
        for &(predator_index, prey_index) in capture_candidates {
            let Some(predator) = self.organisms.get(predator_index) else {
                continue;
            };
            let Some(prey) = self.organisms.get(prey_index) else {
                continue;
            };
            let predator_radius = predator.physical_radius();
            let prey_radius = prey.physical_radius();
            if predator_radius < prey_radius * PHAGOCYTOSIS_MIN_RADIUS_RATIO
                || predator.position.distance(prey.position) > predator_radius - prey_radius
            {
                continue;
            }
            winner_by_prey
                .entry(prey_index)
                .and_modify(|known_predator| {
                    let known = &self.organisms[*known_predator];
                    let candidate_ratio = predator_radius / prey_radius;
                    let known_ratio = known.physical_radius() / prey_radius;
                    if candidate_ratio > known_ratio
                        || (candidate_ratio == known_ratio && predator.id < known.id)
                    {
                        *known_predator = predator_index;
                    }
                })
                .or_insert(predator_index);
        }

        if winner_by_prey.is_empty() {
            return;
        }
        let mut energy_gains = vec![0.0; self.organisms.len()];
        let mut consumed = vec![false; self.organisms.len()];
        for (prey_index, predator_index) in winner_by_prey {
            if prey_index != predator_index {
                energy_gains[predator_index] += self.organisms[prey_index].energy.max(0.0);
                consumed[prey_index] = true;
            }
        }
        for (index, gain) in energy_gains.into_iter().enumerate() {
            if gain > 0.0 && !consumed[index] {
                self.organisms[index].energy =
                    (self.organisms[index].energy + gain).min(reproduction_limit);
            }
        }
        let population_before = self.organisms.len();
        self.organisms = self
            .organisms
            .drain(..)
            .enumerate()
            .filter_map(|(index, organism)| (!consumed[index]).then_some(organism))
            .collect();
        self.deaths_total += (population_before - self.organisms.len()) as u64;
        self.deaths_by_phagocytosis_total += (population_before - self.organisms.len()) as u64;
    }

    /// Troca conservativa de energia entre corpos em contato. A escolha do
    /// alvo é geométrica e neutra; parentesco só influencia a decisão pela
    /// rede, via Ct + PG e pelo motor Doar.
    fn apply_donations(&mut self, energy_capacity: f32) {
        let grid = self.organism_grid();
        let attempts: Vec<(usize, usize, f32)> = self
            .organisms
            .iter()
            .enumerate()
            .filter_map(|(donor, organism)| {
                (organism.motors.donate > 0.0)
                    .then(|| self.closest_contact_target(donor, &grid))
                    .flatten()
                    .map(|target| (donor, target, organism.motors.donate))
            })
            .collect();

        for (donor_index, recipient_index, activation) in attempts {
            if donor_index == recipient_index
                || donor_index >= self.organisms.len()
                || recipient_index >= self.organisms.len()
            {
                continue;
            }
            let (donor, recipient) = if donor_index < recipient_index {
                let (donor, recipient) = two_mut(&mut self.organisms, donor_index, recipient_index);
                (donor, recipient)
            } else {
                let (recipient, donor) = two_mut(&mut self.organisms, recipient_index, donor_index);
                (donor, recipient)
            };
            // A taxa operacional também sai do doador. Logo, a energia
            // transferível precisa caber junto com esse custo acima da reserva.
            let operational_cost = DONATION_OPERATIONAL_COST * donor.energy_cost_multiplier();
            let available =
                (donor.energy - DONATION_RESERVE_ENERGY).max(0.0) / (1.0 + operational_cost);
            let room = (energy_capacity - recipient.energy).max(0.0);
            let transfer = (MAX_DONATION_PER_TICK * activation)
                .min(available)
                .min(room);
            if transfer <= 0.0 {
                continue;
            }
            donor.energy -= transfer + transfer * operational_cost;
            recipient.energy += transfer;
        }
    }

    fn closest_contact_target(
        &self,
        index: usize,
        grid: &HashMap<(i32, i32), Vec<usize>>,
    ) -> Option<usize> {
        let organism = &self.organisms[index];
        let origin = grid_cell_with_size(organism.position, self.contact_grid_cell_size());
        (-1..=1)
            .flat_map(|offset_y| {
                (-1..=1).filter_map(move |offset_x| {
                    grid.get(&(origin.0 + offset_x, origin.1 + offset_y))
                })
            })
            .flatten()
            .filter(|&&candidate| candidate != index)
            .filter_map(|&candidate| {
                let other = &self.organisms[candidate];
                let distance_squared = organism.position.distance_squared(other.position);
                let contact_radius = organism.physical_radius() + other.physical_radius();
                (distance_squared <= contact_radius * contact_radius).then_some((
                    distance_squared,
                    other.id,
                    candidate,
                ))
            })
            .min_by(|left, right| {
                left.0
                    .total_cmp(&right.0)
                    .then_with(|| left.1.cmp(&right.1))
            })
            .map(|candidate| candidate.2)
    }

    /// Resolve sobreposições usando uma grade uniforme: só pares nos nove
    /// blocos locais são avaliados, em vez de comparar toda a população.
    fn apply_anti_cramming(&mut self, phagocytosis_pairs: &HashSet<(usize, usize)>) {
        for organism in &mut self.organisms {
            organism.crowding_violation = false;
        }
        self.crowding = CrowdingMetrics::default();
        if !self.anti_cramming.enabled {
            return;
        }
        if self.anti_cramming.force <= 0.0 {
            self.measure_crowding(phagocytosis_pairs);
            return;
        }

        let passes = (1.0 + self.anti_cramming.force * 5.0).round() as usize;
        for _ in 0..passes.clamp(1, MAX_CRAMMING_RELAXATION_PASSES) {
            let grid = self.organism_grid_for_distance(self.max_pair_exclusion_distance());
            self.resolve_cramming_pairs(&grid, phagocytosis_pairs);
        }
        self.measure_crowding(phagocytosis_pairs);
    }

    fn organism_grid_for_distance(&self, cell_size: f32) -> HashMap<(i32, i32), Vec<usize>> {
        let mut grid = HashMap::with_capacity(self.organisms.len() * 2);
        for (index, organism) in self.organisms.iter().enumerate() {
            grid.entry(grid_cell_with_size(organism.position, cell_size))
                .or_insert_with(Vec::new)
                .push(index);
        }
        grid
    }

    fn resolve_cramming_pairs(
        &mut self,
        grid: &HashMap<(i32, i32), Vec<usize>>,
        phagocytosis_pairs: &HashSet<(usize, usize)>,
    ) {
        let grid_size = self.max_pair_exclusion_distance();
        for index in 0..self.organisms.len() {
            let origin_cell = grid_cell_with_size(self.organisms[index].position, grid_size);
            for offset_y in -1..=1 {
                for offset_x in -1..=1 {
                    let cell = (origin_cell.0 + offset_x, origin_cell.1 + offset_y);
                    let Some(neighbors) = grid.get(&cell) else {
                        continue;
                    };
                    for &neighbor_index in neighbors {
                        if neighbor_index <= index {
                            continue;
                        }
                        if phagocytosis_pairs.contains(&(index, neighbor_index)) {
                            continue;
                        }
                        let exclusion_distance =
                            self.pair_exclusion_distance(index, neighbor_index);
                        let (left, right) = two_mut(&mut self.organisms, index, neighbor_index);
                        let (left_force, right_force) = resolve_pair_separation(
                            left,
                            right,
                            exclusion_distance,
                            self.anti_cramming.force,
                        );
                        left.external_force += left_force;
                        right.external_force += right_force;
                    }
                }
            }
        }
    }

    fn measure_crowding(&mut self, phagocytosis_pairs: &HashSet<(usize, usize)>) {
        let grid_size = self.max_pair_exclusion_distance();
        let grid = self.organism_grid_for_distance(grid_size);
        let mut violating_pairs = 0;
        let mut minimum_distance = f32::INFINITY;
        for index in 0..self.organisms.len() {
            let origin_cell = grid_cell_with_size(self.organisms[index].position, grid_size);
            for offset_y in -1..=1 {
                for offset_x in -1..=1 {
                    let cell = (origin_cell.0 + offset_x, origin_cell.1 + offset_y);
                    let Some(neighbors) = grid.get(&cell) else {
                        continue;
                    };
                    for &neighbor_index in neighbors {
                        if neighbor_index <= index {
                            continue;
                        }
                        if phagocytosis_pairs.contains(&(index, neighbor_index)) {
                            continue;
                        }
                        let distance = self.organisms[index]
                            .position
                            .distance(self.organisms[neighbor_index].position);
                        let exclusion_distance =
                            self.pair_exclusion_distance(index, neighbor_index);
                        if distance + 0.001 < exclusion_distance {
                            violating_pairs += 1;
                            minimum_distance = minimum_distance.min(distance);
                            self.organisms[index].crowding_violation = true;
                            self.organisms[neighbor_index].crowding_violation = true;
                        }
                    }
                }
            }
        }
        self.crowding = CrowdingMetrics {
            violating_pairs,
            minimum_violation_distance: (violating_pairs > 0).then_some(minimum_distance),
        };
    }

    /// O valor configurado na UI preserva a noção anterior de "espaço":
    /// 1R significa tocar os raios físicos; 3R significa deixar três vezes
    /// esse afastamento entre os dois núcleos.
    fn anti_cramming_spacing_multiplier(&self) -> f32 {
        (self.anti_cramming.exclusion_distance / MIN_EXCLUSION_DISTANCE)
            .clamp(1.0, MAX_EXCLUSION_DISTANCE / MIN_EXCLUSION_DISTANCE)
    }

    fn pair_exclusion_distance(&self, left: usize, right: usize) -> f32 {
        (self.organisms[left].physical_radius() + self.organisms[right].physical_radius())
            * self.anti_cramming_spacing_multiplier()
    }

    fn max_pair_exclusion_distance(&self) -> f32 {
        self.contact_grid_cell_size() * self.anti_cramming_spacing_multiplier()
    }

    fn contact_grid_cell_size(&self) -> f32 {
        self.organisms
            .iter()
            .map(Organism::physical_radius)
            .fold(CELL_RADIUS, f32::max)
            * 2.0
    }

    fn closest_organism_containing(
        &self,
        food_position: Vec2,
        grid: &HashMap<(i32, i32), Vec<usize>>,
    ) -> Option<usize> {
        let cell = grid_cell_with_size(food_position, self.contact_grid_cell_size());
        (-1..=1)
            .flat_map(|offset_y| {
                (-1..=1)
                    .filter_map(move |offset_x| grid.get(&(cell.0 + offset_x, cell.1 + offset_y)))
            })
            .flatten()
            .filter_map(|index| {
                let organism = &self.organisms[*index];
                let distance_squared = organism.position.distance_squared(food_position);
                (distance_squared <= organism.physical_radius().powi(2)).then_some((
                    *index,
                    distance_squared,
                    organism.id,
                ))
            })
            .min_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.2.cmp(&right.2))
            })
            .map(|winner| winner.0)
    }

    fn reproduce_ready_organisms(&mut self, minimum: f32, limit: f32) {
        let parents: Vec<(Vec2, Genome, LongevityGene, u32, f32, bool)> = self
            .organisms
            .iter_mut()
            .filter_map(|organism| {
                let reached_energy_limit = organism.energy >= limit;
                let neural_decision = organism.energy >= minimum
                    && organism.motors.reproduce >= REPRODUCTION_ACTIVATION_THRESHOLD;
                if !reached_energy_limit && !neural_decision {
                    return None;
                }
                // O teto físico prevalece quando ambas as regras acontecem
                // no mesmo tick: assim a origem do nascimento fica inequívoca.
                let caused_by_energy_limit = reached_energy_limit;
                let child_energy = organism.energy * 0.5;
                organism.energy = child_energy;
                organism.children_produced += 1;
                Some((
                    organism.position,
                    organism.genome.clone(),
                    organism.longevity_gene,
                    organism.generation + 1,
                    child_energy,
                    caused_by_energy_limit,
                ))
            })
            .collect();

        let mut births = Vec::with_capacity(parents.len());
        let mut births_by_neural_decision = 0u64;
        let mut births_by_energy_limit = 0u64;
        for (
            parent_position,
            parent_genome,
            parent_longevity_gene,
            generation,
            child_energy,
            caused_by_energy_limit,
        ) in parents
        {
            if caused_by_energy_limit {
                births_by_energy_limit += 1;
            } else {
                births_by_neural_decision += 1;
            }
            let genome = Genome::mutated_from(&parent_genome, &mut self.random_state);
            let longevity_gene = LongevityGene::mutated_from(
                parent_longevity_gene,
                self.next_random_unit(),
                self.next_random_unit(),
            );
            let angle = self.next_random_unit() * std::f32::consts::TAU;
            let child_radius = physical_radius_for_energy(child_energy);
            let offset = vec2(angle.cos(), angle.sin()) * child_radius * 2.2;
            let position = vec2(
                clamp_coordinate_to_world(parent_position.x + offset.x, child_radius, WORLD_WIDTH),
                clamp_coordinate_to_world(parent_position.y + offset.y, child_radius, WORLD_HEIGHT),
            );
            births.push(self.make_organism(
                position,
                genome,
                longevity_gene,
                generation,
                child_energy,
            ));
        }
        self.births_total += births.len() as u64;
        self.births_by_neural_decision_total += births_by_neural_decision;
        self.births_by_energy_limit_total += births_by_energy_limit;
        self.organisms.extend(births);
    }

    fn clamp_organisms_to_physical_bounds(&mut self) {
        for organism in &mut self.organisms {
            let radius = organism.physical_radius();
            organism.position.x =
                clamp_coordinate_to_world(organism.position.x, radius, WORLD_WIDTH);
            organism.position.y =
                clamp_coordinate_to_world(organism.position.y, radius, WORLD_HEIGHT);
        }
    }

    fn spawn_food_in(&mut self, terrain: Terrain) -> bool {
        let map = self.map.clone();
        let Some(position) =
            map.random_point_in_terrain(|| self.next_random_unit(), terrain, FOOD_RADIUS)
        else {
            return false;
        };
        self.food.push(Phytoplankton {
            position,
            age_ticks: 0,
        });
        true
    }

    fn spawn_organism(
        &mut self,
        initial_neural_configuration: InitialNeuralConfiguration,
        inherited_genome: Option<Genome>,
    ) {
        let radius = physical_radius_for_energy(STARTING_ENERGY);
        let terrain = if self.map.has_fertile_area() && self.next_random_unit() < 0.5 {
            Terrain::Fertile
        } else {
            Terrain::Desert
        };
        let map = self.map.clone();
        let position = map
            .random_point_in_terrain(|| self.next_random_unit(), terrain, radius)
            .unwrap_or_else(|| map.random_free_point(|| self.next_random_unit(), radius));
        let genome = inherited_genome.unwrap_or_else(|| match initial_neural_configuration {
            InitialNeuralConfiguration::Classic => Genome::random(&mut self.random_state),
            InitialNeuralConfiguration::MotorMinimum => {
                Genome::random_with_motor_minimum(&mut self.random_state)
            }
            // Sem uma run válida, manter o modo funcional e explícito: ele
            // cai no clássico, nunca em um genoma incompleto.
            InitialNeuralConfiguration::ValidatedV1 | InitialNeuralConfiguration::ValidatedV2 => {
                Genome::random(&mut self.random_state)
            }
        });
        let genome = match initial_neural_configuration {
            InitialNeuralConfiguration::ValidatedV1 => genome.with_validated_v1_motor_group(),
            InitialNeuralConfiguration::ValidatedV2 => genome.with_validated_v2_cooperation(),
            InitialNeuralConfiguration::Classic | InitialNeuralConfiguration::MotorMinimum => {
                genome
            }
        };
        let longevity_gene =
            LongevityGene::founder(self.next_random_unit(), initial_neural_configuration);
        let organism = self.make_organism(position, genome, longevity_gene, 1, STARTING_ENERGY);
        self.organisms.push(organism);
    }

    fn make_organism(
        &mut self,
        position: Vec2,
        genome: Genome,
        longevity_gene: LongevityGene,
        generation: u32,
        energy: f32,
    ) -> Organism {
        let mut brain = Brain::default();
        brain.configure_network(genome.decode_connections(), genome.neural_node_activity());
        let organism = Organism {
            id: self.next_organism_id,
            position,
            energy,
            age_ticks: 0,
            longevity_gene,
            generation,
            children_produced: 0,
            crowding_violation: false,
            external_force: Vec2::ZERO,
            motors: MotorOutput::default(),
            sensors: SensorInput::default(),
            brain,
            genome,
        };
        self.next_organism_id += 1;
        organism
    }

    fn next_random_unit(&mut self) -> f32 {
        self.random_state = self
            .random_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        // Usa 32 bits completos: a versão anterior extraía 24 bits e dividia
        // por um máximo de 32, concentrando todos os nascimentos perto de 0.
        ((self.random_state >> 32) as u32) as f32 / (u32::MAX as f32)
    }
}

fn grid_cell_with_size(position: Vec2, cell_size: f32) -> (i32, i32) {
    (
        (position.x / cell_size).floor() as i32,
        (position.y / cell_size).floor() as i32,
    )
}

fn pair_key(left: usize, right: usize) -> (usize, usize) {
    if left < right {
        (left, right)
    } else {
        (right, left)
    }
}

fn read_sensors(
    reader: &mut impl Read,
    checkpoint_format: CheckpointFormat,
) -> io::Result<SensorInput> {
    if matches!(
        checkpoint_format,
        CheckpointFormat::PreDonation
            | CheckpointFormat::PreWorldExpansion
            | CheckpointFormat::PreLongevity
            | CheckpointFormat::PreEventCounters
            | CheckpointFormat::PreAgeCounters
            | CheckpointFormat::PreFoodSensorRange
            | CheckpointFormat::PreNeuralNodeActivity
            | CheckpointFormat::Current
    ) {
        return Ok(SensorInput {
            food_left: read_f32(reader)?,
            food_right: read_f32(reader)?,
            food_up: read_f32(reader)?,
            food_down: read_f32(reader)?,
            current_x: read_f32(reader)?,
            current_y: read_f32(reader)?,
            energy: read_f32(reader)?,
            constant: read_f32(reader)?,
            cell_contact: read_f32(reader)?,
            pg_intensity: read_f32(reader)?,
            pg_gradient_x: read_f32(reader)?,
            pg_gradient_y: read_f32(reader)?,
            p1_intensity: read_f32(reader)?,
            p1_gradient_x: read_f32(reader)?,
            p1_gradient_y: read_f32(reader)?,
            p2_intensity: read_f32(reader)?,
            p2_gradient_x: read_f32(reader)?,
            p2_gradient_y: read_f32(reader)?,
            p3_intensity: read_f32(reader)?,
            p3_gradient_x: read_f32(reader)?,
            p3_gradient_y: read_f32(reader)?,
            force_x: read_f32(reader)?,
            force_y: read_f32(reader)?,
        });
    }
    if checkpoint_format == CheckpointFormat::V2 {
        // A V2 tinha uma ordem de leitura histórica invertida para Ct/Sm.
        // Mantemos o consumo binário compatível; o retrato é recalculado no
        // primeiro tick restaurado e substitui esses valores transitórios.
        let cell_contact = read_f32(reader)?;
        let contact_similarity = read_f32(reader)?;
        return Ok(SensorInput {
            food_left: read_f32(reader)?,
            food_right: read_f32(reader)?,
            food_up: read_f32(reader)?,
            food_down: read_f32(reader)?,
            current_x: read_f32(reader)?,
            current_y: read_f32(reader)?,
            energy: read_f32(reader)?,
            constant: read_f32(reader)?,
            cell_contact,
            pg_intensity: contact_similarity,
            force_x: 0.0,
            force_y: 0.0,
            ..SensorInput::default()
        });
    }

    let mut sensors = SensorInput {
        food_left: read_f32(reader)?,
        food_right: read_f32(reader)?,
        food_up: read_f32(reader)?,
        food_down: read_f32(reader)?,
        current_x: read_f32(reader)?,
        current_y: read_f32(reader)?,
        energy: read_f32(reader)?,
        constant: read_f32(reader)?,
        ..SensorInput::default()
    };
    if checkpoint_format != CheckpointFormat::V1 {
        sensors.cell_contact = read_f32(reader)?;
        sensors.pg_intensity = read_f32(reader)?;
    }
    if matches!(
        checkpoint_format,
        CheckpointFormat::V3
            | CheckpointFormat::PrePheromone
            | CheckpointFormat::PreDonation
            | CheckpointFormat::PreWorldExpansion
            | CheckpointFormat::PreLongevity
            | CheckpointFormat::PreEventCounters
            | CheckpointFormat::PreAgeCounters
            | CheckpointFormat::PreFoodSensorRange
            | CheckpointFormat::PreNeuralNodeActivity
            | CheckpointFormat::Current
    ) {
        sensors.force_x = read_f32(reader)?;
        sensors.force_y = read_f32(reader)?;
    }
    Ok(sensors)
}

fn two_mut<T>(items: &mut [T], first: usize, second: usize) -> (&mut T, &mut T) {
    debug_assert!(first < second);
    let (left, right) = items.split_at_mut(second);
    (&mut left[first], &mut right[0])
}

fn resolve_pair_separation(
    left: &mut Organism,
    right: &mut Organism,
    exclusion_distance: f32,
    force: f32,
) -> (Vec2, Vec2) {
    let offset = right.position - left.position;
    let distance = offset.length();
    if distance >= exclusion_distance {
        return (Vec2::ZERO, Vec2::ZERO);
    }
    let direction = if distance > 0.001 {
        offset / distance
    } else {
        spread_direction(left.id ^ right.id)
    };
    let desired_correction = (exclusion_distance - distance) * force;
    let left_capacity = travel_capacity(left.position, -direction, left.physical_radius());
    let right_capacity = travel_capacity(right.position, direction, right.physical_radius());
    let mut left_move = (desired_correction * 0.5).min(left_capacity);
    let mut right_move = (desired_correction * 0.5).min(right_capacity);
    let mut remaining = desired_correction - left_move - right_move;
    let left_extra = (left_capacity - left_move).max(0.0).min(remaining);
    left_move += left_extra;
    remaining -= left_extra;
    right_move += (right_capacity - right_move).max(0.0).min(remaining);
    let left_force = -direction * left_move;
    let right_force = direction * right_move;
    left.position += left_force;
    right.position += right_force;
    (left_force, right_force)
}

fn travel_capacity(position: Vec2, direction: Vec2, radius: f32) -> f32 {
    let mut capacity = f32::INFINITY;
    if direction.x > 0.000_1 {
        capacity = capacity.min((WORLD_WIDTH - radius - position.x) / direction.x);
    } else if direction.x < -0.000_1 {
        capacity = capacity.min((radius - position.x) / direction.x);
    }
    if direction.y > 0.000_1 {
        capacity = capacity.min((WORLD_HEIGHT - radius - position.y) / direction.y);
    } else if direction.y < -0.000_1 {
        capacity = capacity.min((radius - position.y) / direction.y);
    }
    capacity.max(0.0)
}

fn physical_radius_for_energy(energy: f32) -> f32 {
    // A área relativa vai de 1 em energia zero a 4 em 2.000 de energia. Não há
    // clamp: energia adicional continua virando área, mas o raio cresce pela
    // raiz quadrada da área.
    let area_ratio = 1.0 + 3.0 * energy.max(0.0) / SIZE_GROWTH_ENERGY;
    CELL_RADIUS * area_ratio.sqrt()
}

fn clamp_coordinate_to_world(position: f32, radius: f32, dimension: f32) -> f32 {
    if radius * 2.0 >= dimension {
        dimension * 0.5
    } else {
        position.clamp(radius, dimension - radius)
    }
}

fn spread_direction(id: u64) -> Vec2 {
    let mixed = id.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(19);
    let angle = (mixed as f64 / u64::MAX as f64 * std::f64::consts::TAU) as f32;
    vec2(angle.cos(), angle.sin())
}

fn sensor_grid_cell(position: Vec2) -> (i32, i32) {
    (
        (position.x / MAX_FOOD_SENSOR_RANGE).floor() as i32,
        (position.y / MAX_FOOD_SENSOR_RANGE).floor() as i32,
    )
}

/// Distância ao alimento mais próximo em cada cone, na ordem
/// esquerda, direita, cima e baixo. O empate diagonal pertence ao cone
/// horizontal; assim, nenhum alimento ativa dois sensores no mesmo tick.
fn nearest_food_in_cones(
    grid: &HashMap<(i32, i32), Vec<Vec2>>,
    origin: Vec2,
    range: f32,
    map: &MapDefinition,
) -> [Option<f32>; 4] {
    let mut nearest = [None; 4];
    let base = sensor_grid_cell(origin);
    // O alelo tem teto próprio, mas o corpo pode ampliar o alcance efetivo.
    // A grade calcula quantas células visitar, portanto não cortamos esse
    // benefício geométrico no máximo genético de 480 unidades.
    let range = range.max(0.0);
    let range_squared = range * range;
    let grid_radius = (range / MAX_FOOD_SENSOR_RANGE).ceil() as i32;
    for grid_y in base.1 - grid_radius..=base.1 + grid_radius {
        for grid_x in base.0 - grid_radius..=base.0 + grid_radius {
            let Some(food_positions) = grid.get(&(grid_x, grid_y)) else {
                continue;
            };
            for &food_position in food_positions {
                let relative = food_position - origin;
                let distance_squared = relative.length_squared();
                if distance_squared > range_squared
                    || distance_squared == 0.0
                    || !map.has_clear_line(origin, food_position)
                {
                    continue;
                }
                let direction = if relative.x.abs() >= relative.y.abs() {
                    if relative.x < 0.0 { 0 } else { 1 }
                } else if relative.y < 0.0 {
                    2
                } else {
                    3
                };
                let distance = distance_squared.sqrt();
                if nearest[direction].is_none_or(|known| distance < known) {
                    nearest[direction] = Some(distance);
                }
            }
        }
    }
    nearest
}

fn cone_signal(distance: Option<f32>, range: f32) -> f32 {
    distance
        .map(|distance| 1.0 - distance / range.max(f32::EPSILON))
        .unwrap_or(0.0)
}

fn write_u32(writer: &mut impl Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_u64(writer: &mut impl Write, value: u64) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}
fn write_u16(writer: &mut impl Write, value: u16) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_f32(writer: &mut impl Write, value: f32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_bool(writer: &mut impl Write, value: bool) -> io::Result<()> {
    writer.write_all(&[value as u8])
}

fn read_u32(reader: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(reader: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}
fn read_u16(reader: &mut impl Read) -> io::Result<u16> {
    let mut bytes = [0; 2];
    reader.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_f32(reader: &mut impl Read) -> io::Result<f32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(f32::from_le_bytes(bytes))
}

fn read_bool(reader: &mut impl Read) -> io::Result<bool> {
    let mut byte = [0];
    reader.read_exact(&mut byte)?;
    match byte[0] {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "booleano inválido",
        )),
    }
}

fn read_count(reader: &mut impl Read, maximum: usize) -> io::Result<usize> {
    let count = read_u32(reader)? as usize;
    if count > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "arquivo grande demais",
        ));
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_energy_cell_divides_into_two_equal_energy_cells() {
        let mut world = World::new(7, 20);
        world.organisms.truncate(1);
        world.organisms[0].energy = MAX_ENERGY;

        world.reproduce_ready_organisms(MAX_ENERGY, MAX_ENERGY);

        assert_eq!(world.organisms.len(), 2);
        assert_eq!(world.births_total, 1);
        assert!(
            world
                .organisms
                .iter()
                .all(|organism| organism.energy == STARTING_ENERGY)
        );
    }

    #[test]
    fn reproduction_motor_can_divide_between_minimum_and_limit() {
        let mut world = World::new(9, 1);
        world.organisms[0].energy = 200.0;
        world.organisms[0].motors.reproduce = REPRODUCTION_ACTIVATION_THRESHOLD;

        world.reproduce_ready_organisms(200.0, 1_000.0);

        assert_eq!(world.organisms.len(), 2);
        assert!(
            world
                .organisms
                .iter()
                .all(|organism| organism.energy == 100.0)
        );
    }

    #[test]
    fn food_is_reported_by_one_directional_cone_only() {
        let origin = vec2(1_000.0, 1_000.0);
        let mut grid = HashMap::new();
        // Esquerda e levemente acima: a componente horizontal domina.
        let food = origin + vec2(-60.0, -20.0);
        grid.entry(sensor_grid_cell(food))
            .or_insert_with(Vec::new)
            .push(food);

        let nearest = nearest_food_in_cones(
            &grid,
            origin,
            DEFAULT_FOOD_SENSOR_RANGE,
            &MapDefinition::empty(),
        );

        assert!(nearest[0].is_some());
        assert!(nearest[1].is_none());
        assert!(nearest[2].is_none());
        assert!(nearest[3].is_none());
    }

    #[test]
    fn wall_blocks_food_sensor_line_of_sight() {
        let origin = vec2(1_000.0, 1_000.0);
        let food = origin + vec2(120.0, 0.0);
        let mut grid = HashMap::new();
        grid.entry(sensor_grid_cell(food))
            .or_insert_with(Vec::new)
            .push(food);
        let mut map = MapDefinition::empty();
        map.paint_wall(origin + vec2(60.0, 0.0), true);

        let nearest = nearest_food_in_cones(&grid, origin, DEFAULT_FOOD_SENSOR_RANGE, &map);

        assert_eq!(nearest, [None; 4]);
    }

    #[test]
    fn longer_food_sensor_reaches_food_outside_the_default_range() {
        let origin = vec2(1_000.0, 1_000.0);
        let food = origin + vec2(360.0, 0.0);
        let mut grid = HashMap::new();
        grid.entry(sensor_grid_cell(food))
            .or_insert_with(Vec::new)
            .push(food);

        let default_range = nearest_food_in_cones(
            &grid,
            origin,
            DEFAULT_FOOD_SENSOR_RANGE,
            &MapDefinition::empty(),
        );
        let long_range = nearest_food_in_cones(&grid, origin, 400.0, &MapDefinition::empty());

        assert_eq!(default_range, [None; 4]);
        assert_eq!(long_range[1], Some(360.0));
    }

    #[test]
    fn longer_food_sensor_has_a_quadratically_higher_upkeep() {
        let mut world = World::new(2_021, 1);
        let organism = &mut world.organisms[0];
        let default_cost = organism.food_sensor_energy_cost();
        let mut bits = *organism.genome.bytes();
        bits[PRE_NEURAL_NODE_ACTIVITY_GENOME_BYTES - 1] = 0b1111_1000;
        organism.genome = Genome::from_bytes(bits);

        assert!(organism.food_sensor_range() > DEFAULT_FOOD_SENSOR_RANGE);
        assert!(organism.food_sensor_energy_cost() > default_cost * 3.5);
    }

    #[test]
    fn body_size_makes_sensor_more_efficient_and_slows_swimming() {
        let mut world = World::new(2_022, 1);
        let organism = &mut world.organisms[0];
        organism.energy = 0.0;
        let base_range = organism.food_sensor_range();
        let base_cost = organism.food_sensor_energy_cost();
        organism.energy = SIZE_GROWTH_ENERGY;

        assert!((organism.physical_radius() / CELL_RADIUS - 2.0).abs() < 0.000_1);
        assert!((organism.food_sensor_range() / base_range - 8.0).abs() < 0.000_1);
        assert!((organism.food_sensor_energy_cost() / base_cost - 4.0).abs() < 0.000_1);
        assert!(organism.maximum_swim_speed() < crate::config::MAX_SWIM_SPEED);
        assert!(
            (organism.maximum_swim_speed() - crate::config::MAX_SWIM_SPEED / 2.0_f32.sqrt()).abs()
                < 0.000_1
        );
    }

    #[test]
    fn phagocytosis_cost_uses_the_large_body_multiplier() {
        let mut world = World::new(2_023, 2);
        world.organisms[0].energy = SIZE_GROWTH_ENERGY;
        world.organisms[1].energy = STARTING_ENERGY;
        world.organisms[0].position = vec2(1_000.0, 1_000.0);
        world.organisms[1].position = vec2(1_020.0, 1_000.0);
        world.organisms[0].motors.phagocytose = 1.0;
        let size_capture_scale = (world.organisms[0].physical_radius() / CELL_RADIUS).powi(3);

        world.apply_phagocytosis();

        // A energia do predador é descontada antes da física, então a tração
        // usa o raio residual deste mesmo tick.
        let size_advantage =
            (world.organisms[0].physical_radius() / world.organisms[1].physical_radius() - 1.0)
                .clamp(0.0, 1.0);
        let actual_size_capture_scale =
            (world.organisms[0].physical_radius() / CELL_RADIUS).powi(3);
        let expected_pull =
            MAX_PHAGOCYTOSIS_PULL_PER_TICK * size_advantage * actual_size_capture_scale;

        assert!(
            (world.organisms[0].energy - (SIZE_GROWTH_ENERGY - 0.04 * MAX_BODY_COST_MULTIPLIER))
                .abs()
                < 0.000_1
        );
        // Com 2R, a escala corporal da tração é 2³; 20% move o predador e
        // 80% aproxima a presa. A distância inicial evita captura instantânea.
        assert!((size_capture_scale - 8.0).abs() < 0.000_1);
        assert!((world.organisms[0].position.x - (1_000.0 + expected_pull * 0.20)).abs() < 0.000_1);
        assert!((world.organisms[1].position.x - (1_020.0 - expected_pull * 0.80)).abs() < 0.000_1);
    }

    #[test]
    fn touching_cells_receive_contact_and_genetic_pheromone() {
        let mut world = World::new_with_map(17, 2, MapDefinition::empty());
        world.organisms[0].position = vec2(1_000.0, 1_000.0);
        world.organisms[1].position = world.organisms[0].position;
        world.organisms[1].genome = world.organisms[0].genome.clone();
        let still_water = FlowField::new(17).with_intensity(0.0);

        world.pheromones.rebuild(&world.organisms, &world.map);
        world.update_sensors(&still_water, MAX_ENERGY);

        for organism in &world.organisms {
            assert_eq!(organism.sensors.cell_contact, 1.0);
            assert!(organism.sensors.pg_intensity > 0.5);
        }
    }

    #[test]
    fn donation_transfers_energy_without_crossing_the_reserve() {
        let mut world = World::new_with_map(23, 2, MapDefinition::empty());
        world.organisms[0].position = vec2(1_000.0, 1_000.0);
        world.organisms[1].position = world.organisms[0].position;
        world.organisms[0].energy = 100.0;
        world.organisms[1].energy = STARTING_ENERGY;
        world.organisms[0].motors.donate = 1.0;

        world.apply_donations(MAX_ENERGY);

        assert!(
            (world.organisms[1].energy - (STARTING_ENERGY + MAX_DONATION_PER_TICK)).abs() < 0.000_1
        );
        assert!(world.organisms[0].energy > DONATION_RESERVE_ENERGY);
        assert!(world.organisms[0].energy < 100.0 - MAX_DONATION_PER_TICK);
    }

    #[test]
    fn longevity_gene_removes_a_cell_at_its_hereditary_limit() {
        let mut world = World::new_with_map(83, 1, MapDefinition::empty());
        let limit = world.organisms[0].maximum_age_ticks();
        world.organisms[0].age_ticks = limit - 1;
        world.organisms[0].energy = 100.0;

        world.tick(
            &FlowField::new(83).with_intensity(0.0),
            0.0,
            0.0,
            600,
            FOOD_ENERGY,
            DEFAULT_MENTAL_ENERGY_COST,
            MAX_ENERGY,
            1_000.0,
        );

        assert!(world.organisms.is_empty());
        assert_eq!(world.deaths_total, 1);
    }

    #[test]
    fn profiled_tick_preserves_the_same_world_state_as_a_normal_tick() {
        let mut normal = World::new(84, 8);
        let mut profiled = normal.clone();
        let flow = FlowField::new(84).with_intensity(0.0);

        normal.tick(
            &flow,
            0.12,
            0.24,
            600,
            FOOD_ENERGY,
            DEFAULT_MENTAL_ENERGY_COST,
            MAX_ENERGY,
            1_000.0,
        );
        let profile = profiled.tick_profiled(
            &flow,
            0.12,
            0.24,
            600,
            FOOD_ENERGY,
            DEFAULT_MENTAL_ENERGY_COST,
            MAX_ENERGY,
            1_000.0,
        );

        let mut normal_checkpoint = Vec::new();
        normal.write_checkpoint(&mut normal_checkpoint).unwrap();
        let mut profiled_checkpoint = Vec::new();
        profiled.write_checkpoint(&mut profiled_checkpoint).unwrap();
        assert_eq!(normal_checkpoint, profiled_checkpoint);
        assert!(!profile.total().is_zero());
    }

    #[test]
    fn validated_v2_forces_the_contact_and_pg_donation_gate() {
        let mut random = 51;
        let founder = Genome::random(&mut random);
        let world = World::new_with_initial_population(
            52,
            1,
            MapDefinition::empty(),
            InitialNeuralConfiguration::ValidatedV2,
            &[founder],
        );
        let connections = world.organisms[0].genome.decode_connections();
        for (source, destination) in [
            (0, crate::simulation::brain::MOTOR_START),
            (1, crate::simulation::brain::MOTOR_START + 1),
            (2, crate::simulation::brain::MOTOR_START + 2),
            (3, crate::simulation::brain::MOTOR_START + 3),
        ] {
            assert!(connections.iter().any(|connection| {
                connection.source == source
                    && connection.destination == destination
                    && connection.weight > 0.0
            }));
        }
        assert!(connections.iter().any(|connection| connection.source == 8
            && connection.destination == crate::simulation::brain::INTERNAL_START));
        assert!(connections.iter().any(|connection| connection.source == 9
            && connection.destination == crate::simulation::brain::INTERNAL_START));
        assert_eq!(
            connections
                .iter()
                .filter(|connection| {
                    connection.source == crate::simulation::brain::INTERNAL_START
                        && connection.destination == crate::simulation::brain::SLOT_COUNT - 1
                })
                .count(),
            3
        );
    }

    #[test]
    fn energy_and_force_sensors_use_reproduction_limit_and_previous_force() {
        let mut world = World::new(18, 1);
        world.organisms[0].energy = 100.0;
        world.organisms[0].external_force = vec2(2.0, -4.0);
        let still_water = FlowField::new(18).with_intensity(0.0);

        world.update_sensors(&still_water, 1_000.0);

        assert!((world.organisms[0].sensors.energy - 0.1).abs() < 0.000_1);
        assert!((world.organisms[0].sensors.force_x - 0.5).abs() < 0.000_1);
        assert!((world.organisms[0].sensors.force_y + 1.0).abs() < 0.000_1);
    }

    #[test]
    fn phagocytosis_needs_ten_percent_radius_advantage() {
        let mut world = World::new(19, 2);
        world.organisms[0].energy = 60.0;
        world.organisms[1].energy = 500.0;
        world.organisms[0].position = vec2(1_000.0, 1_000.0);
        world.organisms[1].position = vec2(1_020.0, 1_000.0);
        world.organisms[0].motors.phagocytose = 1.0;

        let outcome = world.apply_phagocytosis();

        // A célula menor tentou: pagou o esforço, mas não exerce sucção.
        assert!(world.organisms[0].external_force.length() < 0.000_1);
        assert!(world.organisms[0].energy < 60.0);
        assert!(outcome.protected_pairs.is_empty());
    }

    #[test]
    fn contained_smaller_cell_is_consumed_by_phagocytosis() {
        let mut world = World::new(20, 2);
        world.organisms[0].energy = SIZE_GROWTH_ENERGY;
        world.organisms[1].energy = 60.0;
        world.organisms[0].position = vec2(1_000.0, 1_000.0);
        world.organisms[1].position = vec2(1_005.0, 1_000.0);
        world.organisms[0].motors.phagocytose = 1.0;

        let outcome = world.apply_phagocytosis();
        world.consume_phagocytosed(&outcome.capture_candidates, 3_000.0);

        assert_eq!(world.organisms.len(), 1);
        assert_eq!(world.deaths_total, 1);
        assert!(world.organisms[0].energy > SIZE_GROWTH_ENERGY);
    }

    #[test]
    fn checkpoint_round_trip_preserves_world_and_brain_state() {
        let mut world = World::new(13, 2);
        world.tick = 456;
        world.organisms[0].energy = 83.5;
        let custom_wall = vec2(2_400.0, 1_200.0);
        world.map.paint_wall(custom_wall, true);
        let mut bytes = Vec::new();
        world.write_checkpoint(&mut bytes).unwrap();

        let restored = World::read_checkpoint(&mut bytes.as_slice()).unwrap();

        assert_eq!(restored.tick, 456);
        assert_eq!(restored.organisms.len(), world.organisms.len());
        assert_eq!(restored.food.len(), world.food.len());
        assert_eq!(restored.organisms[0].energy, 83.5);
        assert_eq!(
            restored.organisms[0].brain.internal_state(),
            world.organisms[0].brain.internal_state()
        );
        assert!(restored.map.is_wall(custom_wall));
    }

    #[test]
    fn amadeus_starts_one_random_cell_at_world_center() {
        let world = World::new_amadeus_with_map(771, MapDefinition::default());

        assert_eq!(world.organisms.len(), 1);
        assert_eq!(
            world.organisms[0].position,
            vec2(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5)
        );
    }

    #[test]
    fn validated_population_uses_only_genomes_from_the_saved_founder_pool() {
        let mut state = 61;
        let founders = vec![Genome::random(&mut state), Genome::random(&mut state)];
        let world = World::new_with_initial_population(
            62,
            7,
            MapDefinition::default(),
            InitialNeuralConfiguration::ValidatedV1,
            &founders,
        );

        assert_eq!(world.organisms.len(), 7);
        assert!(world.organisms.iter().all(|organism| {
            founders
                .iter()
                .any(|founder| founder.bytes() == organism.genome.bytes())
        }));
    }

    #[test]
    fn validated_v2_founders_start_in_the_long_lifespan_band() {
        let world = World::new_with_initial_neural_configuration(
            91,
            16,
            MapDefinition::default(),
            InitialNeuralConfiguration::ValidatedV2,
        );

        assert!(world.organisms.iter().all(|organism| {
            (VALIDATED_V2_FOUNDER_LIFESPAN_MIN_TICKS..=VALIDATED_V2_FOUNDER_LIFESPAN_MAX_TICKS)
                .contains(&organism.maximum_age_ticks())
        }));
    }

    #[test]
    fn senescence_is_late_and_reaches_zero_at_the_age_limit() {
        let mut world = World::new(92, 1);
        let organism = &mut world.organisms[0];
        organism.longevity_gene = LongevityGene::from_age_ticks(10_000);

        organism.age_ticks = 5_000;
        assert!(organism.energy_efficiency() > 0.97);
        organism.age_ticks = 9_000;
        assert!(organism.energy_efficiency() < 0.60);
        organism.age_ticks = 10_000;
        assert_eq!(organism.energy_efficiency(), 0.0);
    }

    #[test]
    fn zero_current_does_not_move_phytoplankton() {
        let mut world = World::new(7, 0);
        let positions_before: Vec<Vec2> = world.food.iter().map(|food| food.position).collect();
        let still_water = FlowField::new(7).with_intensity(0.0);

        world.tick(
            &still_water,
            0.0,
            0.0,
            20_000,
            FOOD_ENERGY,
            DEFAULT_MENTAL_ENERGY_COST,
            MAX_ENERGY,
            MAX_ENERGY,
        );

        let positions_after: Vec<Vec2> = world.food.iter().map(|food| food.position).collect();
        assert_eq!(positions_before, positions_after);
    }

    #[test]
    fn anti_cramming_separates_cells_at_the_same_position() {
        let mut world = World::new(7, 2);
        world.organisms[0].energy = MAX_ENERGY;
        world.organisms[1].energy = MAX_ENERGY;
        world.organisms[1].position = world.organisms[0].position;
        let initial_distance = world.organisms[0]
            .position
            .distance(world.organisms[1].position);
        world.set_anti_cramming(AntiCrammingConfig {
            enabled: true,
            force: 1.0,
            exclusion_distance: MIN_EXCLUSION_DISTANCE,
        });

        world.apply_anti_cramming(&HashSet::new());

        let final_distance = world.organisms[0]
            .position
            .distance(world.organisms[1].position);
        let expected_distance =
            world.organisms[0].physical_radius() + world.organisms[1].physical_radius();
        assert_eq!(initial_distance, 0.0);
        assert!(final_distance >= expected_distance - 0.01);
        assert!(expected_distance > MIN_EXCLUSION_DISTANCE);
        assert_eq!(world.crowding.violating_pairs, 0);
    }

    #[test]
    fn physical_radius_grows_without_a_ceiling() {
        let mut world = World::new(11, 1);
        let organism = &mut world.organisms[0];
        organism.energy = 0.0;
        let empty_radius = organism.physical_radius();
        organism.energy = SIZE_GROWTH_ENERGY;
        let double_radius = organism.physical_radius();
        organism.energy = 10_000.0;
        let larger_radius = organism.physical_radius();

        assert_eq!(empty_radius, CELL_RADIUS);
        assert!((double_radius - CELL_RADIUS * 2.0).abs() < 0.000_1);
        assert!(larger_radius > double_radius);
        assert!((larger_radius - CELL_RADIUS * 4.0).abs() < 0.000_1);
    }

    #[test]
    fn doubled_radius_cost_is_forty_percent_higher_but_keeps_growing() {
        let mut world = World::new(12, 1);
        let organism = &mut world.organisms[0];
        organism.energy = 0.0;
        assert!((organism.physical_area_scale() - 1.0).abs() < 0.000_1);

        organism.energy = SIZE_GROWTH_ENERGY;
        assert!((organism.physical_area_scale() - MAX_BODY_COST_MULTIPLIER).abs() < 0.000_1);

        organism.energy = 10_000.0;
        assert!(organism.physical_area_scale() > MAX_BODY_COST_MULTIPLIER);
    }

    #[test]
    fn zero_anti_cramming_force_keeps_overlapping_cells_unchanged() {
        let mut world = World::new(9, 2);
        world.organisms[1].position = world.organisms[0].position;
        world.set_anti_cramming(AntiCrammingConfig {
            enabled: true,
            force: 0.0,
            exclusion_distance: MAX_EXCLUSION_DISTANCE,
        });

        world.apply_anti_cramming(&HashSet::new());

        assert_eq!(world.organisms[0].position, world.organisms[1].position);
    }
}
