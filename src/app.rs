use macroquad::prelude::*;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
    process::{self, Child, Command},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use crate::{
    config::{WORLD_HEIGHT, WORLD_WIDTH},
    environment::{
        flow::{FlowField, FlowPreset},
        map::{MAP_GRID_HEIGHT, MAP_GRID_WIDTH, MapDefinition},
    },
    simulation::{
        brain::{
            EXTERNAL_SLOT_COUNT, INTERNAL_START, MOTOR_START, SLOT_COUNT, TIMER_INTERNAL_START,
        },
        genome::{
            CONNECTION_RECORDS, GENOME_BIT_COUNT, GENOME_BYTES, Genome,
            PRE_FOOD_SENSOR_RANGE_GENOME_BYTES, PRE_NEURAL_NODE_ACTIVITY_GENOME_BYTES,
        },
        pheromone::{
            FIELD_HEIGHT as PHEROMONE_FIELD_HEIGHT, FIELD_WIDTH as PHEROMONE_FIELD_WIDTH,
            PheromoneDisplay,
        },
        world::{
            AntiCrammingConfig, CELL_RADIUS, DEFAULT_MENTAL_ENERGY_COST, FOOD_ENERGY, FOOD_RADIUS,
            InitialNeuralConfiguration, MAX_ENERGY, MAX_EXCLUSION_DISTANCE, MIN_EXCLUSION_DISTANCE,
            Organism, Phytoplankton, World,
        },
    },
};

const PANEL_WIDTH: f32 = 336.0;
const BACKGROUND: Color = Color::new(0.035, 0.043, 0.059, 1.0);
const SURFACE: Color = Color::new(0.075, 0.09, 0.12, 1.0);
const SURFACE_HOVER: Color = Color::new(0.12, 0.145, 0.19, 1.0);
const BORDER: Color = Color::new(0.20, 0.235, 0.30, 1.0);
const TEXT: Color = Color::new(0.91, 0.93, 0.96, 1.0);
const MUTED: Color = Color::new(0.56, 0.62, 0.70, 1.0);
const ACCENT: Color = Color::new(0.20, 0.66, 0.93, 1.0);
const RED: Color = Color::new(0.91, 0.12, 0.16, 1.0);
const SPEED_MULTIPLIERS: [f32; 7] = [1.0, 10.0, 100.0, 1_000.0, 10_000.0, 100_000.0, 1_000_000.0];
const TELEMETRY_INTERVAL_TICKS: u64 = 250;
const MAX_TELEMETRY_SAMPLES: usize = 12_000;
const GENETICS_REFRESH_TICKS: u64 = 250;
const GENETIC_GROUP_SIMILARITY: f32 = 0.985;
const ENERGY_HISTOGRAM_BUCKETS: usize = 12;
const SESSION_CONTENT_BOTTOM: f32 = 1_460.0;
const PRESET_MAGIC: [u8; 8] = *b"ECOPRE01";
const PRESET_FORMAT_VERSION: u32 = 2;
const MAP_PRESET_MAGIC: [u8; 8] = *b"ECOMAP01";
const MAP_PRESET_FORMAT_VERSION: u32 = 1;
const MAX_MAP_PRESETS: usize = 24;
const MAX_SESSION_PRESETS: usize = 80;
/// Preset-base escolhido para a distribuição compartilhável. Ele é incluído
/// no binário, logo um usuário novo não depende de arquivos do autor.
const PRESET_INICIAL_EMBUTIDO: &[u8] = include_bytes!("../assets/presets/preset_inicial.ecopreset");
/// Fonte histórica do desenho New Gods. O binário inclui a máscara original,
/// portanto o mapa em S não depende de um arquivo externo da máquina autora.
const NEW_GODS_MAP_EMBUTIDO: &[u8] = include_bytes!("../assets/presets/new_gods_source.ecopreset");
const PRESET_INICIAL_NOME: &str = "Novo preset3344";
/// Teto de energia que força a divisão. É um valor explícito, não um
/// múltiplo da energia mínima de reprodução.
const DEFAULT_REPRODUCTION_LIMIT: f32 = 5_000.0;
/// A interface usa uma escala humana: 100 pontos equivalem a 0,24/tick no
/// motor, preservando exatamente a configuração aprovada de 50/100.
const EMISSION_DISPLAY_UNIT: f32 = 0.0024;

#[derive(Clone)]
struct EngineSettings {
    seed: u64,
    amadeus_mode: bool,
    map: MapDefinition,
    flow_intensity: f32,
    flow_preset: FlowPreset,
    anti_cramming: AntiCrammingConfig,
    mental_energy_cost: f32,
    desert_emission: f32,
    fertile_emission: f32,
    lifetime: u32,
    food_energy: f32,
    /// Energia mínima: acima dela o motor Reproduzir pode ser aceito.
    reproduction_energy: f32,
    /// Capacidade física: neste teto a divisão é obrigatória.
    reproduction_limit: f32,
    initial_cells: usize,
    initial_neural_configuration: InitialNeuralConfiguration,
    validated_genomes: Vec<Genome>,
}

enum EngineCommand {
    Pause(bool),
    Speed(usize),
    UiFps(f32),
    /// Mantém a simulação viva, mas interrompe a cópia de snapshots para a
    /// interface. O painel headless continua recebendo apenas métricas leves.
    SetHeadless(bool),
    SetFlowIntensity(f32),
    SetFlowPreset(FlowPreset),
    SetDesertEmission(f32),
    SetFertileEmission(f32),
    SetFoodLifetime(u32),
    SetFoodEnergy(f32),
    SetReproductionEnergy(f32),
    SetReproductionLimit(f32),
    SetMentalEnergyCost(f32),
    SetAntiCramming(AntiCrammingConfig),
    SetMap(MapDefinition),
    Restart(EngineSettings),
    Load {
        world: World,
        settings: EngineSettings,
        telemetry: Vec<TelemetrySample>,
    },
}

#[derive(Clone, Copy, Default)]
struct EngineMetrics {
    ticks_per_second: f32,
    publication_hz: f32,
    active_desert_emission: f32,
    active_fertile_emission: f32,
    tick: u64,
    population: usize,
    headless: bool,
}

#[derive(Clone, Copy)]
struct TelemetrySample {
    tick: u64,
    population: usize,
    births_total: u64,
    deaths_total: u64,
    births_by_neural_decision_total: u64,
    births_by_energy_limit_total: u64,
    deaths_by_starvation_total: u64,
    deaths_by_phagocytosis_total: u64,
    deaths_by_age_total: u64,
    average_energy: f32,
    average_age: f32,
    average_generation: f32,
    max_generation: u32,
    genetic_diversity: f32,
}

#[derive(Default)]
struct TelemetryLog {
    samples: VecDeque<TelemetrySample>,
}

impl TelemetryLog {
    fn reset(&mut self, world: &World) {
        self.samples.clear();
        self.push(world);
    }

    fn push(&mut self, world: &World) {
        if self.samples.len() == MAX_TELEMETRY_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(capture_telemetry(world));
    }
}

struct SimulationEngine {
    commands: mpsc::Sender<EngineCommand>,
    latest: Arc<Mutex<Arc<World>>>,
    metrics: Arc<Mutex<EngineMetrics>>,
    telemetry: Arc<Mutex<TelemetryLog>>,
}

struct DemographicsWindowHandle {
    child: Child,
    data_path: PathBuf,
    last_written: Instant,
}

struct DemographicsWindowData {
    tick: u64,
    population: usize,
    births_total: u64,
    deaths_total: u64,
    births_by_neural_decision_total: u64,
    births_by_energy_limit_total: u64,
    deaths_by_starvation_total: u64,
    deaths_by_phagocytosis_total: u64,
    deaths_by_age_total: u64,
    energy_distribution: Vec<usize>,
    energy_bucket_width: f32,
    telemetry: Vec<TelemetrySample>,
}

#[derive(Clone)]
struct SavedRunInfo {
    path: PathBuf,
    run_id: u64,
    tick: u64,
    population: usize,
}

#[derive(Clone, Copy)]
struct RunEvent {
    tick: u64,
    before: AppliedSettings,
    after: AppliedSettings,
}

struct LoadedRun {
    run_id: u64,
    settings: EngineSettings,
    world: World,
    telemetry: Vec<TelemetrySample>,
    events: Vec<RunEvent>,
}

/// Um preset é somente a configuração de uma próxima sessão. Ele não guarda
/// população, tick ou telemetria do mundo atual — para isso há `.ecosrun`.
#[derive(Clone)]
struct SessionPreset {
    name: String,
    settings: EngineSettings,
}

/// Um preset de mapa é intencionalmente menor que um preset de sessão: ele
/// guarda apenas paredes, área fértil e o modelo de corrente.
#[derive(Clone)]
struct MapPreset {
    name: String,
    map: MapDefinition,
    flow_preset: FlowPreset,
}

/// População fundadora extraída do último estado de uma run. A origem fica em
/// disco como `.ecosrun`; a cópia em memória só dá à interface uma seleção
/// pronta para o próximo reinício.
#[derive(Clone)]
struct ValidatedPopulation {
    source_label: String,
    source_tick: u64,
    genomes: Vec<Genome>,
}

#[derive(Default)]
struct PanelAction {
    apply: bool,
    revert: bool,
    restart: bool,
    open_demographics: bool,
    save_run: bool,
    refresh_saved_runs: bool,
    load_path: Option<PathBuf>,
    save_preset: Option<String>,
    load_preset: Option<usize>,
    delete_preset: Option<usize>,
    save_map_preset: Option<String>,
    load_map_preset: Option<usize>,
    delete_map_preset: Option<usize>,
    open_headless_modal: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PanelTab {
    Session,
    Cells,
    Settings,
    Visibility,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditingField {
    Seed,
    InitialCells,
    ReproductionEnergy,
    ReproductionLimit,
    DesertEmission,
    FertileEmission,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MapTool {
    None,
    Wall,
    Eraser,
    FertileLasso,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DemographicWindow {
    Last5k,
    Last30k,
    Last100k,
    LastQuarter,
    EntireSession,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CellsView {
    Individual,
    Genetics,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GeneticsView {
    Summary,
    Groups,
    Circuits,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CircuitView {
    Connections,
    Neurons,
}

struct GeneticCluster {
    representative_id: u64,
    members: usize,
    average_generation: f32,
    average_similarity: f32,
    color: Color,
}

#[derive(Clone, Copy)]
struct ConnectionKpi {
    source: usize,
    destination: usize,
    organisms_using: usize,
    average_weight: f32,
    groups_using: usize,
}

#[derive(Clone, Copy)]
struct NeuronKpi {
    slot: usize,
    used_by: usize,
    outgoing_by: usize,
    incoming_by: usize,
}

struct GeneticsReport {
    tick: u64,
    population: usize,
    consensus_agreement: f32,
    clusters: Vec<GeneticCluster>,
    connections: Vec<ConnectionKpi>,
    neurons: Vec<NeuronKpi>,
}

struct SessionUi {
    paused: bool,
    speed: usize,
    speed_menu_open: bool,
    /// Quando ativo, a interface mostra somente o painel de controle leve;
    /// o worker deixa de copiar o mundo para desenho.
    headless: bool,
    headless_modal_open: bool,
    /// Um popup consumiu o clique atual. Enquanto o botão do mouse continua
    /// pressionado, controles sob ele (especialmente sliders) não reagem.
    pointer_captured_until_release: bool,
    show_flow: bool,
    show_sensor_reach: bool,
    show_pheromone_field: bool,
    show_fertile_area: bool,
    pheromone_display: PheromoneDisplay,
    pheromone_menu_open: bool,
    flow_enabled: bool,
    flow_intensity: f32,
    flow_preset: FlowPreset,
    flow_preset_modal_open: bool,
    preset_modal_open: bool,
    preset_name_input: String,
    preset_name_editing: bool,
    saved_presets: Vec<SessionPreset>,
    preset_status: Option<String>,
    map_preset_modal_open: bool,
    map_preset_name_input: String,
    map_preset_name_editing: bool,
    saved_map_presets: Vec<MapPreset>,
    map_preset_status: Option<String>,
    cells_section_open: bool,
    phytoplankton_section_open: bool,
    flow_section_open: bool,
    density_section_open: bool,
    anti_cramming: bool,
    anti_cramming_force: f32,
    anti_cramming_distance: f32,
    show_crowding_violations: bool,
    mental_energy_cost: f32,
    desert_emission: f32,
    fertile_emission: f32,
    lifetime: i32,
    food_energy: f32,
    initial_cells: usize,
    initial_neural_configuration: InitialNeuralConfiguration,
    validated_population: Option<ValidatedPopulation>,
    amadeus_mode: bool,
    edited_map: MapDefinition,
    applied_map: MapDefinition,
    map_tool: MapTool,
    wall_brush_radius: u8,
    lasso_points: Vec<Vec2>,
    last_brush_position: Option<Vec2>,
    map_dirty: bool,
    reproduction_energy: f32,
    reproduction_limit: f32,
    seed: u64,
    seed_input: String,
    initial_cells_input: String,
    reproduction_energy_input: String,
    reproduction_limit_input: String,
    desert_emission_input: String,
    fertile_emission_input: String,
    editing_field: Option<EditingField>,
    session_scroll: f32,
    applied: AppliedSettings,
    tab: PanelTab,
    selected_cell_id: Option<u64>,
    show_genome: bool,
    cells_view: CellsView,
    genetics_view: GeneticsView,
    circuit_view: CircuitView,
    genetics_report: Option<GeneticsReport>,
    demographics_window: Option<DemographicsWindowHandle>,
    run_id: u64,
    run_events: Vec<RunEvent>,
    saved_runs: Vec<SavedRunInfo>,
    save_status: Option<String>,
}

/// Foto da última configuração que chegou ao motor. Ela permite distinguir
/// claramente um valor apenas editado de um valor efetivamente aplicado.
#[derive(Clone, Copy, PartialEq)]
struct AppliedSettings {
    seed: u64,
    initial_cells: usize,
    initial_neural_configuration: InitialNeuralConfiguration,
    amadeus_mode: bool,
    reproduction_energy: f32,
    reproduction_limit: f32,
    mental_energy_cost: f32,
    desert_emission: f32,
    fertile_emission: f32,
    lifetime: i32,
    food_energy: f32,
    flow_enabled: bool,
    flow_intensity: f32,
    flow_preset: FlowPreset,
    anti_cramming: bool,
    anti_cramming_force: f32,
    anti_cramming_distance: f32,
}

impl Default for SessionUi {
    fn default() -> Self {
        let seed = 847_291_530;
        Self {
            paused: false,
            speed: 0,
            speed_menu_open: false,
            headless: false,
            headless_modal_open: false,
            pointer_captured_until_release: false,
            // Vetores são uma camada de leitura. A corrente física continua
            // ativa conforme a configuração, mas a tela começa limpa.
            show_flow: false,
            show_sensor_reach: false,
            show_pheromone_field: false,
            show_fertile_area: true,
            pheromone_display: PheromoneDisplay::Pg,
            pheromone_menu_open: false,
            flow_enabled: true,
            flow_intensity: 1.0,
            flow_preset: FlowPreset::Classic,
            flow_preset_modal_open: false,
            preset_modal_open: false,
            preset_name_input: "Novo preset".to_owned(),
            preset_name_editing: false,
            saved_presets: Vec::new(),
            preset_status: None,
            map_preset_modal_open: false,
            map_preset_name_input: "Novo mapa".to_owned(),
            map_preset_name_editing: false,
            saved_map_presets: Vec::new(),
            map_preset_status: None,
            cells_section_open: true,
            phytoplankton_section_open: true,
            flow_section_open: true,
            density_section_open: true,
            anti_cramming: true,
            anti_cramming_force: 1.0,
            anti_cramming_distance: MIN_EXCLUSION_DISTANCE,
            show_crowding_violations: false,
            mental_energy_cost: DEFAULT_MENTAL_ENERGY_COST,
            desert_emission: 0.12,
            fertile_emission: 0.24,
            lifetime: 600,
            food_energy: FOOD_ENERGY,
            initial_cells: 20,
            initial_neural_configuration: InitialNeuralConfiguration::Classic,
            validated_population: None,
            amadeus_mode: false,
            edited_map: MapDefinition::default(),
            applied_map: MapDefinition::default(),
            map_tool: MapTool::None,
            wall_brush_radius: 4,
            lasso_points: Vec::new(),
            last_brush_position: None,
            map_dirty: false,
            reproduction_energy: MAX_ENERGY,
            reproduction_limit: DEFAULT_REPRODUCTION_LIMIT,
            seed,
            seed_input: seed.to_string(),
            initial_cells_input: "20".to_owned(),
            reproduction_energy_input: MAX_ENERGY.to_string(),
            reproduction_limit_input: DEFAULT_REPRODUCTION_LIMIT.to_string(),
            desert_emission_input: "50".to_owned(),
            fertile_emission_input: "100".to_owned(),
            editing_field: None,
            session_scroll: 0.0,
            applied: AppliedSettings {
                seed,
                initial_cells: 20,
                initial_neural_configuration: InitialNeuralConfiguration::Classic,
                amadeus_mode: false,
                reproduction_energy: MAX_ENERGY,
                reproduction_limit: DEFAULT_REPRODUCTION_LIMIT,
                mental_energy_cost: DEFAULT_MENTAL_ENERGY_COST,
                desert_emission: 0.12,
                fertile_emission: 0.24,
                lifetime: 600,
                food_energy: FOOD_ENERGY,
                flow_enabled: true,
                flow_intensity: 1.0,
                flow_preset: FlowPreset::Classic,
                anti_cramming: true,
                anti_cramming_force: 1.0,
                anti_cramming_distance: MIN_EXCLUSION_DISTANCE,
            },
            tab: PanelTab::Session,
            selected_cell_id: None,
            show_genome: false,
            cells_view: CellsView::Individual,
            genetics_view: GeneticsView::Summary,
            circuit_view: CircuitView::Connections,
            genetics_report: None,
            demographics_window: None,
            run_id: seed,
            run_events: Vec::new(),
            saved_runs: Vec::new(),
            save_status: None,
        }
    }
}

impl SessionUi {
    /// A intensidade guardada no slider é preservada mesmo quando a corrente
    /// está desligada; o motor recebe zero somente enquanto ela estiver off.
    fn effective_flow_intensity(&self) -> f32 {
        if self.flow_enabled {
            self.flow_intensity
        } else {
            0.0
        }
    }

    fn anti_cramming_config(&self) -> AntiCrammingConfig {
        AntiCrammingConfig {
            enabled: self.anti_cramming,
            force: self.anti_cramming_force,
            exclusion_distance: self.anti_cramming_distance,
        }
    }

    fn current_settings(&self) -> AppliedSettings {
        AppliedSettings {
            seed: self.seed,
            initial_cells: self.initial_cells,
            initial_neural_configuration: self.initial_neural_configuration,
            amadeus_mode: self.amadeus_mode,
            reproduction_energy: self.reproduction_energy,
            reproduction_limit: self.reproduction_limit,
            mental_energy_cost: self.mental_energy_cost,
            desert_emission: self.desert_emission,
            fertile_emission: self.fertile_emission,
            lifetime: self.lifetime,
            food_energy: self.food_energy,
            flow_enabled: self.flow_enabled,
            flow_intensity: self.flow_intensity,
            flow_preset: self.flow_preset,
            anti_cramming: self.anti_cramming,
            anti_cramming_force: self.anti_cramming_force,
            anti_cramming_distance: self.anti_cramming_distance,
        }
    }

    fn mark_applied(&mut self) {
        self.applied = self.current_settings();
        self.applied_map = self.edited_map.clone();
    }

    fn mark_live_settings_applied(&mut self) {
        let current = self.current_settings();
        self.applied.reproduction_energy = current.reproduction_energy;
        self.applied.reproduction_limit = current.reproduction_limit;
        self.applied.mental_energy_cost = current.mental_energy_cost;
        self.applied.desert_emission = current.desert_emission;
        self.applied.fertile_emission = current.fertile_emission;
        self.applied.lifetime = current.lifetime;
        self.applied.food_energy = current.food_energy;
        self.applied.flow_enabled = current.flow_enabled;
        self.applied.flow_intensity = current.flow_intensity;
        self.applied.flow_preset = current.flow_preset;
        self.applied.anti_cramming = current.anti_cramming;
        self.applied.anti_cramming_force = current.anti_cramming_force;
        self.applied.anti_cramming_distance = current.anti_cramming_distance;
    }

    fn seed_is_dirty(&self) -> bool {
        self.seed_input
            .parse::<u64>()
            .map_or(true, |value| value != self.applied.seed)
    }

    fn initial_cells_are_dirty(&self) -> bool {
        self.initial_cells_input
            .parse::<usize>()
            .map_or(true, |value| value != self.applied.initial_cells)
    }

    fn initial_neural_configuration_is_dirty(&self) -> bool {
        self.initial_neural_configuration != self.applied.initial_neural_configuration
    }

    fn reproduction_energy_is_dirty(&self) -> bool {
        self.reproduction_energy_input
            .parse::<f32>()
            .map_or(true, |value| value != self.applied.reproduction_energy)
    }

    fn desert_emission_is_dirty(&self) -> bool {
        self.desert_emission_input
            .parse::<f32>()
            .map_or(true, |value| {
                (emission_from_display(value) - self.applied.desert_emission).abs() > f32::EPSILON
            })
    }

    fn fertile_emission_is_dirty(&self) -> bool {
        self.fertile_emission_input
            .parse::<f32>()
            .map_or(true, |value| {
                (emission_from_display(value) - self.applied.fertile_emission).abs() > f32::EPSILON
            })
    }

    fn reproduction_limit_is_dirty(&self) -> bool {
        self.reproduction_limit_input
            .parse::<f32>()
            .map_or(true, |value| value != self.applied.reproduction_limit)
    }

    fn has_pending_live_changes(&self) -> bool {
        self.reproduction_energy_is_dirty()
            || self.reproduction_limit_is_dirty()
            || self.desert_emission_is_dirty()
            || self.fertile_emission_is_dirty()
            || self.mental_energy_cost != self.applied.mental_energy_cost
            || self.lifetime != self.applied.lifetime
            || self.food_energy != self.applied.food_energy
            || self.flow_enabled != self.applied.flow_enabled
            || self.flow_intensity != self.applied.flow_intensity
            || self.flow_preset != self.applied.flow_preset
            || self.anti_cramming != self.applied.anti_cramming
            || self.anti_cramming_force != self.applied.anti_cramming_force
            || self.anti_cramming_distance != self.applied.anti_cramming_distance
    }

    fn has_pending_changes(&self) -> bool {
        self.has_pending_live_changes()
            || self.seed_is_dirty()
            || self.initial_cells_are_dirty()
            || self.initial_neural_configuration_is_dirty()
            || self.amadeus_mode != self.applied.amadeus_mode
            || self.map_dirty
    }

    fn revert_pending_changes(&mut self) {
        let applied = self.applied;
        self.seed = applied.seed;
        self.seed_input = applied.seed.to_string();
        self.initial_cells = applied.initial_cells;
        self.initial_cells_input = applied.initial_cells.to_string();
        self.initial_neural_configuration = applied.initial_neural_configuration;
        self.amadeus_mode = applied.amadeus_mode;
        self.reproduction_energy = applied.reproduction_energy;
        self.reproduction_energy_input = applied.reproduction_energy.to_string();
        self.reproduction_limit = applied.reproduction_limit;
        self.reproduction_limit_input = applied.reproduction_limit.to_string();
        self.mental_energy_cost = applied.mental_energy_cost;
        self.desert_emission = applied.desert_emission;
        self.desert_emission_input = format_emission_display(applied.desert_emission);
        self.fertile_emission = applied.fertile_emission;
        self.fertile_emission_input = format_emission_display(applied.fertile_emission);
        self.lifetime = applied.lifetime;
        self.food_energy = applied.food_energy;
        self.flow_enabled = applied.flow_enabled;
        self.flow_intensity = applied.flow_intensity;
        self.flow_preset = applied.flow_preset;
        self.anti_cramming = applied.anti_cramming;
        self.anti_cramming_force = applied.anti_cramming_force;
        self.anti_cramming_distance = applied.anti_cramming_distance;
        self.edited_map = self.applied_map.clone();
        self.map_dirty = false;
        self.map_tool = MapTool::None;
        self.lasso_points.clear();
        self.last_brush_position = None;
        self.editing_field = None;
    }

    fn restore_loaded_run(&mut self, loaded: &LoadedRun) {
        let settings = &loaded.settings;
        self.seed = settings.seed;
        self.seed_input = settings.seed.to_string();
        self.initial_cells = settings.initial_cells;
        self.amadeus_mode = settings.amadeus_mode;
        self.edited_map = loaded.world.map.clone();
        self.map_tool = MapTool::None;
        self.lasso_points.clear();
        self.last_brush_position = None;
        self.map_dirty = false;
        self.initial_cells_input = settings.initial_cells.to_string();
        self.flow_enabled = settings.flow_intensity > 0.0;
        self.flow_intensity = settings.flow_intensity;
        self.flow_preset = settings.flow_preset;
        self.initial_neural_configuration = settings.initial_neural_configuration;
        self.validated_population =
            (!settings.validated_genomes.is_empty()).then(|| ValidatedPopulation {
                source_label: "Pool incluído nesta run".to_owned(),
                source_tick: loaded.world.tick,
                genomes: settings.validated_genomes.clone(),
            });
        self.anti_cramming = settings.anti_cramming.enabled;
        self.anti_cramming_force = settings.anti_cramming.force;
        self.anti_cramming_distance = settings.anti_cramming.exclusion_distance;
        self.mental_energy_cost = settings.mental_energy_cost;
        self.desert_emission = settings.desert_emission;
        self.desert_emission_input = format_emission_display(settings.desert_emission);
        self.fertile_emission = settings.fertile_emission;
        self.fertile_emission_input = format_emission_display(settings.fertile_emission);
        self.lifetime = settings.lifetime as i32;
        self.food_energy = settings.food_energy;
        self.reproduction_energy = settings.reproduction_energy;
        self.reproduction_energy_input = settings.reproduction_energy.to_string();
        self.reproduction_limit = settings.reproduction_limit;
        self.reproduction_limit_input = settings.reproduction_limit.to_string();
        self.run_id = loaded.run_id;
        self.run_events = loaded.events.clone();
        self.editing_field = None;
        self.mark_applied();
    }
}

impl From<&SessionUi> for EngineSettings {
    fn from(session: &SessionUi) -> Self {
        Self {
            seed: session.seed,
            amadeus_mode: session.amadeus_mode,
            map: session.edited_map.clone(),
            flow_intensity: session.effective_flow_intensity(),
            flow_preset: session.flow_preset,
            anti_cramming: session.anti_cramming_config(),
            mental_energy_cost: session.mental_energy_cost,
            desert_emission: session.desert_emission,
            fertile_emission: session.fertile_emission,
            lifetime: session.lifetime as u32,
            food_energy: session.food_energy,
            reproduction_energy: session.reproduction_energy,
            reproduction_limit: session.reproduction_limit,
            initial_cells: session.initial_cells,
            initial_neural_configuration: session.initial_neural_configuration,
            validated_genomes: session
                .validated_population
                .as_ref()
                .map(|population| population.genomes.clone())
                .unwrap_or_default(),
        }
    }
}

impl SimulationEngine {
    fn new(settings: EngineSettings) -> Self {
        let mut initial_world = make_world(&settings);
        initial_world.set_anti_cramming(settings.anti_cramming);
        let initial_world = Arc::new(initial_world);
        let latest = Arc::new(Mutex::new(initial_world));
        let metrics = Arc::new(Mutex::new(EngineMetrics::default()));
        let telemetry = Arc::new(Mutex::new(TelemetryLog::default()));
        let (commands, receiver) = mpsc::channel();
        let worker_latest = Arc::clone(&latest);
        let worker_metrics = Arc::clone(&metrics);
        let worker_telemetry = Arc::clone(&telemetry);
        thread::spawn(move || {
            simulation_worker(
                settings,
                receiver,
                worker_latest,
                worker_metrics,
                worker_telemetry,
            )
        });
        Self {
            commands,
            latest,
            metrics,
            telemetry,
        }
    }

    fn snapshot(&self) -> Arc<World> {
        self.latest
            .lock()
            .map(|snapshot| Arc::clone(&snapshot))
            .unwrap_or_else(|_| Arc::new(World::new(0, 0)))
    }

    fn metrics(&self) -> EngineMetrics {
        self.metrics
            .lock()
            .map(|metrics| *metrics)
            .unwrap_or_default()
    }

    fn telemetry(&self) -> Vec<TelemetrySample> {
        self.telemetry
            .lock()
            .map(|log| log.samples.iter().copied().collect())
            .unwrap_or_default()
    }

    fn send(&self, command: EngineCommand) {
        let _ = self.commands.send(command);
    }
}

fn make_world(settings: &EngineSettings) -> World {
    if settings.amadeus_mode {
        World::new_amadeus_with_initial_population(
            settings.seed,
            settings.map.clone(),
            settings.initial_neural_configuration,
            &settings.validated_genomes,
        )
    } else {
        World::new_with_initial_population(
            settings.seed,
            settings.initial_cells,
            settings.map.clone(),
            settings.initial_neural_configuration,
            &settings.validated_genomes,
        )
    }
}

fn simulation_worker(
    mut settings: EngineSettings,
    commands: mpsc::Receiver<EngineCommand>,
    latest: Arc<Mutex<Arc<World>>>,
    metrics: Arc<Mutex<EngineMetrics>>,
    telemetry: Arc<Mutex<TelemetryLog>>,
) {
    let mut world = make_world(&settings);
    world.set_anti_cramming(settings.anti_cramming);
    let mut flow = FlowField::new(settings.seed)
        .with_preset(settings.flow_preset)
        .with_intensity(settings.flow_intensity);
    let mut paused = false;
    let mut headless = false;
    let mut speed = 0usize;
    let mut ui_fps = 60.0;
    let mut estimated_tps = 1_000.0;
    let mut last_publish = Instant::now();
    let mut last_telemetry_tick = 0;
    if let Ok(mut log) = telemetry.lock() {
        log.reset(&world);
    }

    loop {
        while let Ok(command) = commands.try_recv() {
            match command {
                EngineCommand::Pause(value) => paused = value,
                EngineCommand::Speed(value) => speed = value.min(SPEED_MULTIPLIERS.len() - 1),
                EngineCommand::UiFps(value) => ui_fps = value.max(1.0),
                EngineCommand::SetHeadless(value) => {
                    headless = value;
                    // Ao sair, a primeira fotografia já é publicada para que
                    // a tela volte a refletir o mundo atual sem aguardar o
                    // próximo intervalo de publicação normal.
                    if !headless {
                        publish_snapshot(
                            &world,
                            &latest,
                            &metrics,
                            estimated_tps,
                            target_publish_hz(ui_fps, speed),
                            settings.desert_emission,
                            settings.fertile_emission,
                        );
                    } else if let Ok(mut current) = metrics.lock() {
                        *current = EngineMetrics {
                            ticks_per_second: estimated_tps,
                            publication_hz: 0.0,
                            active_desert_emission: settings.desert_emission,
                            active_fertile_emission: settings.fertile_emission,
                            tick: world.tick,
                            population: world.organisms.len(),
                            headless: true,
                        };
                    }
                    last_publish = Instant::now();
                }
                EngineCommand::SetFlowIntensity(intensity) => {
                    settings.flow_intensity = intensity.clamp(0.0, 1.0);
                    flow = FlowField::new(settings.seed)
                        .with_preset(settings.flow_preset)
                        .with_intensity(settings.flow_intensity);
                }
                EngineCommand::SetFlowPreset(preset) => {
                    settings.flow_preset = preset;
                    flow = FlowField::new(settings.seed)
                        .with_preset(settings.flow_preset)
                        .with_intensity(settings.flow_intensity);
                }
                EngineCommand::SetDesertEmission(emission) => {
                    settings.desert_emission = emission.max(0.0);
                }
                EngineCommand::SetFertileEmission(emission) => {
                    settings.fertile_emission = emission.max(0.0);
                }
                EngineCommand::SetFoodLifetime(lifetime) => {
                    settings.lifetime = lifetime.max(1);
                }
                EngineCommand::SetFoodEnergy(energy) => {
                    settings.food_energy = energy.max(0.0);
                }
                EngineCommand::SetReproductionEnergy(energy) => {
                    settings.reproduction_energy = energy.max(1.0);
                    settings.reproduction_limit = settings
                        .reproduction_limit
                        .max(settings.reproduction_energy);
                }
                EngineCommand::SetReproductionLimit(limit) => {
                    settings.reproduction_limit = limit.max(settings.reproduction_energy);
                }
                EngineCommand::SetMentalEnergyCost(cost) => {
                    settings.mental_energy_cost = cost.max(0.0);
                }
                EngineCommand::SetAntiCramming(config) => {
                    settings.anti_cramming = config;
                    world.set_anti_cramming(config);
                }
                EngineCommand::SetMap(map) => {
                    settings.map = map.clone();
                    world.map = map;
                }
                EngineCommand::Restart(new_settings) => {
                    settings = new_settings;
                    flow = FlowField::new(settings.seed)
                        .with_preset(settings.flow_preset)
                        .with_intensity(settings.flow_intensity);
                    world = make_world(&settings);
                    world.set_anti_cramming(settings.anti_cramming);
                    publish_snapshot(
                        &world,
                        &latest,
                        &metrics,
                        0.0,
                        0.0,
                        settings.desert_emission,
                        settings.fertile_emission,
                    );
                    if let Ok(mut log) = telemetry.lock() {
                        log.reset(&world);
                    }
                    last_telemetry_tick = 0;
                    last_publish = Instant::now();
                }
                EngineCommand::Load {
                    world: loaded_world,
                    settings: loaded_settings,
                    telemetry: loaded_telemetry,
                } => {
                    settings = loaded_settings;
                    settings.map = loaded_world.map.clone();
                    flow = FlowField::new(settings.seed)
                        .with_preset(settings.flow_preset)
                        .with_intensity(settings.flow_intensity);
                    world = loaded_world;
                    publish_snapshot(
                        &world,
                        &latest,
                        &metrics,
                        0.0,
                        0.0,
                        settings.desert_emission,
                        settings.fertile_emission,
                    );
                    if let Ok(mut log) = telemetry.lock() {
                        log.samples = loaded_telemetry.into_iter().collect();
                        if log.samples.is_empty() {
                            log.push(&world);
                        }
                    }
                    last_telemetry_tick = world.tick;
                    last_publish = Instant::now();
                }
            }
        }

        if paused {
            thread::sleep(Duration::from_millis(2));
            continue;
        }

        let speed_multiplier = SPEED_MULTIPLIERS[speed];
        let requested_tps = 10.0 * speed_multiplier;
        let target_publish_hz = target_publish_hz(ui_fps, speed);
        let batch = if headless {
            // O modo headless não obedece ao limitador de velocidade da UI:
            // ele mede o máximo que o cálculo do mundo consegue avançar,
            // mas ainda verifica comandos aproximadamente a cada 8 ms.
            (estimated_tps * 0.008).round().clamp(64.0, 8_192.0) as u32
        } else if requested_tps <= estimated_tps {
            (requested_tps / target_publish_hz).ceil().max(1.0) as u32
        } else {
            // Mantém a latência de pausa e controles abaixo de aproximadamente 8 ms.
            (estimated_tps * 0.008).round().clamp(64.0, 8_192.0) as u32
        };
        let started = Instant::now();
        for _ in 0..batch {
            world.tick(
                &flow,
                settings.desert_emission,
                settings.fertile_emission,
                settings.lifetime,
                settings.food_energy,
                settings.mental_energy_cost,
                settings.reproduction_energy,
                settings.reproduction_limit,
            );
            if world.tick.saturating_sub(last_telemetry_tick) >= TELEMETRY_INTERVAL_TICKS {
                if let Ok(mut log) = telemetry.lock() {
                    log.push(&world);
                }
                last_telemetry_tick = world.tick;
            }
        }
        let elapsed = started.elapsed().as_secs_f32().max(0.000_001);
        let observed_tps = batch as f32 / elapsed;
        estimated_tps = estimated_tps * 0.85 + observed_tps * 0.15;
        if headless && last_publish.elapsed() >= Duration::from_millis(250) {
            // Métricas simples mantêm o modal legível sem clonar `World`.
            if let Ok(mut current) = metrics.lock() {
                *current = EngineMetrics {
                    ticks_per_second: estimated_tps,
                    publication_hz: 0.0,
                    active_desert_emission: settings.desert_emission,
                    active_fertile_emission: settings.fertile_emission,
                    tick: world.tick,
                    population: world.organisms.len(),
                    headless: true,
                };
            }
            last_publish = Instant::now();
        } else if !headless && last_publish.elapsed().as_secs_f32() >= 1.0 / target_publish_hz {
            let snapshot = Arc::new(world.clone());
            if let Ok(mut target) = latest.lock() {
                *target = snapshot;
            }
            if let Ok(mut current) = metrics.lock() {
                *current = EngineMetrics {
                    ticks_per_second: estimated_tps,
                    publication_hz: target_publish_hz,
                    active_desert_emission: settings.desert_emission,
                    active_fertile_emission: settings.fertile_emission,
                    tick: world.tick,
                    population: world.organisms.len(),
                    headless: false,
                };
            }
            last_publish = Instant::now();
        }

        if !headless && requested_tps < observed_tps {
            let desired_seconds = batch as f32 / requested_tps;
            let remaining = desired_seconds - elapsed;
            if remaining > 0.000_5 {
                thread::sleep(Duration::from_secs_f32(remaining));
            }
        }
    }
}

fn publish_snapshot(
    world: &World,
    latest: &Arc<Mutex<Arc<World>>>,
    metrics: &Arc<Mutex<EngineMetrics>>,
    ticks_per_second: f32,
    publication_hz: f32,
    active_desert_emission: f32,
    active_fertile_emission: f32,
) {
    if let Ok(mut target) = latest.lock() {
        *target = Arc::new(world.clone());
    }
    if let Ok(mut current) = metrics.lock() {
        *current = EngineMetrics {
            ticks_per_second,
            publication_hz,
            active_desert_emission,
            active_fertile_emission,
            tick: world.tick,
            population: world.organisms.len(),
            headless: false,
        };
    }
}

fn target_publish_hz(ui_fps: f32, speed: usize) -> f32 {
    let speed_multiplier = SPEED_MULTIPLIERS[speed];
    let speed_penalty = 1.0 + 0.35 * speed_multiplier.log10().max(0.0);
    (ui_fps * 0.9 / speed_penalty).clamp(5.0, 60.0)
}

fn capture_telemetry(world: &World) -> TelemetrySample {
    let population = world.organisms.len();
    let mut max_generation = 0;
    let mut ones_per_bit = [0u32; GENOME_BIT_COUNT];
    for organism in &world.organisms {
        max_generation = max_generation.max(organism.generation);
        for (byte_index, byte) in organism.genome.bytes().iter().enumerate() {
            for bit_in_byte in 0..8 {
                let bit_index = byte_index * 8 + bit_in_byte;
                if bit_index >= GENOME_BIT_COUNT {
                    continue;
                }
                ones_per_bit[bit_index] += ((byte >> (7 - bit_in_byte)) & 1) as u32;
            }
        }
    }
    let genetic_diversity = if population == 0 {
        0.0
    } else {
        ones_per_bit
            .iter()
            .map(|ones| binary_entropy(*ones as f32 / population as f32))
            .sum::<f32>()
            / ones_per_bit.len() as f32
    };
    TelemetrySample {
        tick: world.tick,
        population,
        births_total: world.births_total,
        deaths_total: world.deaths_total,
        births_by_neural_decision_total: world.births_by_neural_decision_total,
        births_by_energy_limit_total: world.births_by_energy_limit_total,
        deaths_by_starvation_total: world.deaths_by_starvation_total,
        deaths_by_phagocytosis_total: world.deaths_by_phagocytosis_total,
        deaths_by_age_total: world.deaths_by_age_total,
        average_energy: world.average_energy(),
        average_age: world.average_age_ticks(),
        average_generation: world.average_generation(),
        max_generation,
        genetic_diversity,
    }
}

fn binary_entropy(probability: f32) -> f32 {
    if probability <= 0.0 || probability >= 1.0 {
        0.0
    } else {
        -probability * probability.log2() - (1.0 - probability) * (1.0 - probability).log2()
    }
}

fn emission_display_value(emission_per_tick: f32) -> f32 {
    (emission_per_tick / EMISSION_DISPLAY_UNIT).max(0.0)
}

fn emission_from_display(value: f32) -> f32 {
    (value * EMISSION_DISPLAY_UNIT).max(0.0)
}

fn format_emission_display(emission_per_tick: f32) -> String {
    let value = emission_display_value(emission_per_tick);
    if (value - value.round()).abs() < 0.000_1 {
        format!("{:.0}", value)
    } else {
        format!("{value:.2}")
    }
}

fn mental_cost_display(cost_per_connection: f32) -> f32 {
    (cost_per_connection.max(0.0) * MENTAL_COST_DISPLAY_SCALE).round()
}

fn mental_cost_from_display(display_value: f32) -> f32 {
    display_value.max(0.0) / MENTAL_COST_DISPLAY_SCALE
}

const RUN_MAGIC: [u8; 8] = *b"ECSRUN01";
/// V17 adiciona os bits hereditários que habilitam nós internos e motores.
const RUN_FORMAT_VERSION: u32 = 17;
/// Unidade humana do controle de custo mental. O motor continua guardando o
/// valor físico pequeno para preservar runs e presets; na interface, 100
/// unidades equivalem ao custo-padrão de 0,0025 por conexão/tick.
const MENTAL_COST_DISPLAY_SCALE: f32 = 40_000.0;

/// Raiz portátil: os dados de uma cópia compartilhada ficam ao lado do .exe,
/// e não dependem da estrutura `target/release` usada no desenvolvimento.
fn application_storage_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn saved_runs_directory() -> PathBuf {
    application_storage_root().join("runs_salvas")
}

fn session_presets_path() -> PathBuf {
    saved_runs_directory()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("presets_sessao.ecopreset")
}

fn map_presets_path() -> PathBuf {
    application_storage_root().join("presets_mapa.ecomap")
}

fn embedded_initial_settings() -> Option<EngineSettings> {
    let mut reader = Cursor::new(PRESET_INICIAL_EMBUTIDO);
    read_session_presets_from_reader(&mut reader)
        .ok()?
        .into_iter()
        .find(|preset| preset.name.eq_ignore_ascii_case(PRESET_INICIAL_NOME))
        .map(|preset| preset.settings)
}

fn embedded_new_gods_map() -> Option<MapDefinition> {
    let mut reader = Cursor::new(NEW_GODS_MAP_EMBUTIDO);
    read_session_presets_from_reader(&mut reader)
        .ok()?
        .into_iter()
        .find(|preset| preset.name.eq_ignore_ascii_case("preset mania"))
        .map(|preset| preset.settings.map)
}

fn built_in_map_presets() -> Vec<MapPreset> {
    vec![
        MapPreset {
            name: "Oásis original".to_owned(),
            map: MapDefinition::default(),
            flow_preset: FlowPreset::Classic,
        },
        MapPreset {
            name: "3.344 · New Gods".to_owned(),
            map: embedded_new_gods_map().unwrap_or_else(MapDefinition::default),
            flow_preset: FlowPreset::Classic,
        },
    ]
}

fn refresh_map_presets(session: &mut SessionUi) {
    let path = map_presets_path();
    let mut presets = match read_map_presets(&path) {
        Ok(presets) => presets,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            session.map_preset_status = Some(format!("Não foi possível ler mapas: {error}"));
            Vec::new()
        }
    };
    // Os dois mapas distribuídos são canônicos: substituí-los aqui repara
    // arquivos de versões anteriores que tinham "3.344" com o disco errado,
    // sem apagar os mapas particulares do usuário.
    for built_in in built_in_map_presets() {
        let is_new_gods = built_in.name.starts_with("3.344");
        if let Some(index) = presets.iter().position(|preset| {
            if is_new_gods {
                preset.name.starts_with("3.344")
            } else {
                preset.name.eq_ignore_ascii_case(&built_in.name)
            }
        }) {
            presets[index] = built_in;
        } else {
            presets.push(built_in);
        }
    }
    session.saved_map_presets = presets;
    if let Err(error) = write_map_presets(&session.saved_map_presets) {
        session.map_preset_status =
            Some(format!("Não foi possível gravar presets de mapa: {error}"));
    }
}

fn write_map_presets(presets: &[MapPreset]) -> io::Result<()> {
    let path = map_presets_path();
    let mut writer = io::BufWriter::new(fs::File::create(path)?);
    writer.write_all(&MAP_PRESET_MAGIC)?;
    write_u32_run(&mut writer, MAP_PRESET_FORMAT_VERSION)?;
    write_u32_run(&mut writer, presets.len().min(MAX_MAP_PRESETS) as u32)?;
    for preset in presets.iter().take(MAX_MAP_PRESETS) {
        write_string(&mut writer, &preset.name)?;
        write_flow_preset(&mut writer, preset.flow_preset)?;
        preset.map.write_to(&mut writer)?;
    }
    writer.flush()
}

fn read_map_presets(path: &Path) -> io::Result<Vec<MapPreset>> {
    let mut reader = io::BufReader::new(fs::File::open(path)?);
    let mut magic = [0; MAP_PRESET_MAGIC.len()];
    reader.read_exact(&mut magic)?;
    let version = read_u32_run(&mut reader)?;
    if magic != MAP_PRESET_MAGIC || version != MAP_PRESET_FORMAT_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "formato de mapa inválido",
        ));
    }
    let count = read_count_run(&mut reader, MAX_MAP_PRESETS)?;
    let mut presets = Vec::with_capacity(count);
    for _ in 0..count {
        presets.push(MapPreset {
            name: read_string(&mut reader, 80)?,
            flow_preset: read_flow_preset(&mut reader)?,
            map: MapDefinition::read_from(&mut reader)?,
        });
    }
    Ok(presets)
}

fn save_or_replace_map_preset(session: &mut SessionUi, name: String) -> io::Result<()> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "dê um nome ao mapa",
        ));
    }
    let preset = MapPreset {
        name: name.clone(),
        map: session.edited_map.clone(),
        flow_preset: session.flow_preset,
    };
    if let Some(index) = session
        .saved_map_presets
        .iter()
        .position(|saved| saved.name.eq_ignore_ascii_case(&name))
    {
        session.saved_map_presets[index] = preset;
    } else {
        if session.saved_map_presets.len() >= MAX_MAP_PRESETS {
            session.saved_map_presets.remove(0);
        }
        session.saved_map_presets.push(preset);
    }
    write_map_presets(&session.saved_map_presets)
}

fn refresh_session_presets(session: &mut SessionUi) {
    let path = session_presets_path();
    session.saved_presets = match read_session_presets(&path) {
        Ok(presets) => presets,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            session.preset_status = Some(format!("Não foi possível ler presets: {error}"));
            Vec::new()
        }
    };
}

fn write_session_presets(presets: &[SessionPreset]) -> io::Result<()> {
    let path = session_presets_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut writer = io::BufWriter::new(fs::File::create(path)?);
    writer.write_all(&PRESET_MAGIC)?;
    write_u32_run(&mut writer, PRESET_FORMAT_VERSION)?;
    write_u32_run(&mut writer, presets.len().min(MAX_SESSION_PRESETS) as u32)?;
    for preset in presets.iter().take(MAX_SESSION_PRESETS) {
        write_string(&mut writer, &preset.name)?;
        write_engine_settings(&mut writer, &preset.settings)?;
        preset.settings.map.write_to(&mut writer)?;
    }
    writer.flush()
}

fn read_session_presets(path: &Path) -> io::Result<Vec<SessionPreset>> {
    let mut reader = io::BufReader::new(fs::File::open(path)?);
    read_session_presets_from_reader(&mut reader)
}

fn read_session_presets_from_reader(mut reader: &mut impl Read) -> io::Result<Vec<SessionPreset>> {
    let mut magic = [0; PRESET_MAGIC.len()];
    reader.read_exact(&mut magic)?;
    let version = read_u32_run(&mut reader)?;
    if magic != PRESET_MAGIC || !(1..=PRESET_FORMAT_VERSION).contains(&version) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "formato de preset incompatível",
        ));
    }
    let count = read_count_run(&mut reader, MAX_SESSION_PRESETS)?;
    let mut presets = Vec::with_capacity(count);
    for _ in 0..count {
        let name = read_string(&mut reader, 80)?;
        let mut settings = read_engine_settings(
            &mut reader,
            if version == 1 { 15 } else { RUN_FORMAT_VERSION },
        )?;
        settings.map = MapDefinition::read_from(&mut reader)?;
        presets.push(SessionPreset { name, settings });
    }
    Ok(presets)
}

fn apply_embedded_default_preset(session: &mut SessionUi) {
    let Some(settings) = embedded_initial_settings() else {
        return;
    };
    apply_session_preset(session, &settings);
    // O preset de sessão preserva os demais parâmetros da run 3.344; a
    // geometria padrão é o mapa New Gods original, em formato de S.
    session.edited_map = embedded_new_gods_map().unwrap_or_else(MapDefinition::default);
    session.map_dirty = false;
    // A configuração herdada define o mundo; a população inicial aprovada é
    // deliberadamente igual em toda cópia nova do executável.
    session.initial_cells = 100;
    session.initial_cells_input = "100".to_owned();
    session.initial_neural_configuration = InitialNeuralConfiguration::ValidatedV2;
    // Amadeus é uma ferramenta de experimento pontual, não o modo de
    // partida da distribuição nem parte do preset inicial.
    session.amadeus_mode = false;
    session.seed = 847_291_530;
    session.seed_input = session.seed.to_string();
    session.reproduction_energy = 120.0;
    session.reproduction_energy_input = "120".to_owned();
    session.reproduction_limit = DEFAULT_REPRODUCTION_LIMIT;
    session.reproduction_limit_input = "5000".to_owned();
    session.mental_energy_cost = 0.0009;
    session.desert_emission = emission_from_display(30.0);
    session.desert_emission_input = "30".to_owned();
    session.fertile_emission = emission_from_display(100.0);
    session.fertile_emission_input = "100".to_owned();
    session.lifetime = 3_511;
    session.food_energy = 85.0;
    session.flow_enabled = true;
    session.flow_intensity = 0.06;
    session.flow_preset = FlowPreset::Classic;
    session.preset_name_input = PRESET_INICIAL_NOME.to_owned();
    session.preset_status = Some("Configuração inicial carregada: preset 3.344.".to_owned());
    session.mark_applied();
}

fn write_string(writer: &mut impl Write, value: &str) -> io::Result<()> {
    let bytes = value.as_bytes();
    let length = bytes.len().min(80);
    write_u32_run(writer, length as u32)?;
    writer.write_all(&bytes[..length])
}

fn read_string(reader: &mut impl Read, maximum: usize) -> io::Result<String> {
    let length = read_count_run(reader, maximum)?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "nome de preset inválido"))
}

fn save_or_replace_session_preset(session: &mut SessionUi, name: String) -> io::Result<()> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "dê um nome ao preset",
        ));
    }
    let preset = SessionPreset {
        name: name.clone(),
        settings: EngineSettings::from(&*session),
    };
    if let Some(index) = session
        .saved_presets
        .iter()
        .position(|saved| saved.name.eq_ignore_ascii_case(&name))
    {
        session.saved_presets[index] = preset;
    } else {
        if session.saved_presets.len() >= MAX_SESSION_PRESETS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "limite de presets atingido",
            ));
        }
        session.saved_presets.push(preset);
    }
    write_session_presets(&session.saved_presets)
}

fn apply_session_preset(session: &mut SessionUi, settings: &EngineSettings) {
    session.seed = settings.seed;
    session.seed_input = settings.seed.to_string();
    session.amadeus_mode = settings.amadeus_mode;
    session.initial_cells = settings.initial_cells;
    session.initial_cells_input = settings.initial_cells.to_string();
    session.initial_neural_configuration = settings.initial_neural_configuration;
    session.flow_enabled = settings.flow_intensity > 0.0;
    session.flow_intensity = settings.flow_intensity;
    session.flow_preset = settings.flow_preset;
    session.anti_cramming = settings.anti_cramming.enabled;
    session.anti_cramming_force = settings.anti_cramming.force;
    session.anti_cramming_distance = settings.anti_cramming.exclusion_distance;
    session.mental_energy_cost = settings.mental_energy_cost;
    session.desert_emission = settings.desert_emission;
    session.desert_emission_input = format_emission_display(settings.desert_emission);
    session.fertile_emission = settings.fertile_emission;
    session.fertile_emission_input = format_emission_display(settings.fertile_emission);
    session.lifetime = settings.lifetime as i32;
    session.food_energy = settings.food_energy;
    session.reproduction_energy = settings.reproduction_energy;
    session.reproduction_energy_input = settings.reproduction_energy.to_string();
    session.reproduction_limit = settings.reproduction_limit;
    session.reproduction_limit_input = settings.reproduction_limit.to_string();
    session.edited_map = settings.map.clone();
    session.map_dirty = true;
    session.map_tool = MapTool::None;
    session.validated_population =
        (!settings.validated_genomes.is_empty()).then(|| ValidatedPopulation {
            source_label: "Pool incluído no preset".to_owned(),
            source_tick: 0,
            genomes: settings.validated_genomes.clone(),
        });
    session.editing_field = None;
}

fn display_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("run desconhecida")
        .to_owned()
}

fn refresh_saved_runs(session: &mut SessionUi) {
    let directory = saved_runs_directory();
    let Ok(entries) = fs::read_dir(directory) else {
        session.saved_runs.clear();
        return;
    };
    let mut runs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("ecosrun") {
            continue;
        }
        if let Ok(info) = read_saved_run_info(&path) {
            runs.push(info);
        }
    }
    runs.sort_by(|left, right| right.tick.cmp(&left.tick));
    session.saved_runs = runs;
}

fn validated_population_from_world(
    world: &World,
    source_label: impl Into<String>,
) -> Option<ValidatedPopulation> {
    let genomes: Vec<Genome> = world
        .organisms
        .iter()
        .map(|organism| organism.genome.clone())
        .collect();
    (!genomes.is_empty()).then(|| ValidatedPopulation {
        source_label: source_label.into(),
        source_tick: world.tick,
        genomes,
    })
}

/// A fonte padrão de Val 1.0 é a run salva mais avançada. O arquivo continua
/// sendo a fonte de verdade; esta função apenas extrai sua população final.
fn refresh_validated_population(session: &mut SessionUi) {
    let Some(saved) = session.saved_runs.first() else {
        session.validated_population = None;
        return;
    };
    session.validated_population = load_run_file(&saved.path).ok().and_then(|loaded| {
        validated_population_from_world(
            &loaded.world,
            format!("run {} · tick {}", saved.run_id, saved.tick),
        )
    });
}

fn save_run_file(
    session: &SessionUi,
    world: &World,
    telemetry: &[TelemetrySample],
) -> io::Result<PathBuf> {
    let directory = saved_runs_directory();
    fs::create_dir_all(&directory)?;
    let path = directory.join(format!("run-{}.ecosrun", session.run_id));
    let mut writer = io::BufWriter::new(fs::File::create(&path)?);
    writer.write_all(&RUN_MAGIC)?;
    write_u32_run(&mut writer, RUN_FORMAT_VERSION)?;
    write_u64_run(&mut writer, session.run_id)?;
    write_u64_run(&mut writer, world.tick)?;
    write_u32_run(&mut writer, world.organisms.len() as u32)?;
    write_engine_settings(&mut writer, &EngineSettings::from(session))?;
    world.write_checkpoint(&mut writer)?;
    write_u32_run(&mut writer, telemetry.len() as u32)?;
    for sample in telemetry {
        write_telemetry_sample(&mut writer, *sample)?;
    }
    write_u32_run(&mut writer, session.run_events.len() as u32)?;
    for event in &session.run_events {
        write_u64_run(&mut writer, event.tick)?;
        write_applied_settings(&mut writer, event.before)?;
        write_applied_settings(&mut writer, event.after)?;
    }
    writer.flush()?;
    Ok(path)
}

fn read_saved_run_info(path: &Path) -> io::Result<SavedRunInfo> {
    let mut reader = io::BufReader::new(fs::File::open(path)?);
    let _version = validate_run_header(&mut reader)?;
    let run_id = read_u64_run(&mut reader)?;
    let tick = read_u64_run(&mut reader)?;
    let population = read_u32_run(&mut reader)? as usize;
    Ok(SavedRunInfo {
        path: path.to_path_buf(),
        run_id,
        tick,
        population,
    })
}

fn load_run_file(path: &Path) -> io::Result<LoadedRun> {
    let mut reader = io::BufReader::new(fs::File::open(path)?);
    let version = validate_run_header(&mut reader)?;
    let run_id = read_u64_run(&mut reader)?;
    let _header_tick = read_u64_run(&mut reader)?;
    let _header_population = read_u32_run(&mut reader)?;
    let settings = read_engine_settings(&mut reader, version)?;
    let world = match version {
        1 => World::read_checkpoint_v1(&mut reader)?,
        2 => World::read_checkpoint_v2(&mut reader)?,
        3 => World::read_checkpoint_v3(&mut reader)?,
        4 | 5 => World::read_checkpoint_v4(&mut reader)?,
        6..=9 => World::read_checkpoint_pre_pheromone(&mut reader)?,
        10 => World::read_checkpoint_pre_donation(&mut reader)?,
        11 => World::read_checkpoint_pre_world_expansion(&mut reader)?,
        12 => World::read_checkpoint_pre_longevity(&mut reader)?,
        13 => World::read_checkpoint_pre_event_counters(&mut reader)?,
        14 => World::read_checkpoint_pre_age_counters(&mut reader)?,
        15 => World::read_checkpoint_pre_food_sensor_range(&mut reader)?,
        16 => World::read_checkpoint_pre_neural_node_activity(&mut reader)?,
        _ => World::read_checkpoint(&mut reader)?,
    };
    let telemetry_count = read_count_run(&mut reader, MAX_TELEMETRY_SAMPLES)?;
    let mut telemetry = Vec::with_capacity(telemetry_count);
    for _ in 0..telemetry_count {
        telemetry.push(read_telemetry_sample(&mut reader, version)?);
    }
    let event_count = read_count_run(&mut reader, 100_000)?;
    let mut events = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        events.push(RunEvent {
            tick: read_u64_run(&mut reader)?,
            before: read_applied_settings(&mut reader, version)?,
            after: read_applied_settings(&mut reader, version)?,
        });
    }
    Ok(LoadedRun {
        run_id,
        settings,
        world,
        telemetry,
        events,
    })
}

fn validate_run_header(reader: &mut impl Read) -> io::Result<u32> {
    let mut magic = [0; RUN_MAGIC.len()];
    reader.read_exact(&mut magic)?;
    let version = read_u32_run(reader)?;
    if magic != RUN_MAGIC || !(1..=RUN_FORMAT_VERSION).contains(&version) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "formato .ecosrun incompatível",
        ));
    }
    Ok(version)
}

fn write_engine_settings(writer: &mut impl Write, settings: &EngineSettings) -> io::Result<()> {
    write_u64_run(writer, settings.seed)?;
    write_bool_run(writer, settings.amadeus_mode)?;
    write_initial_neural_configuration(writer, settings.initial_neural_configuration)?;
    write_flow_preset(writer, settings.flow_preset)?;
    write_f32_run(writer, settings.flow_intensity)?;
    write_bool_run(writer, settings.anti_cramming.enabled)?;
    write_f32_run(writer, settings.anti_cramming.force)?;
    write_f32_run(writer, settings.anti_cramming.exclusion_distance)?;
    write_f32_run(writer, settings.mental_energy_cost)?;
    write_f32_run(writer, settings.desert_emission)?;
    write_f32_run(writer, settings.fertile_emission)?;
    write_u32_run(writer, settings.lifetime)?;
    write_f32_run(writer, settings.food_energy)?;
    write_f32_run(writer, settings.reproduction_energy)?;
    write_f32_run(writer, settings.reproduction_limit)?;
    write_u64_run(writer, settings.initial_cells as u64)?;
    write_validated_genomes(writer, &settings.validated_genomes)
}

fn read_engine_settings(reader: &mut impl Read, version: u32) -> io::Result<EngineSettings> {
    let seed = read_u64_run(reader)?;
    let amadeus_mode = if version >= 5 {
        read_bool_run(reader)?
    } else {
        false
    };
    let initial_neural_configuration = if version >= 8 {
        read_initial_neural_configuration(reader)?
    } else {
        InitialNeuralConfiguration::Classic
    };
    let flow_preset = if version >= 7 {
        read_flow_preset(reader)?
    } else {
        FlowPreset::Classic
    };
    let flow_intensity = read_f32_run(reader)?;
    let anti_cramming = AntiCrammingConfig {
        enabled: read_bool_run(reader)?,
        force: read_f32_run(reader)?,
        exclusion_distance: read_f32_run(reader)?,
    };
    let mental_energy_cost = read_f32_run(reader)?;
    let desert_emission = read_f32_run(reader)?;
    let fertile_emission = if version >= 4 {
        read_f32_run(reader)?
    } else {
        desert_emission * 2.0
    };
    Ok(EngineSettings {
        seed,
        amadeus_mode,
        map: MapDefinition::default(),
        flow_intensity,
        flow_preset,
        anti_cramming,
        mental_energy_cost,
        desert_emission,
        fertile_emission,
        lifetime: read_u32_run(reader)?,
        food_energy: read_f32_run(reader)?,
        reproduction_energy: read_f32_run(reader)?,
        reproduction_limit: read_f32_run(reader)?,
        initial_cells: read_u64_run(reader)? as usize,
        initial_neural_configuration,
        validated_genomes: if version >= 17 {
            read_validated_genomes(reader)?
        } else if version >= 16 {
            read_validated_genomes_pre_neural_node_activity(reader)?
        } else if version >= 10 {
            read_validated_genomes_pre_food_sensor_range(reader)?
        } else if version >= 9 {
            read_validated_genomes_pre_pheromone(reader)?
        } else {
            Vec::new()
        },
    })
}

fn write_applied_settings(writer: &mut impl Write, settings: AppliedSettings) -> io::Result<()> {
    write_u64_run(writer, settings.seed)?;
    write_u64_run(writer, settings.initial_cells as u64)?;
    write_initial_neural_configuration(writer, settings.initial_neural_configuration)?;
    write_f32_run(writer, settings.reproduction_energy)?;
    write_f32_run(writer, settings.reproduction_limit)?;
    write_f32_run(writer, settings.mental_energy_cost)?;
    write_f32_run(writer, settings.desert_emission)?;
    write_f32_run(writer, settings.fertile_emission)?;
    write_u32_run(writer, settings.lifetime as u32)?;
    write_f32_run(writer, settings.food_energy)?;
    write_bool_run(writer, settings.flow_enabled)?;
    write_f32_run(writer, settings.flow_intensity)?;
    write_flow_preset(writer, settings.flow_preset)?;
    write_bool_run(writer, settings.anti_cramming)?;
    write_f32_run(writer, settings.anti_cramming_force)?;
    write_f32_run(writer, settings.anti_cramming_distance)
}

fn read_applied_settings(reader: &mut impl Read, version: u32) -> io::Result<AppliedSettings> {
    Ok(AppliedSettings {
        seed: read_u64_run(reader)?,
        initial_cells: read_u64_run(reader)? as usize,
        initial_neural_configuration: if version >= 8 {
            read_initial_neural_configuration(reader)?
        } else {
            InitialNeuralConfiguration::Classic
        },
        // Eventos antigos não serializavam este modo; tratamos como desligado
        // porque Amadeus nunca é o comportamento padrão de uma run.
        amadeus_mode: false,
        reproduction_energy: read_f32_run(reader)?,
        reproduction_limit: read_f32_run(reader)?,
        mental_energy_cost: read_f32_run(reader)?,
        desert_emission: read_f32_run(reader)?,
        fertile_emission: if version >= 4 {
            read_f32_run(reader)?
        } else {
            0.0
        },
        lifetime: read_u32_run(reader)? as i32,
        food_energy: read_f32_run(reader)?,
        flow_enabled: read_bool_run(reader)?,
        flow_intensity: read_f32_run(reader)?,
        flow_preset: if version >= 7 {
            read_flow_preset(reader)?
        } else {
            FlowPreset::Classic
        },
        anti_cramming: read_bool_run(reader)?,
        anti_cramming_force: read_f32_run(reader)?,
        anti_cramming_distance: read_f32_run(reader)?,
    })
}

fn write_telemetry_sample(writer: &mut impl Write, sample: TelemetrySample) -> io::Result<()> {
    write_u64_run(writer, sample.tick)?;
    write_u64_run(writer, sample.population as u64)?;
    write_u64_run(writer, sample.births_total)?;
    write_u64_run(writer, sample.deaths_total)?;
    write_u64_run(writer, sample.births_by_neural_decision_total)?;
    write_u64_run(writer, sample.births_by_energy_limit_total)?;
    write_u64_run(writer, sample.deaths_by_starvation_total)?;
    write_u64_run(writer, sample.deaths_by_phagocytosis_total)?;
    write_u64_run(writer, sample.deaths_by_age_total)?;
    write_f32_run(writer, sample.average_energy)?;
    write_f32_run(writer, sample.average_age)?;
    write_f32_run(writer, sample.average_generation)?;
    write_u32_run(writer, sample.max_generation)?;
    write_f32_run(writer, sample.genetic_diversity)
}

fn read_telemetry_sample(reader: &mut impl Read, version: u32) -> io::Result<TelemetrySample> {
    Ok(TelemetrySample {
        tick: read_u64_run(reader)?,
        population: read_u64_run(reader)? as usize,
        births_total: read_u64_run(reader)?,
        deaths_total: read_u64_run(reader)?,
        births_by_neural_decision_total: if version >= 14 {
            read_u64_run(reader)?
        } else {
            0
        },
        births_by_energy_limit_total: if version >= 14 {
            read_u64_run(reader)?
        } else {
            0
        },
        deaths_by_starvation_total: if version >= 14 {
            read_u64_run(reader)?
        } else {
            0
        },
        deaths_by_phagocytosis_total: if version >= 14 {
            read_u64_run(reader)?
        } else {
            0
        },
        deaths_by_age_total: if version >= 15 {
            read_u64_run(reader)?
        } else {
            0
        },
        average_energy: read_f32_run(reader)?,
        average_age: read_f32_run(reader)?,
        average_generation: read_f32_run(reader)?,
        max_generation: read_u32_run(reader)?,
        genetic_diversity: read_f32_run(reader)?,
    })
}

fn write_u32_run(writer: &mut impl Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}
fn write_u64_run(writer: &mut impl Write, value: u64) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}
fn write_f32_run(writer: &mut impl Write, value: f32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}
fn write_bool_run(writer: &mut impl Write, value: bool) -> io::Result<()> {
    writer.write_all(&[value as u8])
}

fn write_flow_preset(writer: &mut impl Write, preset: FlowPreset) -> io::Result<()> {
    write_u32_run(
        writer,
        match preset {
            FlowPreset::Oasis => 0,
            FlowPreset::Classic => 1,
            FlowPreset::Left => 2,
        },
    )
}

fn write_initial_neural_configuration(
    writer: &mut impl Write,
    configuration: InitialNeuralConfiguration,
) -> io::Result<()> {
    write_u32_run(
        writer,
        match configuration {
            InitialNeuralConfiguration::Classic => 0,
            InitialNeuralConfiguration::MotorMinimum => 1,
            InitialNeuralConfiguration::ValidatedV1 => 2,
            InitialNeuralConfiguration::ValidatedV2 => 3,
        },
    )
}
fn read_u32_run(reader: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}
fn read_u64_run(reader: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}
fn read_f32_run(reader: &mut impl Read) -> io::Result<f32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(f32::from_le_bytes(bytes))
}
fn read_bool_run(reader: &mut impl Read) -> io::Result<bool> {
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

fn read_flow_preset(reader: &mut impl Read) -> io::Result<FlowPreset> {
    match read_u32_run(reader)? {
        // O código histórico 0 era Oásis; migra para Clássico porque a
        // corrente agora é independente do desenho de terra fértil.
        0 => Ok(FlowPreset::Classic),
        1 => Ok(FlowPreset::Classic),
        2 => Ok(FlowPreset::Left),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "corrente inválida",
        )),
    }
}

fn read_initial_neural_configuration(
    reader: &mut impl Read,
) -> io::Result<InitialNeuralConfiguration> {
    match read_u32_run(reader)? {
        0 => Ok(InitialNeuralConfiguration::Classic),
        1 => Ok(InitialNeuralConfiguration::MotorMinimum),
        2 => Ok(InitialNeuralConfiguration::ValidatedV1),
        3 => Ok(InitialNeuralConfiguration::ValidatedV2),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "configuração neural inicial inválida",
        )),
    }
}

const MAX_VALIDATED_GENOMES: usize = 20_000;

fn write_validated_genomes(writer: &mut impl Write, genomes: &[Genome]) -> io::Result<()> {
    write_u32_run(writer, genomes.len().min(MAX_VALIDATED_GENOMES) as u32)?;
    for genome in genomes.iter().take(MAX_VALIDATED_GENOMES) {
        writer.write_all(genome.bytes())?;
    }
    Ok(())
}

fn read_validated_genomes(reader: &mut impl Read) -> io::Result<Vec<Genome>> {
    let count = read_count_run(reader, MAX_VALIDATED_GENOMES)?;
    let mut genomes = Vec::with_capacity(count);
    for _ in 0..count {
        let mut bits = [0; GENOME_BYTES];
        reader.read_exact(&mut bits)?;
        genomes.push(Genome::from_bytes(bits));
    }
    Ok(genomes)
}

fn read_validated_genomes_pre_neural_node_activity(
    reader: &mut impl Read,
) -> io::Result<Vec<Genome>> {
    let count = read_count_run(reader, MAX_VALIDATED_GENOMES)?;
    let mut genomes = Vec::with_capacity(count);
    for _ in 0..count {
        let mut bits = [0; PRE_NEURAL_NODE_ACTIVITY_GENOME_BYTES];
        reader.read_exact(&mut bits)?;
        genomes.push(Genome::from_pre_neural_node_activity_bytes(bits));
    }
    Ok(genomes)
}

fn read_validated_genomes_pre_food_sensor_range(reader: &mut impl Read) -> io::Result<Vec<Genome>> {
    let count = read_count_run(reader, MAX_VALIDATED_GENOMES)?;
    let mut genomes = Vec::with_capacity(count);
    for _ in 0..count {
        let mut bits = [0; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES];
        reader.read_exact(&mut bits)?;
        genomes.push(Genome::from_pre_food_sensor_range_bytes(bits));
    }
    Ok(genomes)
}

fn read_validated_genomes_pre_pheromone(reader: &mut impl Read) -> io::Result<Vec<Genome>> {
    let count = read_count_run(reader, MAX_VALIDATED_GENOMES)?;
    let mut genomes = Vec::with_capacity(count);
    for _ in 0..count {
        let mut bits = [0; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES];
        reader.read_exact(&mut bits)?;
        genomes.push(Genome::from_pre_pheromone_bytes(bits));
    }
    Ok(genomes)
}
fn read_count_run(reader: &mut impl Read, maximum: usize) -> io::Result<usize> {
    let count = read_u32_run(reader)? as usize;
    if count > maximum {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "arquivo grande demais",
        ))
    } else {
        Ok(count)
    }
}

pub async fn run() {
    let mut session = SessionUi::default();
    // Esta cópia compartilhável começa sempre com o mundo aprovado embutido;
    // presets gravados pelo usuário continuam disponíveis apenas para escolha.
    refresh_session_presets(&mut session);
    apply_embedded_default_preset(&mut session);
    refresh_map_presets(&mut session);
    refresh_saved_runs(&mut session);
    if session.validated_population.is_none() {
        refresh_validated_population(&mut session);
    }
    let engine = SimulationEngine::new(EngineSettings::from(&session));
    let mut sent_paused = session.paused;
    let mut sent_speed = session.speed;
    let mut sent_flow_intensity = session.effective_flow_intensity();
    let mut sent_flow_preset = session.flow_preset;
    let mut sent_desert_emission = session.desert_emission;
    let mut sent_fertile_emission = session.fertile_emission;
    let mut sent_lifetime = session.lifetime;
    let mut sent_food_energy = session.food_energy;
    let mut sent_reproduction_energy = session.reproduction_energy;
    let mut sent_reproduction_limit = session.reproduction_limit;
    let mut sent_mental_energy_cost = session.mental_energy_cost;
    let mut sent_anti_cramming = session.anti_cramming_config();
    let mut apply_pending = false;
    let mut last_ui_feedback = 0.0f64;

    loop {
        if session.headless {
            // Neste ramo não pedimos `engine.snapshot()`: além de não desenhar
            // a arena, evitamos a cópia completa do mundo que alimentaria a UI.
            if session.paused != sent_paused {
                engine.send(EngineCommand::Pause(session.paused));
                sent_paused = session.paused;
            }
            clear_background(BACKGROUND);
            if draw_headless_runtime(&mut session, engine.metrics()) {
                session.headless = false;
                engine.send(EngineCommand::SetHeadless(false));
            }
            next_frame().await;
            continue;
        }

        let world = engine.snapshot();
        refresh_demographics_window(&mut session, &world, || engine.telemetry());
        ensure_selected_cell(&mut session, &world);
        if session.paused != sent_paused {
            engine.send(EngineCommand::Pause(session.paused));
            sent_paused = session.paused;
        }
        if session.speed != sent_speed {
            engine.send(EngineCommand::Speed(session.speed));
            sent_speed = session.speed;
        }
        let effective_flow_intensity = session.effective_flow_intensity();
        if apply_pending {
            if (effective_flow_intensity - sent_flow_intensity).abs() > f32::EPSILON {
                engine.send(EngineCommand::SetFlowIntensity(effective_flow_intensity));
                sent_flow_intensity = effective_flow_intensity;
            }
            if session.flow_preset != sent_flow_preset {
                engine.send(EngineCommand::SetFlowPreset(session.flow_preset));
                sent_flow_preset = session.flow_preset;
            }
            if (session.desert_emission - sent_desert_emission).abs() > f32::EPSILON {
                engine.send(EngineCommand::SetDesertEmission(session.desert_emission));
                sent_desert_emission = session.desert_emission;
            }
            if (session.fertile_emission - sent_fertile_emission).abs() > f32::EPSILON {
                engine.send(EngineCommand::SetFertileEmission(session.fertile_emission));
                sent_fertile_emission = session.fertile_emission;
            }
            if session.lifetime != sent_lifetime {
                engine.send(EngineCommand::SetFoodLifetime(
                    session.lifetime.max(1) as u32
                ));
                sent_lifetime = session.lifetime;
            }
            if (session.food_energy - sent_food_energy).abs() > f32::EPSILON {
                engine.send(EngineCommand::SetFoodEnergy(session.food_energy));
                sent_food_energy = session.food_energy;
            }
            if (session.reproduction_energy - sent_reproduction_energy).abs() > f32::EPSILON {
                engine.send(EngineCommand::SetReproductionEnergy(
                    session.reproduction_energy,
                ));
                sent_reproduction_energy = session.reproduction_energy;
            }
            if (session.reproduction_limit - sent_reproduction_limit).abs() > f32::EPSILON {
                engine.send(EngineCommand::SetReproductionLimit(
                    session.reproduction_limit,
                ));
                sent_reproduction_limit = session.reproduction_limit;
            }
            if (session.mental_energy_cost - sent_mental_energy_cost).abs() > f32::EPSILON {
                engine.send(EngineCommand::SetMentalEnergyCost(
                    session.mental_energy_cost,
                ));
                sent_mental_energy_cost = session.mental_energy_cost;
            }
            let anti_cramming_config = session.anti_cramming_config();
            if anti_cramming_config != sent_anti_cramming {
                engine.send(EngineCommand::SetAntiCramming(anti_cramming_config));
                sent_anti_cramming = anti_cramming_config;
            }
            session.mark_live_settings_applied();
            apply_pending = false;
        }
        if get_time() - last_ui_feedback > 0.5 {
            engine.send(EngineCommand::UiFps(get_fps() as f32));
            last_ui_feedback = get_time();
        }

        clear_background(BACKGROUND);
        let panel_x = (screen_width() - PANEL_WIDTH).max(820.0);
        let arena = arena_rect(panel_x);
        handle_map_editor(&mut session, arena);
        draw_top_bar(panel_x);
        draw_arena(
            arena,
            session.show_flow,
            session.seed,
            effective_flow_intensity,
            session.flow_preset,
            if session.tab == PanelTab::Settings {
                &session.edited_map
            } else {
                &world.map
            },
            session.reproduction_limit,
            &world,
            session.show_sensor_reach,
            session.show_crowding_violations,
            session.show_pheromone_field,
            session.show_fertile_area,
            session.pheromone_display,
            (session.tab == PanelTab::Cells)
                .then_some(session.selected_cell_id)
                .flatten(),
        );
        if session.tab == PanelTab::Settings {
            draw_map_editor_overlay(arena, &session);
        }
        let panel_action = if session.headless_modal_open {
            if draw_headless_activation_modal(&mut session) {
                session.headless = true;
                session.headless_modal_open = false;
                engine.send(EngineCommand::SetHeadless(true));
            }
            PanelAction::default()
        } else {
            let action = draw_panel(panel_x, &mut session, &world, engine.metrics());
            if action.open_headless_modal {
                session.headless_modal_open = true;
            }
            action
        };
        if let Some(name) = panel_action.save_preset {
            commit_all_numeric_inputs(&mut session);
            session.editing_field = None;
            match save_or_replace_session_preset(&mut session, name.clone()) {
                Ok(()) => {
                    session.preset_name_input = name;
                    session.preset_name_editing = false;
                    session.preset_status = Some("Preset salvo.".to_owned());
                }
                Err(error) => session.preset_status = Some(format!("Preset não salvo: {error}")),
            }
        }
        if let Some(index) = panel_action.load_preset {
            if let Some(preset) = session.saved_presets.get(index).cloned() {
                let name = preset.name.clone();
                apply_session_preset(&mut session, &preset.settings);
                session.preset_name_input = name.clone();
                session.preset_status = Some(format!(
                    "Preset '{name}' carregado. Clique em Aplicar ou Reiniciar sessão."
                ));
            }
        }
        if let Some(index) = panel_action.delete_preset {
            if index < session.saved_presets.len() {
                let name = session.saved_presets.remove(index).name;
                match write_session_presets(&session.saved_presets) {
                    Ok(()) => session.preset_status = Some(format!("Preset '{name}' apagado.")),
                    Err(error) => {
                        session.preset_status =
                            Some(format!("Não foi possível persistir a exclusão: {error}"))
                    }
                }
            }
        }
        if let Some(name) = panel_action.save_map_preset {
            match save_or_replace_map_preset(&mut session, name.clone()) {
                Ok(()) => {
                    session.map_preset_name_input = name;
                    session.map_preset_name_editing = false;
                    session.map_preset_status = Some("Preset de mapa salvo.".to_owned());
                }
                Err(error) => {
                    session.map_preset_status = Some(format!("Mapa não salvo: {error}"));
                }
            }
        }
        if let Some(index) = panel_action.load_map_preset {
            if let Some(preset) = session.saved_map_presets.get(index).cloned() {
                session.edited_map = preset.map.clone();
                session.flow_preset = preset.flow_preset;
                // Carregar um preset de mapa deve surtir efeito por si só.
                // Antes, o modal permanecia aberto e era fácil clicar em
                // "Aplicar mapa" no lugar errado, parecendo que nada mudou.
                engine.send(EngineCommand::SetMap(preset.map));
                engine.send(EngineCommand::SetFlowPreset(preset.flow_preset));
                session.map_dirty = false;
                session.applied_map = session.edited_map.clone();
                session.applied.flow_preset = session.flow_preset;
                session.map_preset_name_input = preset.name.clone();
                session.map_preset_modal_open = false;
                session.map_preset_status =
                    Some(format!("Mapa '{}' carregado e aplicado.", preset.name));
            }
        }
        if let Some(index) = panel_action.delete_map_preset {
            if index < session.saved_map_presets.len() {
                let name = session.saved_map_presets.remove(index).name;
                match write_map_presets(&session.saved_map_presets) {
                    Ok(()) => session.map_preset_status = Some(format!("Mapa '{name}' apagado.")),
                    Err(error) => {
                        session.map_preset_status =
                            Some(format!("Não foi possível apagar mapa: {error}"))
                    }
                }
            }
        }
        if panel_action.apply {
            let before = session.applied;
            commit_all_numeric_inputs(&mut session);
            session.editing_field = None;
            let after = session.current_settings();
            if before != after {
                session.run_events.push(RunEvent {
                    tick: world.tick,
                    before,
                    after,
                });
            }
            apply_pending = true;
            if session.map_dirty {
                engine.send(EngineCommand::SetMap(session.edited_map.clone()));
                session.map_dirty = false;
            }
        }
        if panel_action.revert {
            session.revert_pending_changes();
            session.preset_status = Some("Alterações pendentes revertidas.".to_owned());
        }
        if panel_action.restart {
            commit_all_numeric_inputs(&mut session);
            session.editing_field = None;
            engine.send(EngineCommand::Restart(EngineSettings::from(&session)));
            sent_flow_intensity = session.effective_flow_intensity();
            sent_flow_preset = session.flow_preset;
            sent_desert_emission = session.desert_emission;
            sent_fertile_emission = session.fertile_emission;
            sent_lifetime = session.lifetime;
            sent_food_energy = session.food_energy;
            sent_reproduction_energy = session.reproduction_energy;
            sent_reproduction_limit = session.reproduction_limit;
            sent_mental_energy_cost = session.mental_energy_cost;
            sent_anti_cramming = session.anti_cramming_config();
            session.mark_applied();
            session.run_id = session.seed ^ get_time().to_bits();
            session.run_events.clear();
            session.selected_cell_id = None;
            session.genetics_report = None;
        }
        if panel_action.open_demographics {
            open_demographics_window(&mut session, &world, &engine.telemetry());
        }
        if panel_action.save_run {
            if session.has_pending_live_changes()
                || session.seed_is_dirty()
                || session.initial_cells_are_dirty()
                || session.initial_neural_configuration_is_dirty()
                || session.map_dirty
            {
                session.save_status =
                    Some("Aplique ou reinicie as alterações pendentes antes de salvar.".to_owned());
            } else {
                match save_run_file(&session, &world, &engine.telemetry()) {
                    Ok(path) => {
                        session.save_status = Some(format!("Salvo: {}", display_file_name(&path)));
                        session.validated_population = validated_population_from_world(
                            &world,
                            format!("run {} · tick {}", session.run_id, world.tick),
                        );
                        refresh_saved_runs(&mut session);
                    }
                    Err(error) => session.save_status = Some(format!("Falha ao salvar: {error}")),
                }
            }
        }
        if panel_action.refresh_saved_runs {
            refresh_saved_runs(&mut session);
            refresh_validated_population(&mut session);
            session.save_status = Some(format!("{} run(s) encontradas.", session.saved_runs.len()));
        }
        if let Some(path) = panel_action.load_path {
            match load_run_file(&path) {
                Ok(loaded) => {
                    let validated_from_loaded = validated_population_from_world(
                        &loaded.world,
                        format!("run {} · tick {}", loaded.run_id, loaded.world.tick),
                    );
                    session.restore_loaded_run(&loaded);
                    session.validated_population = validated_from_loaded;
                    engine.send(EngineCommand::Load {
                        world: loaded.world,
                        settings: loaded.settings,
                        telemetry: loaded.telemetry,
                    });
                    sent_flow_intensity = session.effective_flow_intensity();
                    sent_flow_preset = session.flow_preset;
                    sent_desert_emission = session.desert_emission;
                    sent_fertile_emission = session.fertile_emission;
                    sent_lifetime = session.lifetime;
                    sent_food_energy = session.food_energy;
                    sent_reproduction_energy = session.reproduction_energy;
                    sent_reproduction_limit = session.reproduction_limit;
                    sent_mental_energy_cost = session.mental_energy_cost;
                    sent_anti_cramming = session.anti_cramming_config();
                    session.paused = true;
                    session.save_status = Some(format!("Carregada: {}", display_file_name(&path)));
                }
                Err(error) => session.save_status = Some(format!("Falha ao carregar: {error}")),
            }
        }
        next_frame().await;
    }
}

pub async fn run_demographics_window(data_path: PathBuf) {
    let mut data = None;
    let mut selected_window = DemographicWindow::Last30k;
    loop {
        if let Some(updated) = read_demographics_data(&data_path) {
            data = Some(updated);
        }
        clear_background(BACKGROUND);
        if let Some(data) = data.as_ref() {
            draw_demographics_dashboard(
                Rect::new(24.0, 24.0, screen_width() - 48.0, screen_height() - 48.0),
                &data.telemetry,
                data,
                &mut selected_window,
            );
        } else {
            draw_text(
                "Aguardando dados da simulação principal...",
                32.0,
                54.0,
                22.0,
                TEXT,
            );
        }
        next_frame().await;
    }
}

fn refresh_demographics_window(
    session: &mut SessionUi,
    world: &World,
    telemetry: impl FnOnce() -> Vec<TelemetrySample>,
) {
    let mut data_path = None;
    let mut closed_path = None;
    if let Some(handle) = session.demographics_window.as_mut() {
        match handle.child.try_wait() {
            Ok(None) if handle.last_written.elapsed() >= Duration::from_millis(500) => {
                data_path = Some(handle.data_path.clone());
            }
            Ok(None) => {}
            Ok(Some(_)) | Err(_) => closed_path = Some(handle.data_path.clone()),
        }
    }
    if let Some(path) = closed_path {
        let _ = fs::remove_file(path);
        session.demographics_window = None;
        return;
    }
    if let Some(path) = data_path {
        let data = demographics_data(world, telemetry());
        if write_demographics_data(&path, &data).is_ok() {
            if let Some(handle) = session.demographics_window.as_mut() {
                handle.last_written = Instant::now();
            }
        }
    }
}

fn open_demographics_window(session: &mut SessionUi, world: &World, telemetry: &[TelemetrySample]) {
    if session.demographics_window.is_some() {
        return;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let data_path = std::env::temp_dir().join(format!(
        "ecossistema_demografia_{}_{}.dat",
        process::id(),
        stamp
    ));
    let data = demographics_data(world, telemetry.to_vec());
    if write_demographics_data(&data_path, &data).is_err() {
        return;
    }
    let Ok(executable) = std::env::current_exe() else {
        let _ = fs::remove_file(data_path);
        return;
    };
    match Command::new(executable)
        .arg("--demographics-window")
        .arg(&data_path)
        .spawn()
    {
        Ok(child) => {
            session.demographics_window = Some(DemographicsWindowHandle {
                child,
                data_path,
                last_written: Instant::now(),
            });
        }
        Err(_) => {
            let _ = fs::remove_file(data_path);
        }
    }
}

fn demographics_data(world: &World, telemetry: Vec<TelemetrySample>) -> DemographicsWindowData {
    let (energy_distribution, energy_bucket_width) =
        world.energy_distribution(ENERGY_HISTOGRAM_BUCKETS);
    DemographicsWindowData {
        tick: world.tick,
        population: world.organisms.len(),
        births_total: world.births_total,
        deaths_total: world.deaths_total,
        births_by_neural_decision_total: world.births_by_neural_decision_total,
        births_by_energy_limit_total: world.births_by_energy_limit_total,
        deaths_by_starvation_total: world.deaths_by_starvation_total,
        deaths_by_phagocytosis_total: world.deaths_by_phagocytosis_total,
        deaths_by_age_total: world.deaths_by_age_total,
        energy_distribution,
        energy_bucket_width,
        telemetry,
    }
}

fn write_demographics_data(path: &Path, data: &DemographicsWindowData) -> std::io::Result<()> {
    let mut encoded = String::from("ECOSYSTEM_DEMOGRAPHICS_V5\n");
    encoded.push_str(&format!(
        "S|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
        data.tick,
        data.population,
        data.births_total,
        data.deaths_total,
        data.births_by_neural_decision_total,
        data.births_by_energy_limit_total,
        data.deaths_by_starvation_total,
        data.deaths_by_phagocytosis_total,
        data.deaths_by_age_total,
    ));
    encoded.push_str(&format!("E|{}|", data.energy_bucket_width));
    for (index, count) in data.energy_distribution.iter().enumerate() {
        if index > 0 {
            encoded.push(';');
        }
        encoded.push_str(&count.to_string());
    }
    encoded.push('\n');
    for sample in &data.telemetry {
        encoded.push_str(&format!(
            "T|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
            sample.tick,
            sample.population,
            sample.births_total,
            sample.deaths_total,
            sample.births_by_neural_decision_total,
            sample.births_by_energy_limit_total,
            sample.deaths_by_starvation_total,
            sample.deaths_by_phagocytosis_total,
            sample.deaths_by_age_total,
            sample.average_energy,
            sample.average_age,
            sample.average_generation,
            sample.max_generation,
            sample.genetic_diversity,
        ));
    }
    fs::write(path, encoded)
}

fn read_demographics_data(path: &Path) -> Option<DemographicsWindowData> {
    let encoded = fs::read_to_string(path).ok()?;
    let mut lines = encoded.lines();
    let version = match lines.next()? {
        "ECOSYSTEM_DEMOGRAPHICS_V3" => 3,
        "ECOSYSTEM_DEMOGRAPHICS_V4" => 4,
        "ECOSYSTEM_DEMOGRAPHICS_V5" => 5,
        _ => return None,
    };
    let stats: Vec<&str> = lines.next()?.split('|').collect();
    if stats.first() != Some(&"S") || stats.len() < 5 {
        return None;
    }
    let stat = |index: usize| stats.get(index).copied().unwrap_or("0");
    let tick = stat(1);
    let population = stat(2);
    let births = stat(3);
    let deaths = stat(4);
    let births_by_neural = if version >= 4 { stat(5) } else { "0" };
    let births_by_limit = if version >= 4 { stat(6) } else { "0" };
    let deaths_by_starvation = if version >= 4 { stat(7) } else { "0" };
    let deaths_by_phagocytosis = if version >= 4 { stat(8) } else { "0" };
    let deaths_by_age = if version >= 5 { stat(9) } else { "0" };
    let energy_line = lines.next()?;
    let energy_parts: Vec<&str> = energy_line.split('|').collect();
    let ["E", energy_bucket_width, encoded_distribution] = energy_parts.as_slice() else {
        return None;
    };
    let energy_distribution = encoded_distribution
        .split(';')
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.parse().ok())
        .collect::<Option<Vec<_>>>()?;
    let telemetry = lines
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.first() != Some(&"T") || parts.len() < 10 {
                return None;
            }
            let value = |index: usize| parts.get(index).copied().unwrap_or("0");
            let base = if version >= 5 {
                10
            } else if version >= 4 {
                9
            } else {
                5
            };
            let tick = value(1);
            let population = value(2);
            let births = value(3);
            let deaths = value(4);
            let births_by_neural = if version >= 4 { value(5) } else { "0" };
            let births_by_limit = if version >= 4 { value(6) } else { "0" };
            let deaths_by_starvation = if version >= 4 { value(7) } else { "0" };
            let deaths_by_phagocytosis = if version >= 4 { value(8) } else { "0" };
            let deaths_by_age = if version >= 5 { value(9) } else { "0" };
            let energy = value(base);
            let age = value(base + 1);
            let generation = value(base + 2);
            let max_generation = value(base + 3);
            let diversity = value(base + 4);
            Some(TelemetrySample {
                tick: tick.parse().ok()?,
                population: population.parse().ok()?,
                births_total: births.parse().ok()?,
                deaths_total: deaths.parse().ok()?,
                births_by_neural_decision_total: births_by_neural.parse().ok()?,
                births_by_energy_limit_total: births_by_limit.parse().ok()?,
                deaths_by_starvation_total: deaths_by_starvation.parse().ok()?,
                deaths_by_phagocytosis_total: deaths_by_phagocytosis.parse().ok()?,
                deaths_by_age_total: deaths_by_age.parse().ok()?,
                average_energy: energy.parse().ok()?,
                average_age: age.parse().ok()?,
                average_generation: generation.parse().ok()?,
                max_generation: max_generation.parse().ok()?,
                genetic_diversity: diversity.parse().ok()?,
            })
        })
        .collect();
    Some(DemographicsWindowData {
        tick: tick.parse().ok()?,
        population: population.parse().ok()?,
        births_total: births.parse().ok()?,
        deaths_total: deaths.parse().ok()?,
        births_by_neural_decision_total: births_by_neural.parse().ok()?,
        births_by_energy_limit_total: births_by_limit.parse().ok()?,
        deaths_by_starvation_total: deaths_by_starvation.parse().ok()?,
        deaths_by_phagocytosis_total: deaths_by_phagocytosis.parse().ok()?,
        deaths_by_age_total: deaths_by_age.parse().ok()?,
        energy_distribution,
        energy_bucket_width: energy_bucket_width.parse().ok()?,
        telemetry,
    })
}

fn arena_rect(panel_x: f32) -> Rect {
    let available_width = panel_x - 64.0;
    let available_height = screen_height() - 132.0;
    let width = available_width.min(available_height * 16.0 / 9.0);
    let height = width * 9.0 / 16.0;
    Rect::new(
        32.0,
        92.0 + (available_height - height).max(0.0) / 2.0,
        width,
        height,
    )
}

fn draw_top_bar(panel_x: f32) {
    draw_text("Ecossistema neural", 32.0, 40.0, 28.0, TEXT);
    draw_text(
        "Neuroevolução em tempo real · células, alimento e corrente",
        32.0,
        66.0,
        16.0,
        MUTED,
    );
    draw_line(
        panel_x - 20.0,
        20.0,
        panel_x - 20.0,
        screen_height() - 20.0,
        1.0,
        BORDER,
    );
}

fn draw_arena(
    arena: Rect,
    show_flow: bool,
    seed: u64,
    flow_intensity: f32,
    flow_preset: FlowPreset,
    map: &MapDefinition,
    energy_capacity: f32,
    world: &World,
    show_sensor_reach: bool,
    show_crowding_violations: bool,
    show_pheromone_field: bool,
    show_fertile_area: bool,
    pheromone_display: PheromoneDisplay,
    selected_cell_id: Option<u64>,
) {
    draw_rectangle(
        arena.x,
        arena.y,
        arena.w,
        arena.h,
        Color::new(0.92, 0.965, 0.925, 1.0),
    );
    draw_rectangle_lines(arena.x, arena.y, arena.w, arena.h, 2.0, BORDER);
    draw_text(
        &format!(
            "Câmara experimental · {} × {} unidades",
            WORLD_WIDTH, WORLD_HEIGHT
        ),
        arena.x + 18.0,
        arena.y + 30.0,
        17.0,
        MUTED,
    );
    if show_pheromone_field {
        draw_pheromone_field(arena, world, pheromone_display);
    }
    draw_map_geometry(arena, map, show_fertile_area);
    if show_flow {
        draw_flow_field(
            arena,
            &FlowField::new(seed)
                .with_preset(flow_preset)
                .with_intensity(flow_intensity),
            map,
        );
    }
    draw_organisms(
        arena,
        &world.organisms,
        selected_cell_id,
        energy_capacity,
        show_sensor_reach,
        show_crowding_violations,
    );
    draw_food(arena, &world.food);
}

fn draw_map_geometry(arena: Rect, map: &MapDefinition, show_fertile_area: bool) {
    let cell_w = arena.w / MAP_GRID_WIDTH as f32;
    let cell_h = arena.h / MAP_GRID_HEIGHT as f32;
    if show_fertile_area {
        for (x, y) in map.fertile_cells() {
            draw_rectangle(
                arena.x + x as f32 * cell_w,
                arena.y + y as f32 * cell_h,
                cell_w + 0.4,
                cell_h + 0.4,
                Color::new(0.95, 0.76, 0.12, 0.20),
            );
        }
    }
    for (x, y) in map.wall_cells() {
        draw_rectangle(
            arena.x + x as f32 * cell_w,
            arena.y + y as f32 * cell_h,
            cell_w + 0.4,
            cell_h + 0.4,
            Color::new(0.055, 0.08, 0.065, 0.94),
        );
    }
}

/// O overlay consulta exatamente o mesmo campo discreto que alimenta os
/// sensores. Fora do tick de emissão, o campo é reconstruído vazio.
fn draw_pheromone_field(arena: Rect, world: &World, display: PheromoneDisplay) {
    let maximum = world.pheromones.display_max(display);
    if maximum <= f32::EPSILON {
        draw_text(
            "mapa químico: sem emissão neste tick",
            arena.x + 18.0,
            arena.y + 54.0,
            13.0,
            MUTED,
        );
        return;
    }
    let cell_w = arena.w / PHEROMONE_FIELD_WIDTH as f32;
    let cell_h = arena.h / PHEROMONE_FIELD_HEIGHT as f32;
    for y in 0..PHEROMONE_FIELD_HEIGHT {
        for x in 0..PHEROMONE_FIELD_WIDTH {
            let ratio = (world.pheromones.display_value(display, x, y) / maximum).clamp(0.0, 1.0);
            if ratio < 0.025 {
                continue;
            }
            // Azul frio para pouco sinal e amarelo/vermelho para as regiões
            // de sobreposição. A intensidade visual é relativa ao pico atual.
            let color = Color::new(
                0.10 + 0.88 * ratio,
                0.22 + 0.58 * ratio,
                0.84 - 0.66 * ratio,
                0.08 + 0.42 * ratio,
            );
            draw_rectangle(
                arena.x + x as f32 * cell_w,
                arena.y + y as f32 * cell_h,
                cell_w + 0.5,
                cell_h + 0.5,
                color,
            );
        }
    }
    draw_text(
        &format!("{} · pico {:.2} ppm", display.label(), maximum),
        arena.x + 18.0,
        arena.y + 54.0,
        13.0,
        Color::new(0.08, 0.20, 0.25, 0.95),
    );
}

fn draw_organisms(
    arena: Rect,
    organisms: &[Organism],
    selected_cell_id: Option<u64>,
    energy_capacity: f32,
    show_sensor_reach: bool,
    show_crowding_violations: bool,
) {
    for organism in organisms {
        let center = world_to_screen(arena, organism.position);
        let radius = organism.physical_radius() / WORLD_WIDTH * arena.w;
        let energy_ratio = (organism.energy / energy_capacity).clamp(0.0, 1.0);
        let [red, green, blue] = organism.genome.genetic_rgb();
        let genetic_color = Color::from_rgba(red, green, blue, 255);
        if show_sensor_reach {
            draw_sensor_reach(arena, center, organism.food_sensor_range());
        }
        draw_circle(
            center.x,
            center.y,
            radius + 2.0,
            Color::new(0.08, 0.22, 0.33, 0.24),
        );
        draw_circle(center.x, center.y, radius, genetic_color);
        draw_circle_lines(
            center.x,
            center.y,
            radius,
            1.5,
            Color::new(0.04, 0.14, 0.24, 1.0),
        );
        if show_crowding_violations && organism.crowding_violation {
            draw_circle_lines(center.x, center.y, radius + 3.0, 2.0, RED);
        }
        draw_circle(
            center.x - radius * 0.22,
            center.y - radius * 0.22,
            radius * (0.16 + energy_ratio * 0.18),
            Color::new(
                0.55 + genetic_color.r * 0.45,
                0.55 + genetic_color.g * 0.45,
                0.55 + genetic_color.b * 0.45,
                0.95,
            ),
        );
        if Some(organism.id) == selected_cell_id {
            let tip = vec2(center.x, center.y - radius - 5.0);
            let left = vec2(tip.x - 8.0, tip.y - 15.0);
            let right = vec2(tip.x + 8.0, tip.y - 15.0);
            draw_triangle(left, right, tip, RED);
            draw_line(
                left.x,
                left.y,
                right.x,
                right.y,
                1.0,
                Color::new(0.42, 0.02, 0.04, 1.0),
            );
            draw_line(
                right.x,
                right.y,
                tip.x,
                tip.y,
                1.0,
                Color::new(0.42, 0.02, 0.04, 1.0),
            );
            draw_line(
                tip.x,
                tip.y,
                left.x,
                left.y,
                1.0,
                Color::new(0.42, 0.02, 0.04, 1.0),
            );
        }
    }
}

/// Mostra os quatro cones exclusivos de 90° usados pelos sensores.
fn draw_sensor_reach(arena: Rect, center: Vec2, sensor_range: f32) {
    let radius = sensor_range / WORLD_WIDTH * arena.w;
    let colors = [
        Color::new(0.30, 0.40, 0.96, 0.11),
        Color::new(0.25, 0.72, 0.94, 0.11),
        Color::new(0.55, 0.42, 0.95, 0.11),
        Color::new(0.22, 0.67, 0.78, 0.11),
    ];
    for (index, (start, end)) in [
        (
            std::f32::consts::FRAC_PI_4 * 3.0,
            std::f32::consts::FRAC_PI_4 * 5.0,
        ),
        (-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4),
        (
            -std::f32::consts::FRAC_PI_4 * 3.0,
            -std::f32::consts::FRAC_PI_4,
        ),
        (
            std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4 * 3.0,
        ),
    ]
    .iter()
    .enumerate()
    {
        draw_sensor_cone(center, radius, *start, *end, colors[index]);
    }
    let border = Color::new(0.24, 0.36, 0.80, 0.34);
    draw_circle_lines(center.x, center.y, radius, 0.7, border);
    draw_line(
        center.x - radius,
        center.y,
        center.x + radius,
        center.y,
        0.7,
        border,
    );
    draw_line(
        center.x,
        center.y - radius,
        center.x,
        center.y + radius,
        0.7,
        border,
    );
}

fn draw_sensor_cone(center: Vec2, radius: f32, start: f32, end: f32, color: Color) {
    const SEGMENTS: usize = 8;
    let mut previous = center + vec2(start.cos(), start.sin()) * radius;
    for step in 1..=SEGMENTS {
        let angle = start + (end - start) * step as f32 / SEGMENTS as f32;
        let current = center + vec2(angle.cos(), angle.sin()) * radius;
        draw_triangle(center, previous, current, color);
        previous = current;
    }
}

fn draw_food(arena: Rect, food: &[Phytoplankton]) {
    for particle in food {
        let center = world_to_screen(arena, particle.position);
        let radius = FOOD_RADIUS / WORLD_WIDTH * arena.w;
        draw_circle(
            center.x,
            center.y,
            radius + 2.5,
            Color::new(0.12, 0.85, 0.61, 0.12),
        );
        draw_circle(
            center.x,
            center.y,
            radius,
            Color::new(0.23, 0.93, 0.67, 1.0),
        );
        draw_circle(
            center.x - radius * 0.3,
            center.y - radius * 0.3,
            radius * 0.25,
            Color::new(0.84, 1.0, 0.91, 0.9),
        );
    }
}

fn draw_flow_field(arena: Rect, flow: &FlowField, map: &MapDefinition) {
    for row in 0..16 {
        for column in 0..28 {
            let world = vec2(
                WORLD_WIDTH * (column as f32 + 0.5) / 28.0,
                WORLD_HEIGHT * (row as f32 + 0.5) / 16.0,
            );
            let velocity = flow.sample_with_map(world, Some(map));
            if velocity.length_squared() <= 0.000_1 {
                continue;
            }
            let screen = world_to_screen(arena, world);
            // O tamanho é deliberadamente fixo: esta camada comunica direção
            // de forma legível; intensidade pertence ao controle lateral.
            draw_flow_triangle(
                screen,
                velocity.normalize_or_zero(),
                Color::new(0.10, 0.40, 0.77, 0.72),
            );
        }
    }
    draw_text(
        "direção da corrente · tamanho visual fixo",
        arena.x + 22.0,
        arena.y + arena.h - 20.0,
        14.0,
        Color::new(0.20, 0.47, 0.70, 0.86),
    );
}

fn draw_flow_triangle(center: Vec2, direction: Vec2, color: Color) {
    let tip = center + direction * 15.0;
    let base = center - direction * 10.0;
    let perpendicular = vec2(-direction.y, direction.x) * 8.0;
    draw_triangle(tip, base + perpendicular, base - perpendicular, color);
}

fn world_to_screen(arena: Rect, position: Vec2) -> Vec2 {
    vec2(
        arena.x + position.x / WORLD_WIDTH * arena.w,
        arena.y + position.y / WORLD_HEIGHT * arena.h,
    )
}

fn screen_to_world(arena: Rect, position: Vec2) -> Vec2 {
    vec2(
        ((position.x - arena.x) / arena.w).clamp(0.0, 0.999_9) * WORLD_WIDTH,
        ((position.y - arena.y) / arena.h).clamp(0.0, 0.999_9) * WORLD_HEIGHT,
    )
}

fn handle_map_editor(session: &mut SessionUi, arena: Rect) {
    if session.tab != PanelTab::Settings || session.map_tool == MapTool::None {
        return;
    }
    let (mouse_x, mouse_y) = mouse_position();
    let mouse = vec2(mouse_x, mouse_y);
    if !arena.contains(mouse) {
        if is_mouse_button_released(MouseButton::Left) {
            session.last_brush_position = None;
        }
        return;
    }
    let world = screen_to_world(arena, mouse);
    match session.map_tool {
        MapTool::Wall if is_mouse_button_down(MouseButton::Left) => {
            paint_map_stroke(
                &mut session.edited_map,
                session.last_brush_position,
                world,
                true,
                session.wall_brush_radius,
            );
            session.last_brush_position = Some(world);
            session.map_dirty = true;
        }
        MapTool::Eraser if is_mouse_button_down(MouseButton::Left) => {
            paint_map_stroke(
                &mut session.edited_map,
                session.last_brush_position,
                world,
                false,
                session.wall_brush_radius,
            );
            session.last_brush_position = Some(world);
            session.map_dirty = true;
        }
        MapTool::FertileLasso => {
            if is_mouse_button_pressed(MouseButton::Left) {
                session.lasso_points.clear();
                session.lasso_points.push(world);
            } else if is_mouse_button_down(MouseButton::Left)
                && session
                    .lasso_points
                    .last()
                    .is_none_or(|previous| previous.distance(world) >= 18.0)
            {
                session.lasso_points.push(world);
            }
            if is_mouse_button_released(MouseButton::Left) {
                session
                    .edited_map
                    .set_fertile_polygon(&session.lasso_points);
                session.lasso_points.clear();
                session.map_tool = MapTool::None;
                session.map_dirty = true;
            }
        }
        MapTool::None => {}
        _ => {}
    }
    if is_mouse_button_released(MouseButton::Left) {
        session.last_brush_position = None;
    }
}

fn paint_map_stroke(
    map: &mut MapDefinition,
    previous: Option<Vec2>,
    current: Vec2,
    enabled: bool,
    brush_radius: u8,
) {
    let start = previous.unwrap_or(current);
    let distance = start.distance(current);
    let logical_pixel = map.logical_pixel_size().x.min(map.logical_pixel_size().y);
    let steps = (distance / (logical_pixel * 0.5)).ceil().max(1.0) as usize;
    for step in 0..=steps {
        map.paint_wall_brush(
            start.lerp(current, step as f32 / steps as f32),
            enabled,
            brush_radius,
        );
    }
}

fn draw_map_editor_overlay(arena: Rect, session: &SessionUi) {
    let (mouse_x, mouse_y) = mouse_position();
    let mouse = vec2(mouse_x, mouse_y);
    if session.map_tool == MapTool::Wall && arena.contains(mouse) {
        let pixel = session.edited_map.logical_pixel_size();
        let radius = session.wall_brush_radius as f32 * pixel.x / WORLD_WIDTH * arena.w;
        draw_circle_lines(
            mouse.x,
            mouse.y,
            radius.max(2.0),
            1.2,
            Color::new(0.05, 0.05, 0.05, 0.9),
        );
    } else if session.map_tool == MapTool::Eraser && arena.contains(mouse) {
        let pixel = session.edited_map.logical_pixel_size();
        let radius = session.wall_brush_radius as f32 * pixel.x / WORLD_WIDTH * arena.w;
        draw_circle_lines(
            mouse.x,
            mouse.y,
            radius.max(2.0),
            1.2,
            Color::new(0.92, 0.22, 0.25, 0.95),
        );
    }
    if session.lasso_points.len() >= 2 {
        for pair in session.lasso_points.windows(2) {
            let start = world_to_screen(arena, pair[0]);
            let end = world_to_screen(arena, pair[1]);
            draw_line(
                start.x,
                start.y,
                end.x,
                end.y,
                3.0,
                Color::new(0.18, 0.92, 0.38, 0.96),
            );
        }
        let start = world_to_screen(arena, session.lasso_points[0]);
        let end = world_to_screen(arena, *session.lasso_points.last().unwrap());
        draw_line(
            start.x,
            start.y,
            end.x,
            end.y,
            1.5,
            Color::new(0.18, 0.92, 0.38, 0.62),
        );
    }
}

fn arrow(origin: Vec2, delta: Vec2, color: Color) {
    let tip = origin + delta;
    draw_line(origin.x, origin.y, tip.x, tip.y, 1.5, color);
    let direction = delta.normalize_or_zero();
    let left = tip - direction.rotate(vec2(0.0, 1.0)) * 5.0 - direction * 6.0;
    let right = tip + direction.rotate(vec2(0.0, 1.0)) * 5.0 - direction * 6.0;
    draw_triangle(tip, left, right, color);
}

/// Tela mínima do modo headless. Ela é propositalmente pequena: não pede
/// snapshots, não desenha mapa, organismos, alimento ou painéis detalhados.
fn draw_headless_runtime(session: &mut SessionUi, metrics: EngineMetrics) -> bool {
    let width = 560.0;
    let height = 340.0;
    let x = (screen_width() - width) * 0.5;
    let y = (screen_height() - height) * 0.5;
    draw_rectangle(x - 2.0, y - 2.0, width + 4.0, height + 4.0, BACKGROUND);
    draw_rectangle(x, y, width, height, SURFACE);
    draw_rectangle_lines(x, y, width, height, 1.0, ACCENT);
    draw_text("Execução headless", x + 22.0, y + 42.0, 27.0, TEXT);
    draw_text(
        if metrics.headless {
            "Cálculo ativo sem renderização nem cópias do mundo para a tela."
        } else {
            "Ativando o worker headless..."
        },
        x + 22.0,
        y + 68.0,
        13.0,
        MUTED,
    );
    draw_text(
        &format!("tick {}", metrics.tick),
        x + 22.0,
        y + 122.0,
        28.0,
        TEXT,
    );
    draw_text(
        &format!("{:.0} ticks/s", metrics.ticks_per_second),
        x + 22.0,
        y + 158.0,
        28.0,
        ACCENT,
    );
    draw_text(
        &format!("{} células", metrics.population),
        x + 22.0,
        y + 188.0,
        17.0,
        MUTED,
    );
    draw_text(
        "O limite de velocidade da interface está suspenso para mostrar o teto de cálculo.",
        x + 22.0,
        y + 222.0,
        12.0,
        MUTED,
    );
    if button(
        Rect::new(x + 22.0, y + height - 60.0, 130.0, 36.0),
        if session.paused { "Retomar" } else { "Pausar" },
        !session.paused,
    ) {
        session.paused = !session.paused;
    }
    button(
        Rect::new(x + width - 208.0, y + height - 60.0, 186.0, 36.0),
        "Voltar à visualização",
        false,
    ) || is_key_pressed(KeyCode::Escape)
}

/// Confirma a troca de modo antes que a tela da arena seja interrompida.
fn draw_headless_activation_modal(session: &mut SessionUi) -> bool {
    let width = 560.0;
    let height = 310.0;
    let x = (screen_width() - width) * 0.5;
    let y = (screen_height() - height) * 0.5;
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.58),
    );
    draw_rectangle(x - 2.0, y - 2.0, width + 4.0, height + 4.0, BACKGROUND);
    draw_rectangle(x, y, width, height, SURFACE);
    draw_rectangle_lines(x, y, width, height, 1.0, ACCENT);
    draw_text("Ativar execução headless?", x + 22.0, y + 42.0, 25.0, TEXT);
    draw_text(
        "A simulação continuará, mas o executável deixa de desenhar a arena",
        x + 22.0,
        y + 79.0,
        14.0,
        MUTED,
    );
    draw_text(
        "e de copiar o World para a interface. Você verá tick, população e ticks/s.",
        x + 22.0,
        y + 102.0,
        14.0,
        MUTED,
    );
    draw_text(
        "Use este modo para comparar o teto de cálculo; o estado biológico continua o mesmo.",
        x + 22.0,
        y + 141.0,
        13.0,
        MUTED,
    );
    if button(
        Rect::new(x + 22.0, y + height - 60.0, 218.0, 36.0),
        "Ativar modo headless",
        true,
    ) {
        return true;
    }
    if button(
        Rect::new(x + width - 130.0, y + height - 60.0, 108.0, 36.0),
        "Cancelar",
        false,
    ) || is_key_pressed(KeyCode::Escape)
    {
        session.headless_modal_open = false;
    }
    false
}

fn draw_panel(
    x: f32,
    session: &mut SessionUi,
    world: &World,
    metrics: EngineMetrics,
) -> PanelAction {
    let left = x + 12.0;
    let width = PANEL_WIDTH - 24.0;
    let tab_width = width / 4.0;
    if button(
        Rect::new(left, 8.0, width, 26.0),
        if session.tab == PanelTab::Settings {
            "Voltar às abas"
        } else {
            "Configurações"
        },
        session.tab == PanelTab::Settings,
    ) {
        session.tab = if session.tab == PanelTab::Settings {
            PanelTab::Session
        } else {
            PanelTab::Settings
        };
        session.editing_field = None;
    }
    if button(
        Rect::new(left, 42.0, tab_width - 3.0, 26.0),
        "Sessão",
        session.tab == PanelTab::Session,
    ) {
        session.tab = PanelTab::Session;
        session.editing_field = None;
    }
    if button(
        Rect::new(left + tab_width + 3.0, 42.0, tab_width - 3.0, 26.0),
        "Células",
        session.tab == PanelTab::Cells,
    ) {
        session.tab = PanelTab::Cells;
        session.editing_field = None;
    }
    if button(
        Rect::new(left + 2.0 * tab_width + 3.0, 42.0, tab_width - 3.0, 26.0),
        "Demografia",
        session.demographics_window.is_some(),
    ) {
        session.editing_field = None;
        return PanelAction {
            open_demographics: true,
            ..Default::default()
        };
    }
    if button(
        Rect::new(left + 3.0 * tab_width + 3.0, 42.0, tab_width - 3.0, 26.0),
        "Filtros",
        session.tab == PanelTab::Visibility,
    ) {
        session.tab = PanelTab::Visibility;
        session.editing_field = None;
    }
    if session.tab == PanelTab::Session {
        let (mouse_x, mouse_y) = mouse_position();
        if mouse_x >= x && mouse_y >= 68.0 && mouse_y <= screen_height() - 12.0 {
            let (_, wheel_y) = mouse_wheel();
            let viewport_bottom = screen_height() - 12.0;
            let maximum_scroll = (SESSION_CONTENT_BOTTOM - viewport_bottom).max(0.0);
            session.session_scroll =
                (session.session_scroll - wheel_y * 32.0).clamp(0.0, maximum_scroll);
        }
    }
    match session.tab {
        PanelTab::Session => draw_session_tab_compact(left, width, session, world, metrics),
        PanelTab::Cells => {
            draw_cells_tab(left, width, session, world);
            PanelAction::default()
        }
        PanelTab::Settings => draw_settings_tab(left, width, session, world),
        PanelTab::Visibility => {
            draw_visibility_tab(left, width, session);
            PanelAction::default()
        }
    }
}

fn draw_visibility_tab(left: f32, width: f32, session: &mut SessionUi) {
    // Um menu aberto consome integralmente o clique. Sem este bloqueio, o
    // mesmo `mouse_pressed` chegava ao controle logo abaixo do popup.
    if session.pheromone_menu_open && is_mouse_button_pressed(MouseButton::Left) {
        let options: Vec<PheromoneDisplay> = std::iter::once(PheromoneDisplay::Pg)
            .chain((0..8).map(PheromoneDisplay::Generic))
            .chain(std::iter::once(PheromoneDisplay::All))
            .collect();
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let menu = Rect::new(
            left - 2.0,
            420.0,
            width + 4.0,
            options.len() as f32 * 28.0 + 4.0,
        );
        if menu.contains(mouse) {
            let index = ((mouse.y - 422.0) / 28.0).floor() as usize;
            if let Some(option) = options.get(index) {
                session.pheromone_display = *option;
                session.show_pheromone_field = true;
            }
        }
        session.pheromone_menu_open = false;
        return;
    }
    draw_text("Filtros de visibilidade", left, 98.0, 21.0, TEXT);
    draw_text(
        "Apenas a visualização muda; a engine continua igual.",
        left,
        120.0,
        12.0,
        MUTED,
    );
    section_title(left, 154.0, "Mundo");
    if checkbox_row(
        Rect::new(left, 168.0, width, 34.0),
        "Mostrar vetores da corrente",
        session.show_flow,
    ) {
        session.show_flow = !session.show_flow;
    }
    if checkbox_row(
        Rect::new(left, 210.0, width, 34.0),
        "Mostrar cones dos sensores",
        session.show_sensor_reach,
    ) {
        session.show_sensor_reach = !session.show_sensor_reach;
    }
    if checkbox_row(
        Rect::new(left, 252.0, width, 34.0),
        "Mostrar conflitos de densidade",
        session.show_crowding_violations,
    ) {
        session.show_crowding_violations = !session.show_crowding_violations;
    }
    section_title(left, 324.0, "Mapa químico instantâneo");
    if checkbox_row(
        Rect::new(left, 338.0, width, 34.0),
        "Mostrar mapa de feromônio",
        session.show_pheromone_field,
    ) {
        session.show_pheromone_field = !session.show_pheromone_field;
    }
    if button(
        Rect::new(left, 382.0, width, 34.0),
        &format!("{}  ▾", session.pheromone_display.label()),
        session.pheromone_menu_open,
    ) {
        session.pheromone_menu_open = !session.pheromone_menu_open;
    }
    if session.pheromone_menu_open {
        let options: Vec<PheromoneDisplay> = std::iter::once(PheromoneDisplay::Pg)
            .chain((0..8).map(PheromoneDisplay::Generic))
            .chain(std::iter::once(PheromoneDisplay::All))
            .collect();
        draw_rectangle(
            left - 2.0,
            420.0,
            width + 4.0,
            options.len() as f32 * 28.0 + 4.0,
            BACKGROUND,
        );
        draw_rectangle_lines(
            left - 2.0,
            420.0,
            width + 4.0,
            options.len() as f32 * 28.0 + 4.0,
            1.0,
            BORDER,
        );
        for (index, option) in options.into_iter().enumerate() {
            if button(
                Rect::new(left, 422.0 + index as f32 * 28.0, width, 26.0),
                &option.label(),
                option == session.pheromone_display,
            ) {
                session.pheromone_display = option;
                session.pheromone_menu_open = false;
                session.show_pheromone_field = true;
            }
        }
    }
    draw_text(
        "Temperatura e umidade entrarão aqui depois.",
        left,
        724.0,
        12.0,
        MUTED,
    );
}

#[allow(dead_code)]
fn draw_session_tab(
    left: f32,
    width: f32,
    session: &mut SessionUi,
    world: &World,
    metrics: EngineMetrics,
) -> PanelAction {
    // Copiamos o deslocamento antes de desenhar: os controles abaixo podem
    // mutar `session` durante o mesmo frame.
    let scroll = session.session_scroll;
    let y = |value: f32| value - scroll;
    if session.pointer_captured_until_release {
        if is_mouse_button_down(MouseButton::Left) {
            return PanelAction::default();
        }
        session.pointer_captured_until_release = false;
    }
    // O popup de velocidade consome o clique inteiro, inclusive os frames em
    // que o botão ainda está pressionado. Isso impede que um slider atrás do
    // menu seja arrastado pela mesma interação.
    if session.speed_menu_open && is_mouse_button_pressed(MouseButton::Left) {
        let menu_x = left + 88.0;
        let menu_y = y(407.0);
        let menu_width = width - 88.0;
        let option_height = 30.0;
        let menu = Rect::new(
            menu_x - 2.0,
            menu_y - 2.0,
            menu_width + 4.0,
            SPEED_MULTIPLIERS.len() as f32 * option_height + 4.0,
        );
        let mouse = vec2(mouse_position().0, mouse_position().1);
        if menu.contains(mouse) {
            let index = ((mouse.y - menu_y) / option_height).floor() as usize;
            if index < SPEED_MULTIPLIERS.len() {
                session.speed = index;
            }
        }
        session.speed_menu_open = false;
        session.pointer_captured_until_release = true;
        return PanelAction::default();
    }
    draw_text("Sessão", left, y(88.0), 22.0, TEXT);
    draw_text(
        &format!(
            "tick {} · {} células · {:.0} ticks/s · pub. {:.0} Hz",
            world.tick,
            world.organisms.len(),
            metrics.ticks_per_second,
            metrics.publication_hz,
        ),
        left,
        y(109.0),
        14.0,
        MUTED,
    );
    let mut action = PanelAction::default();

    section_title(left, y(132.0), "Condições iniciais");
    let seed_rect = Rect::new(left, y(146.0), width - 82.0, 38.0);
    if editable_number_row(
        seed_rect,
        "Seed",
        &session.seed_input,
        session.editing_field == Some(EditingField::Seed),
        session.seed_is_dirty(),
    ) {
        begin_editing(session, EditingField::Seed);
    }
    if button(
        Rect::new(left + width - 74.0, y(146.0), 74.0, 38.0),
        "Nova",
        false,
    ) {
        session.seed = session
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(get_time().to_bits());
        session.seed_input = session.seed.to_string();
        session.editing_field = None;
    }
    if editable_number_row(
        Rect::new(left, y(191.0), width, 38.0),
        "Células iniciais",
        &session.initial_cells_input,
        session.editing_field == Some(EditingField::InitialCells),
        session.initial_cells_are_dirty(),
    ) {
        begin_editing(session, EditingField::InitialCells);
    }
    draw_text("Configuração neural", left, y(244.0), 13.0, MUTED);
    let neural_button_width = (width - 12.0) / 4.0;
    if button_dirty(
        Rect::new(left, y(251.0), neural_button_width, 32.0),
        "Clássica",
        session.initial_neural_configuration == InitialNeuralConfiguration::Classic,
        session.initial_neural_configuration_is_dirty(),
    ) {
        session.initial_neural_configuration = InitialNeuralConfiguration::Classic;
    }
    if button_dirty(
        Rect::new(
            left + neural_button_width + 4.0,
            y(251.0),
            neural_button_width,
            32.0,
        ),
        "Motor",
        session.initial_neural_configuration == InitialNeuralConfiguration::MotorMinimum,
        session.initial_neural_configuration_is_dirty(),
    ) {
        session.initial_neural_configuration = InitialNeuralConfiguration::MotorMinimum;
    }
    let has_validated_population = session
        .validated_population
        .as_ref()
        .is_some_and(|population| !population.genomes.is_empty());
    if button_dirty(
        Rect::new(
            left + (neural_button_width + 4.0) * 2.0,
            y(251.0),
            neural_button_width,
            32.0,
        ),
        "Val 1.0",
        session.initial_neural_configuration == InitialNeuralConfiguration::ValidatedV1,
        session.initial_neural_configuration_is_dirty() && has_validated_population,
    ) && has_validated_population
    {
        session.initial_neural_configuration = InitialNeuralConfiguration::ValidatedV1;
    }
    if button_dirty(
        Rect::new(
            left + (neural_button_width + 4.0) * 3.0,
            y(251.0),
            neural_button_width,
            32.0,
        ),
        "Val 2.0",
        session.initial_neural_configuration == InitialNeuralConfiguration::ValidatedV2,
        session.initial_neural_configuration_is_dirty() && has_validated_population,
    ) && has_validated_population
    {
        session.initial_neural_configuration = InitialNeuralConfiguration::ValidatedV2;
    }
    let neural_description = match session.initial_neural_configuration {
        InitialNeuralConfiguration::Classic => "Genomas iniciais totalmente aleatórios.".to_owned(),
        InitialNeuralConfiguration::MotorMinimum => {
            "4 caminhos diretos alimento → motor; resto aleatório.".to_owned()
        }
        InitialNeuralConfiguration::ValidatedV1 => session
            .validated_population
            .as_ref()
            .map(|population| {
                format!(
                    "{} · tick {} · {} genomas fundadores.",
                    population.source_label,
                    population.source_tick,
                    population.genomes.len()
                )
            })
            .unwrap_or_else(|| "Salve ou carregue uma run para ativar Val 1.0.".to_owned()),
        InitialNeuralConfiguration::ValidatedV2 => session
            .validated_population
            .as_ref()
            .map(|population| {
                format!(
                    "{} · portão Ct + PG → Doar · {} genomas.",
                    population.source_label,
                    population.genomes.len()
                )
            })
            .unwrap_or_else(|| "Salve ou carregue uma run para ativar Val 2.0.".to_owned()),
    };
    draw_text(&neural_description, left, y(298.0), 11.0, MUTED);
    update_numeric_input(session);

    section_title(left, y(319.0), "Execução");
    let state_label = if session.paused { "Iniciar" } else { "Pausar" };
    if button(
        Rect::new(left, y(333.0), 72.0, 32.0),
        state_label,
        !session.paused,
    ) {
        session.paused = !session.paused;
    }
    if button_dirty(
        Rect::new(left + 76.0, y(333.0), 64.0, 32.0),
        "Aplicar",
        false,
        session.has_pending_live_changes(),
    ) {
        action.apply = true;
    }
    if button_enabled(
        Rect::new(left + 144.0, y(333.0), 72.0, 32.0),
        "Reverter",
        session.has_pending_changes(),
    ) {
        action.revert = true;
    }
    if button_dirty(
        Rect::new(left + 220.0, y(333.0), width - 220.0, 32.0),
        "Reiniciar",
        false,
        session.seed_is_dirty()
            || session.initial_cells_are_dirty()
            || session.initial_neural_configuration_is_dirty()
            || session.amadeus_mode != session.applied.amadeus_mode,
    ) {
        action.restart = true;
    }
    draw_text("Velocidade", left, y(393.0), 15.0, MUTED);
    if button(
        Rect::new(left + 88.0, y(372.0), width - 88.0, 32.0),
        speed_label(session.speed),
        session.speed_menu_open,
    ) {
        session.speed_menu_open = !session.speed_menu_open;
        session.pointer_captured_until_release = true;
        return action;
    }

    section_title(left, y(457.0), "Células");
    if editable_number_row(
        Rect::new(left, y(471.0), width, 38.0),
        "Mínimo reprodução",
        &session.reproduction_energy_input,
        session.editing_field == Some(EditingField::ReproductionEnergy),
        session.reproduction_energy_is_dirty(),
    ) {
        begin_editing(session, EditingField::ReproductionEnergy);
    }
    if editable_number_row(
        Rect::new(left, y(516.0), width, 38.0),
        "Limite obrigatório",
        &session.reproduction_limit_input,
        session.editing_field == Some(EditingField::ReproductionLimit),
        session.reproduction_limit_is_dirty(),
    ) {
        begin_editing(session, EditingField::ReproductionLimit);
    }
    draw_text(
        "Cones e mapa químico: aba Filtros.",
        left,
        y(582.0),
        12.0,
        MUTED,
    );
    if let Some(cost) = slider_row(
        left,
        y(606.0),
        width,
        "Custo mental",
        mental_cost_display(session.mental_energy_cost),
        0.0,
        mental_cost_display(0.02),
        &format!(
            "{:.0} unidades / conexão/tick",
            mental_cost_display(session.mental_energy_cost)
        ),
        session.mental_energy_cost != session.applied.mental_energy_cost,
    ) {
        session.mental_energy_cost = mental_cost_from_display(cost);
    }

    section_title(left, y(661.0), "Fitoplâncton");
    let food_status = format!(
        "deserto {} · fértil {} · {} no mapa",
        format_emission_display(metrics.active_desert_emission),
        format_emission_display(metrics.active_fertile_emission),
        world.food.len()
    );
    let food_status_width = measure_text(&food_status, None, 11, 1.0).width;
    draw_text(
        &food_status,
        left + width - food_status_width,
        y(661.0),
        11.0,
        MUTED,
    );
    if editable_number_row(
        Rect::new(left, y(675.0), width, 38.0),
        "Emissão deserto",
        &session.desert_emission_input,
        session.editing_field == Some(EditingField::DesertEmission),
        session.desert_emission_is_dirty(),
    ) {
        begin_editing(session, EditingField::DesertEmission);
    }
    if editable_number_row(
        Rect::new(left, y(720.0), width, 38.0),
        "Emissão terra fértil",
        &session.fertile_emission_input,
        session.editing_field == Some(EditingField::FertileEmission),
        session.fertile_emission_is_dirty(),
    ) {
        begin_editing(session, EditingField::FertileEmission);
    }
    if let Some(lifetime) = slider_row(
        left,
        y(765.0),
        width,
        "Vida fito",
        session.lifetime as f32,
        100.0,
        20_000.0,
        &format!("{} ticks", session.lifetime),
        session.lifetime != session.applied.lifetime,
    ) {
        session.lifetime = lifetime.round() as i32;
    }
    match stepper_row(
        left,
        y(810.0),
        width,
        "Energia",
        &format!("{:.0}", session.food_energy),
        None,
        session.food_energy != session.applied.food_energy,
    ) {
        Some(-1) => session.food_energy = (session.food_energy - 5.0).max(0.0),
        Some(1) => session.food_energy += 5.0,
        _ => {}
    }

    section_title(left, y(860.0), "Corrente");
    if let Some(intensity) = slider_row(
        left,
        y(874.0),
        width,
        "Intensidade",
        session.flow_intensity,
        0.0,
        1.0,
        &format!("{:.0}%", session.flow_intensity * 100.0),
        session.flow_intensity != session.applied.flow_intensity,
    ) {
        session.flow_intensity = intensity;
    }
    if button_dirty(
        Rect::new(left, y(914.0), width, 34.0),
        if session.flow_enabled {
            "Corrente ligada"
        } else {
            "Corrente desligada"
        },
        session.flow_enabled,
        session.flow_enabled != session.applied.flow_enabled,
    ) {
        session.flow_enabled = !session.flow_enabled;
    }
    draw_text("Vetores: aba Filtros.", left, y(976.0), 12.0, MUTED);

    let density_title = if !session.anti_cramming {
        "Densidade · desligada".to_owned()
    } else if let Some(minimum) = world.crowding.minimum_violation_distance {
        format!(
            "Densidade · {} pares · mín {:.0}u",
            world.crowding.violating_pairs, minimum
        )
    } else {
        "Densidade · livre".to_owned()
    };
    section_title(left, y(1_000.0), &density_title);
    if button_dirty(
        Rect::new(left, y(1_014.0), width * 0.54 - 4.0, 34.0),
        if session.anti_cramming {
            "Anti-cramming: on"
        } else {
            "Anti-cramming: off"
        },
        session.anti_cramming,
        session.anti_cramming != session.applied.anti_cramming,
    ) {
        session.anti_cramming = !session.anti_cramming;
    }
    if button(
        Rect::new(
            left + width * 0.54 + 4.0,
            y(1_014.0),
            width * 0.46 - 4.0,
            34.0,
        ),
        "Ver conflitos",
        session.show_crowding_violations,
    ) {
        session.show_crowding_violations = !session.show_crowding_violations;
    }
    if let Some(force) = slider_row(
        left,
        y(1_009.0),
        width,
        "Força",
        session.anti_cramming_force,
        0.0,
        1.0,
        &format!("{:.0}%", session.anti_cramming_force * 100.0),
        session.anti_cramming_force != session.applied.anti_cramming_force,
    ) {
        session.anti_cramming_force = force;
    }
    if let Some(distance) = slider_row(
        left,
        y(1_054.0),
        width,
        "Espaço",
        session.anti_cramming_distance,
        MIN_EXCLUSION_DISTANCE,
        MAX_EXCLUSION_DISTANCE,
        &format!("{:.1}R", session.anti_cramming_distance / CELL_RADIUS),
        session.anti_cramming_distance != session.applied.anti_cramming_distance,
    ) {
        session.anti_cramming_distance = distance;
    }
    if session.speed_menu_open {
        let menu_x = left + 88.0;
        let menu_y = y(407.0);
        let menu_width = width - 88.0;
        let option_height = 30.0;
        draw_rectangle(
            menu_x - 2.0,
            menu_y - 2.0,
            menu_width + 4.0,
            SPEED_MULTIPLIERS.len() as f32 * option_height + 4.0,
            BACKGROUND,
        );
        draw_rectangle_lines(
            menu_x - 2.0,
            menu_y - 2.0,
            menu_width + 4.0,
            SPEED_MULTIPLIERS.len() as f32 * option_height + 4.0,
            1.0,
            BORDER,
        );
        for index in 0..SPEED_MULTIPLIERS.len() {
            if button(
                Rect::new(
                    menu_x,
                    menu_y + index as f32 * option_height,
                    menu_width,
                    option_height,
                ),
                speed_label(index),
                session.speed == index,
            ) {
                session.speed = index;
                session.speed_menu_open = false;
            }
        }
    }
    action
}

fn collapsible_session_section(left: f32, y: f32, width: f32, label: &str, open: &mut bool) {
    let marker = if *open { "v" } else { ">" };
    if button(
        Rect::new(left, y, width, 32.0),
        &format!("{marker}  {label}"),
        *open,
    ) {
        *open = !*open;
    }
}

fn update_preset_name_input(session: &mut SessionUi) {
    if !session.preset_name_editing {
        return;
    }
    while let Some(character) = get_char_pressed() {
        if !character.is_control() && session.preset_name_input.len() < 80 {
            session.preset_name_input.push(character);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        session.preset_name_input.pop();
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Escape) {
        session.preset_name_editing = false;
    }
}

fn draw_session_preset_modal(session: &mut SessionUi, action: &mut PanelAction) {
    if !session.preset_modal_open {
        return;
    }
    update_preset_name_input(session);
    let width = 510.0;
    let height = 420.0;
    let x = (screen_width() - width) * 0.5;
    let y = (screen_height() - height) * 0.5;
    draw_rectangle(x - 2.0, y - 2.0, width + 4.0, height + 4.0, BACKGROUND);
    draw_rectangle(x, y, width, height, SURFACE);
    draw_rectangle_lines(x, y, width, height, 1.0, BORDER);
    draw_text("Presets de sessão", x + 18.0, y + 34.0, 23.0, TEXT);
    draw_text(
        "Salva parâmetros e mapa; não salva o estado vivo da run.",
        x + 18.0,
        y + 56.0,
        12.0,
        MUTED,
    );

    let name_rect = Rect::new(x + 18.0, y + 76.0, width - 150.0, 34.0);
    draw_rectangle(
        name_rect.x,
        name_rect.y,
        name_rect.w,
        name_rect.h,
        if session.preset_name_editing {
            SURFACE_HOVER
        } else {
            BACKGROUND
        },
    );
    draw_rectangle_lines(
        name_rect.x,
        name_rect.y,
        name_rect.w,
        name_rect.h,
        1.0,
        if session.preset_name_editing {
            ACCENT
        } else {
            BORDER
        },
    );
    draw_text(
        &session.preset_name_input,
        name_rect.x + 10.0,
        name_rect.y + 22.0,
        14.0,
        TEXT,
    );
    if name_rect.contains(vec2(mouse_position().0, mouse_position().1))
        && is_mouse_button_pressed(MouseButton::Left)
    {
        session.preset_name_editing = true;
        session.editing_field = None;
    }
    if button(
        Rect::new(x + width - 124.0, y + 76.0, 106.0, 34.0),
        "Salvar / atualizar",
        false,
    ) {
        action.save_preset = Some(session.preset_name_input.clone());
    }
    if let Some(status) = &session.preset_status {
        draw_text(status, x + 18.0, y + 129.0, 12.0, MUTED);
    }

    let list_y = y + 148.0;
    draw_text("Presets salvos", x + 18.0, list_y, 15.0, TEXT);
    if session.saved_presets.is_empty() {
        draw_text(
            "Ainda não há presets.",
            x + 18.0,
            list_y + 29.0,
            14.0,
            MUTED,
        );
    } else {
        let max_rows = 5usize;
        for (index, preset) in session.saved_presets.iter().take(max_rows).enumerate() {
            let row_y = list_y + 12.0 + index as f32 * 42.0;
            draw_value_row(x + 18.0, row_y, width - 178.0, &preset.name, "");
            if button(
                Rect::new(x + width - 152.0, row_y, 82.0, 34.0),
                "Carregar",
                false,
            ) {
                action.load_preset = Some(index);
            }
            if button(
                Rect::new(x + width - 64.0, row_y, 46.0, 34.0),
                "Apagar",
                false,
            ) {
                action.delete_preset = Some(index);
            }
        }
        if session.saved_presets.len() > max_rows {
            draw_text(
                &format!(
                    "+ {} outros presets",
                    session.saved_presets.len() - max_rows
                ),
                x + 18.0,
                y + height - 55.0,
                12.0,
                MUTED,
            );
        }
    }
    if button(
        Rect::new(x + width - 104.0, y + height - 42.0, 86.0, 28.0),
        "Fechar",
        false,
    ) {
        session.preset_modal_open = false;
        session.preset_name_editing = false;
    }
}

fn update_map_preset_name_input(session: &mut SessionUi) {
    if !session.map_preset_name_editing {
        return;
    }
    while let Some(character) = get_char_pressed() {
        if !character.is_control() && session.map_preset_name_input.len() < 80 {
            session.map_preset_name_input.push(character);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        session.map_preset_name_input.pop();
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Escape) {
        session.map_preset_name_editing = false;
    }
}

fn draw_map_preset_modal(session: &mut SessionUi, action: &mut PanelAction) {
    if !session.map_preset_modal_open {
        return;
    }
    update_map_preset_name_input(session);
    let width = 510.0;
    let height = 420.0;
    let x = (screen_width() - width) * 0.5;
    let y = (screen_height() - height) * 0.5;
    draw_rectangle(x - 2.0, y - 2.0, width + 4.0, height + 4.0, BACKGROUND);
    draw_rectangle(x, y, width, height, SURFACE);
    draw_rectangle_lines(x, y, width, height, 1.0, BORDER);
    draw_text("Presets de mapa", x + 18.0, y + 34.0, 23.0, TEXT);
    draw_text(
        "Guarda paredes, área fértil e correnteza.",
        x + 18.0,
        y + 56.0,
        12.0,
        MUTED,
    );
    let name_rect = Rect::new(x + 18.0, y + 76.0, width - 150.0, 34.0);
    draw_rectangle(
        name_rect.x,
        name_rect.y,
        name_rect.w,
        name_rect.h,
        if session.map_preset_name_editing {
            SURFACE_HOVER
        } else {
            BACKGROUND
        },
    );
    draw_rectangle_lines(
        name_rect.x,
        name_rect.y,
        name_rect.w,
        name_rect.h,
        1.0,
        if session.map_preset_name_editing {
            ACCENT
        } else {
            BORDER
        },
    );
    draw_text(
        &session.map_preset_name_input,
        name_rect.x + 10.0,
        name_rect.y + 22.0,
        14.0,
        TEXT,
    );
    if name_rect.contains(vec2(mouse_position().0, mouse_position().1))
        && is_mouse_button_pressed(MouseButton::Left)
    {
        session.map_preset_name_editing = true;
    }
    if button(
        Rect::new(x + width - 124.0, y + 76.0, 106.0, 34.0),
        "Salvar / atualizar",
        false,
    ) {
        action.save_map_preset = Some(session.map_preset_name_input.clone());
    }
    if let Some(status) = &session.map_preset_status {
        draw_text(status, x + 18.0, y + 129.0, 12.0, MUTED);
    }
    let list_y = y + 148.0;
    draw_text("Mapas salvos", x + 18.0, list_y, 15.0, TEXT);
    for (index, preset) in session.saved_map_presets.iter().take(5).enumerate() {
        let row_y = list_y + 12.0 + index as f32 * 42.0;
        draw_value_row(
            x + 18.0,
            row_y,
            width - 178.0,
            &preset.name,
            preset.flow_preset.label(),
        );
        if button(
            Rect::new(x + width - 152.0, row_y, 82.0, 34.0),
            "Carregar",
            false,
        ) {
            action.load_map_preset = Some(index);
        }
        if button(
            Rect::new(x + width - 64.0, row_y, 46.0, 34.0),
            "Apagar",
            false,
        ) {
            action.delete_map_preset = Some(index);
        }
    }
    if button(
        Rect::new(x + width - 104.0, y + height - 42.0, 86.0, 28.0),
        "Fechar",
        false,
    ) {
        session.map_preset_modal_open = false;
        session.map_preset_name_editing = false;
    }
}

fn draw_session_tab_compact(
    left: f32,
    width: f32,
    session: &mut SessionUi,
    world: &World,
    metrics: EngineMetrics,
) -> PanelAction {
    let scroll = session.session_scroll;
    let y = |value: f32| value - scroll;
    let mut action = PanelAction::default();
    if session.pointer_captured_until_release {
        if is_mouse_button_down(MouseButton::Left) {
            return action;
        }
        session.pointer_captured_until_release = false;
    }
    // A seleção do multiplicador é um overlay: qualquer clique nela (ou fora
    // dela) termina aqui e não pode vazar para os campos da sessão abaixo.
    if session.speed_menu_open && is_mouse_button_pressed(MouseButton::Left) {
        let menu_x = left + 88.0;
        let menu_y = y(453.0);
        let menu_width = width - 88.0;
        let option_height = 30.0;
        let menu = Rect::new(
            menu_x - 2.0,
            menu_y - 2.0,
            menu_width + 4.0,
            SPEED_MULTIPLIERS.len() as f32 * option_height + 4.0,
        );
        let mouse = vec2(mouse_position().0, mouse_position().1);
        if menu.contains(mouse) {
            let index = ((mouse.y - menu_y) / option_height).floor() as usize;
            if index < SPEED_MULTIPLIERS.len() {
                session.speed = index;
            }
        }
        session.speed_menu_open = false;
        session.pointer_captured_until_release = true;
        return action;
    }
    draw_text("Sessão", left, y(88.0), 22.0, TEXT);
    draw_text(
        &format!(
            "tick {} · {} células · {:.0} ticks/s · pub. {:.0} Hz",
            world.tick,
            world.organisms.len(),
            metrics.ticks_per_second,
            metrics.publication_hz,
        ),
        left,
        y(109.0),
        14.0,
        MUTED,
    );
    if button(
        Rect::new(left, y(124.0), width, 30.0),
        &format!("Presets · {} salvos", session.saved_presets.len()),
        session.preset_modal_open,
    ) {
        session.preset_modal_open = true;
    }

    section_title(left, y(174.0), "Condições iniciais");
    if editable_number_row(
        Rect::new(left, y(188.0), width - 82.0, 38.0),
        "Seed",
        &session.seed_input,
        session.editing_field == Some(EditingField::Seed),
        session.seed_is_dirty(),
    ) {
        begin_editing(session, EditingField::Seed);
    }
    if button(
        Rect::new(left + width - 74.0, y(188.0), 74.0, 38.0),
        "Nova",
        false,
    ) {
        session.seed = session
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(get_time().to_bits());
        session.seed_input = session.seed.to_string();
        session.editing_field = None;
    }
    if editable_number_row(
        Rect::new(left, y(233.0), width, 38.0),
        "Células iniciais",
        &session.initial_cells_input,
        session.editing_field == Some(EditingField::InitialCells),
        session.initial_cells_are_dirty(),
    ) {
        begin_editing(session, EditingField::InitialCells);
    }
    draw_text("Configuração neural", left, y(286.0), 13.0, MUTED);
    let neural_button_width = (width - 12.0) / 4.0;
    let has_validated_population = session
        .validated_population
        .as_ref()
        .is_some_and(|population| !population.genomes.is_empty());
    for (index, configuration, label) in [
        (0, InitialNeuralConfiguration::Classic, "Clássica"),
        (1, InitialNeuralConfiguration::MotorMinimum, "Motor"),
        (2, InitialNeuralConfiguration::ValidatedV1, "Val 1.0"),
        (3, InitialNeuralConfiguration::ValidatedV2, "Val 2.0"),
    ] {
        let allowed = index < 2 || has_validated_population;
        if button_dirty(
            Rect::new(
                left + index as f32 * (neural_button_width + 4.0),
                y(293.0),
                neural_button_width,
                32.0,
            ),
            label,
            session.initial_neural_configuration == configuration,
            session.initial_neural_configuration_is_dirty() && allowed,
        ) && allowed
        {
            session.initial_neural_configuration = configuration;
        }
    }
    let neural_description = match session.initial_neural_configuration {
        InitialNeuralConfiguration::Classic => "Genomas iniciais totalmente aleatórios.".to_owned(),
        InitialNeuralConfiguration::MotorMinimum => {
            "4 caminhos diretos alimento → motor; resto aleatório.".to_owned()
        }
        InitialNeuralConfiguration::ValidatedV1 => "Pool validado salvo na última run.".to_owned(),
        InitialNeuralConfiguration::ValidatedV2 => {
            "Pool validado: alimento → motor + Ct + PG → Doar.".to_owned()
        }
    };
    draw_text(&neural_description, left, y(340.0), 11.0, MUTED);
    update_numeric_input(session);

    section_title(left, y(365.0), "Execução");
    if button(
        Rect::new(left, y(379.0), 72.0, 32.0),
        if session.paused { "Iniciar" } else { "Pausar" },
        !session.paused,
    ) {
        session.paused = !session.paused;
    }
    if button_dirty(
        Rect::new(left + 76.0, y(379.0), 64.0, 32.0),
        "Aplicar",
        false,
        session.has_pending_live_changes(),
    ) {
        action.apply = true;
    }
    if button_enabled(
        Rect::new(left + 144.0, y(379.0), 72.0, 32.0),
        "Reverter",
        session.has_pending_changes(),
    ) {
        action.revert = true;
    }
    if button_dirty(
        Rect::new(left + 220.0, y(379.0), width - 220.0, 32.0),
        "Reiniciar",
        false,
        session.seed_is_dirty()
            || session.initial_cells_are_dirty()
            || session.initial_neural_configuration_is_dirty()
            || session.amadeus_mode != session.applied.amadeus_mode,
    ) {
        action.restart = true;
    }
    draw_text("Velocidade", left, y(439.0), 15.0, MUTED);
    if button(
        Rect::new(left + 88.0, y(418.0), width - 88.0, 32.0),
        speed_label(session.speed),
        session.speed_menu_open,
    ) {
        session.speed_menu_open = !session.speed_menu_open;
        session.pointer_captured_until_release = true;
        return action;
    }
    if button(
        Rect::new(left, y(460.0), width, 32.0),
        "Modo headless · medir teto de cálculo",
        false,
    ) {
        action.open_headless_modal = true;
    }

    let mut cursor = 516.0;
    collapsible_session_section(
        left,
        y(cursor),
        width,
        "Células",
        &mut session.cells_section_open,
    );
    cursor += 40.0;
    if session.cells_section_open {
        if editable_number_row(
            Rect::new(left, y(cursor), width, 38.0),
            "Mínimo reprodução",
            &session.reproduction_energy_input,
            session.editing_field == Some(EditingField::ReproductionEnergy),
            session.reproduction_energy_is_dirty(),
        ) {
            begin_editing(session, EditingField::ReproductionEnergy);
        }
        cursor += 45.0;
        if editable_number_row(
            Rect::new(left, y(cursor), width, 38.0),
            "Limite obrigatório",
            &session.reproduction_limit_input,
            session.editing_field == Some(EditingField::ReproductionLimit),
            session.reproduction_limit_is_dirty(),
        ) {
            begin_editing(session, EditingField::ReproductionLimit);
        }
        cursor += 52.0;
        draw_text(
            "Cones e mapa químico: aba Filtros.",
            left,
            y(cursor),
            12.0,
            MUTED,
        );
        cursor += 20.0;
        if let Some(cost) = slider_row(
            left,
            y(cursor),
            width,
            "Custo mental",
            mental_cost_display(session.mental_energy_cost),
            0.0,
            mental_cost_display(0.02),
            &format!(
                "{:.0} unidades / conexão/tick",
                mental_cost_display(session.mental_energy_cost)
            ),
            session.mental_energy_cost != session.applied.mental_energy_cost,
        ) {
            session.mental_energy_cost = mental_cost_from_display(cost);
        }
        cursor += 53.0;
    }

    collapsible_session_section(
        left,
        y(cursor),
        width,
        "Fitoplâncton",
        &mut session.phytoplankton_section_open,
    );
    cursor += 40.0;
    if session.phytoplankton_section_open {
        let food_status = format!(
            "deserto {} · fértil {} · {} no mapa",
            format_emission_display(metrics.active_desert_emission),
            format_emission_display(metrics.active_fertile_emission),
            world.food.len()
        );
        draw_text(&food_status, left, y(cursor), 11.0, MUTED);
        cursor += 12.0;
        if editable_number_row(
            Rect::new(left, y(cursor), width, 38.0),
            "Emissão deserto",
            &session.desert_emission_input,
            session.editing_field == Some(EditingField::DesertEmission),
            session.desert_emission_is_dirty(),
        ) {
            begin_editing(session, EditingField::DesertEmission);
        }
        cursor += 45.0;
        if editable_number_row(
            Rect::new(left, y(cursor), width, 38.0),
            "Emissão terra fértil",
            &session.fertile_emission_input,
            session.editing_field == Some(EditingField::FertileEmission),
            session.fertile_emission_is_dirty(),
        ) {
            begin_editing(session, EditingField::FertileEmission);
        }
        cursor += 45.0;
        if let Some(lifetime) = slider_row(
            left,
            y(cursor),
            width,
            "Vida fito",
            session.lifetime as f32,
            100.0,
            20_000.0,
            &format!("{} ticks", session.lifetime),
            session.lifetime != session.applied.lifetime,
        ) {
            session.lifetime = lifetime.round() as i32;
        }
        cursor += 45.0;
        match stepper_row(
            left,
            y(cursor),
            width,
            "Energia",
            &format!("{:.0}", session.food_energy),
            None,
            session.food_energy != session.applied.food_energy,
        ) {
            Some(-1) => session.food_energy = (session.food_energy - 5.0).max(0.0),
            Some(1) => session.food_energy += 5.0,
            _ => {}
        }
        cursor += 45.0;
    }

    collapsible_session_section(
        left,
        y(cursor),
        width,
        "Corrente",
        &mut session.flow_section_open,
    );
    cursor += 40.0;
    if session.flow_section_open {
        if let Some(intensity) = slider_row(
            left,
            y(cursor),
            width,
            "Intensidade",
            session.flow_intensity,
            0.0,
            1.0,
            &format!("{:.0}%", session.flow_intensity * 100.0),
            session.flow_intensity != session.applied.flow_intensity,
        ) {
            session.flow_intensity = intensity;
        }
        cursor += 45.0;
        if button_dirty(
            Rect::new(left, y(cursor), width, 34.0),
            if session.flow_enabled {
                "Corrente ligada"
            } else {
                "Corrente desligada"
            },
            session.flow_enabled,
            session.flow_enabled != session.applied.flow_enabled,
        ) {
            session.flow_enabled = !session.flow_enabled;
        }
        cursor += 48.0;
        draw_text("Vetores: aba Filtros.", left, y(cursor), 12.0, MUTED);
        cursor += 24.0;
    }

    collapsible_session_section(
        left,
        y(cursor),
        width,
        "Densidade",
        &mut session.density_section_open,
    );
    cursor += 40.0;
    if session.density_section_open {
        let density_status = if !session.anti_cramming {
            "Densidade desligada".to_owned()
        } else if let Some(minimum) = world.crowding.minimum_violation_distance {
            format!(
                "{} pares · mín {:.0}u",
                world.crowding.violating_pairs, minimum
            )
        } else {
            "Densidade livre".to_owned()
        };
        draw_text(&density_status, left, y(cursor), 12.0, MUTED);
        cursor += 14.0;
        if button_dirty(
            Rect::new(left, y(cursor), width * 0.54 - 4.0, 34.0),
            if session.anti_cramming {
                "Anti-cramming: on"
            } else {
                "Anti-cramming: off"
            },
            session.anti_cramming,
            session.anti_cramming != session.applied.anti_cramming,
        ) {
            session.anti_cramming = !session.anti_cramming;
        }
        if button(
            Rect::new(
                left + width * 0.54 + 4.0,
                y(cursor),
                width * 0.46 - 4.0,
                34.0,
            ),
            "Ver conflitos",
            session.show_crowding_violations,
        ) {
            session.show_crowding_violations = !session.show_crowding_violations;
        }
        cursor += 45.0;
        if let Some(force) = slider_row(
            left,
            y(cursor),
            width,
            "Força",
            session.anti_cramming_force,
            0.0,
            1.0,
            &format!("{:.0}%", session.anti_cramming_force * 100.0),
            session.anti_cramming_force != session.applied.anti_cramming_force,
        ) {
            session.anti_cramming_force = force;
        }
        cursor += 45.0;
        if let Some(distance) = slider_row(
            left,
            y(cursor),
            width,
            "Espaço",
            session.anti_cramming_distance,
            MIN_EXCLUSION_DISTANCE,
            MAX_EXCLUSION_DISTANCE,
            &format!("{:.1}R", session.anti_cramming_distance / CELL_RADIUS),
            session.anti_cramming_distance != session.applied.anti_cramming_distance,
        ) {
            session.anti_cramming_distance = distance;
        }
    }

    if session.speed_menu_open {
        let menu_x = left + 88.0;
        let menu_y = y(453.0);
        let menu_width = width - 88.0;
        let option_height = 30.0;
        draw_rectangle(
            menu_x - 2.0,
            menu_y - 2.0,
            menu_width + 4.0,
            SPEED_MULTIPLIERS.len() as f32 * option_height + 4.0,
            BACKGROUND,
        );
        draw_rectangle_lines(
            menu_x - 2.0,
            menu_y - 2.0,
            menu_width + 4.0,
            SPEED_MULTIPLIERS.len() as f32 * option_height + 4.0,
            1.0,
            BORDER,
        );
        for index in 0..SPEED_MULTIPLIERS.len() {
            if button(
                Rect::new(
                    menu_x,
                    menu_y + index as f32 * option_height,
                    menu_width,
                    option_height,
                ),
                speed_label(index),
                session.speed == index,
            ) {
                session.speed = index;
                session.speed_menu_open = false;
            }
        }
    }
    draw_session_preset_modal(session, &mut action);
    action
}

fn draw_settings_tab(left: f32, width: f32, session: &mut SessionUi, world: &World) -> PanelAction {
    let mut action = PanelAction::default();
    // O seletor central de correnteza também é modal: consumir o clique aqui
    // impede que o botão da tela de configurações receba o mesmo evento.
    if session.flow_preset_modal_open && is_mouse_button_pressed(MouseButton::Left) {
        let modal_width = 360.0;
        let modal_height = 240.0;
        let modal_x = (screen_width() - modal_width) * 0.5;
        let modal_y = (screen_height() - modal_height) * 0.5;
        let mouse = vec2(mouse_position().0, mouse_position().1);
        for (index, preset) in [FlowPreset::Classic, FlowPreset::Left]
            .into_iter()
            .enumerate()
        {
            if Rect::new(
                modal_x + 18.0,
                modal_y + 74.0 + index as f32 * 40.0,
                modal_width - 36.0,
                34.0,
            )
            .contains(mouse)
            {
                action.apply = session.flow_preset != preset;
                session.flow_preset = preset;
                break;
            }
        }
        session.flow_preset_modal_open = false;
        return action;
    }
    draw_text("Configurações", left, 92.0, 22.0, TEXT);
    draw_text(
        &format!(
            "run {} · tick {} · {} células",
            session.run_id,
            world.tick,
            world.organisms.len()
        ),
        left,
        113.0,
        13.0,
        MUTED,
    );
    section_title(left, 140.0, "Execução");
    if checkbox_row(
        Rect::new(left, 152.0, width, 32.0),
        "Modo Amadeus · uma célula central",
        session.amadeus_mode,
    ) {
        session.amadeus_mode = !session.amadeus_mode;
        session.save_status = Some(if session.amadeus_mode {
            "Amadeus ativo: reinicie para gerar uma única célula central.".to_owned()
        } else {
            "Amadeus desativado: reinicie para usar a população inicial.".to_owned()
        });
    }
    draw_text(
        "A linhagem inicial é única; descendentes ainda mutam.",
        left,
        202.0,
        12.0,
        MUTED,
    );
    section_title(left, 228.0, "Editor de mapa");
    if button(
        Rect::new(left, 240.0, width, 32.0),
        &format!(
            "Presets de mapa · {} salvos",
            session.saved_map_presets.len()
        ),
        session.map_preset_modal_open,
    ) {
        session.map_preset_modal_open = true;
    }
    let tool_width = (width - 8.0) / 3.0;
    if button(
        Rect::new(left, 284.0, tool_width, 32.0),
        "Parede",
        session.map_tool == MapTool::Wall,
    ) {
        session.map_tool = if session.map_tool == MapTool::Wall {
            MapTool::None
        } else {
            MapTool::Wall
        };
        session.lasso_points.clear();
        session.last_brush_position = None;
    }
    if button(
        Rect::new(left + tool_width + 4.0, 284.0, tool_width, 32.0),
        "Borracha",
        session.map_tool == MapTool::Eraser,
    ) {
        session.map_tool = if session.map_tool == MapTool::Eraser {
            MapTool::None
        } else {
            MapTool::Eraser
        };
        session.lasso_points.clear();
        session.last_brush_position = None;
    }
    if button(
        Rect::new(left + (tool_width + 4.0) * 2.0, 284.0, tool_width, 32.0),
        "Área fértil",
        session.map_tool == MapTool::FertileLasso || session.edited_map.has_fertile_area(),
    ) {
        if session.edited_map.has_fertile_area() && session.map_tool != MapTool::FertileLasso {
            session.edited_map.clear_fertile();
            session.map_dirty = true;
        } else {
            session.map_tool = MapTool::FertileLasso;
            session.lasso_points.clear();
            session.last_brush_position = None;
        }
    }
    if button(
        Rect::new(left, 328.0, width, 32.0),
        if session.show_fertile_area {
            "Ocultar área fértil"
        } else {
            "Visualizar área fértil"
        },
        session.show_fertile_area,
    ) {
        session.show_fertile_area = !session.show_fertile_area;
    }
    if button(
        Rect::new(left, 372.0, width, 32.0),
        &format!("Correnteza · {}", session.flow_preset.label()),
        session.flow_preset_modal_open,
    ) {
        session.flow_preset_modal_open = true;
    }
    let mut editor_controls_y = 420.0;
    if session.map_tool == MapTool::Wall || session.map_tool == MapTool::Eraser {
        if let Some(radius) = slider_row(
            left,
            editor_controls_y,
            width,
            "Raio do pincel",
            session.wall_brush_radius as f32,
            0.0,
            12.0,
            &format!("{} pixels", session.wall_brush_radius),
            false,
        ) {
            session.wall_brush_radius = radius.round() as u8;
        }
        editor_controls_y += 52.0;
    }
    draw_text(
        "Alterações só entram no mundo após Aplicar mapa.",
        left,
        editor_controls_y,
        12.0,
        MUTED,
    );
    editor_controls_y += 10.0;
    let apply_clicked = button_dirty(
        Rect::new(left, editor_controls_y, width, 32.0),
        if session.map_dirty {
            "Aplicar mapa*"
        } else {
            "Aplicar mapa"
        },
        session.map_dirty,
        session.map_dirty,
    );
    if apply_clicked && session.map_dirty {
        action.apply = true;
    }
    if let Some(status) = &session.save_status {
        draw_text(status, left, editor_controls_y + 48.0, 13.0, MUTED);
    }
    if session.flow_preset_modal_open && draw_flow_preset_modal(session) {
        action.apply = true;
    }
    draw_map_preset_modal(session, &mut action);
    action
}

fn draw_flow_preset_modal(session: &mut SessionUi) -> bool {
    let width = 360.0;
    let height = 240.0;
    let x = (screen_width() - width) * 0.5;
    let y = (screen_height() - height) * 0.5;
    draw_rectangle(
        x - 8.0,
        y - 8.0,
        width + 16.0,
        height + 16.0,
        Color::new(0.01, 0.015, 0.025, 0.86),
    );
    draw_rectangle(x, y, width, height, SURFACE);
    draw_rectangle_lines(x, y, width, height, 1.0, ACCENT);
    draw_text("Correnteza", x + 18.0, y + 34.0, 22.0, TEXT);
    draw_text(
        "Escolha o modelo de vento do mapa.",
        x + 18.0,
        y + 56.0,
        13.0,
        MUTED,
    );
    let mut changed = false;
    for (index, preset) in [FlowPreset::Classic, FlowPreset::Left]
        .into_iter()
        .enumerate()
    {
        let label = match preset {
            FlowPreset::Classic => "Clássico · recirculante com giros locais",
            FlowPreset::Left => "Esquerda · vento contínuo para a esquerda",
            // Mantido apenas para compatibilidade de arquivos antigos; não é
            // apresentado nem selecionável na interface atual.
            FlowPreset::Oasis => "Clássico",
        };
        if button(
            Rect::new(x + 18.0, y + 74.0 + index as f32 * 40.0, width - 36.0, 34.0),
            label,
            session.flow_preset == preset,
        ) {
            changed = session.flow_preset != preset;
            session.flow_preset = preset;
            session.flow_preset_modal_open = false;
        }
    }
    if button(
        Rect::new(x + width - 88.0, y + height - 36.0, 70.0, 24.0),
        "Fechar",
        false,
    ) {
        session.flow_preset_modal_open = false;
    }
    changed
}

fn draw_cells_tab(left: f32, width: f32, session: &mut SessionUi, world: &World) {
    draw_text("Células", left, 88.0, 22.0, TEXT);
    if button(
        Rect::new(left + width - 164.0, 68.0, 78.0, 28.0),
        "Indivíduo",
        session.cells_view == CellsView::Individual,
    ) {
        session.cells_view = CellsView::Individual;
    }
    if button(
        Rect::new(left + width - 80.0, 68.0, 80.0, 28.0),
        "Genética",
        session.cells_view == CellsView::Genetics,
    ) {
        session.cells_view = CellsView::Genetics;
    }
    match session.cells_view {
        CellsView::Individual => draw_individual_cells_view(left, width, session, world),
        CellsView::Genetics => {
            refresh_genetics_report(session, world);
            draw_genetics_view(left, width, session, world);
        }
    }
}

fn draw_individual_cells_view(left: f32, width: f32, session: &mut SessionUi, world: &World) {
    draw_text(
        &format!("{} FPS · tick {}", get_fps(), world.tick),
        left,
        109.0,
        14.0,
        MUTED,
    );
    if world.organisms.is_empty() {
        draw_text(
            "Não há células vivas nesta sessão.",
            left,
            148.0,
            16.0,
            MUTED,
        );
        return;
    }
    let current_index = selected_index(session.selected_cell_id, &world.organisms).unwrap_or(0);
    if button(Rect::new(left, 126.0, 38.0, 38.0), "<", false) {
        let index = (current_index + world.organisms.len() - 1) % world.organisms.len();
        session.selected_cell_id = Some(world.organisms[index].id);
    }
    if button(
        Rect::new(left + width - 38.0, 126.0, 38.0, 38.0),
        ">",
        false,
    ) {
        let index = (current_index + 1) % world.organisms.len();
        session.selected_cell_id = Some(world.organisms[index].id);
    }
    let selected = &world.organisms
        [selected_index(session.selected_cell_id, &world.organisms).unwrap_or(current_index)];
    draw_rectangle(left + 46.0, 126.0, width - 92.0, 38.0, SURFACE);
    let name = format!(
        "Célula #{} · {}",
        selected.id,
        selected.genome.genetic_hex()
    );
    let name_width = measure_text(&name, None, 16, 1.0).width;
    draw_text(
        &name,
        left + width / 2.0 - name_width / 2.0,
        150.0,
        16.0,
        TEXT,
    );
    draw_value_row(
        left,
        178.0,
        width,
        "Energia",
        &format!("{:.1}/{:.0}", selected.energy, session.reproduction_limit),
    );
    draw_value_row(
        left,
        218.0,
        width,
        "Conexões / neurônios",
        &format!(
            "{} / {} ativos",
            selected.brain.connection_count(),
            selected.brain.active_node_count()
        ),
    );
    draw_value_row(
        left,
        258.0,
        width,
        "Filhos produzidos",
        &selected.children_produced.to_string(),
    );
    draw_value_row(
        left,
        298.0,
        width,
        "Tempo viva / limite",
        &format!(
            "{} / {} ticks",
            selected.age_ticks,
            selected.maximum_age_ticks()
        ),
    );
    let sensor_range = selected.food_sensor_range();
    let sensor_cost = selected.food_sensor_energy_cost();
    draw_value_row(
        left,
        338.0,
        width,
        "Alcance alimento",
        &format!("{sensor_range:.0} · custo {sensor_cost:.4}/tick"),
    );
    let similarity = world
        .most_similar_living_cell(selected.id)
        .map(|(id, value)| format!("mais parecida: #{} · {:.1}%", id, value * 100.0))
        .unwrap_or_else(|| "sem outra célula para comparar".to_owned());
    draw_text(&similarity, left + 4.0, 384.0, 12.0, MUTED);
    let genome_button_y = 392.0;
    let genome_label = if session.show_genome {
        "Código genético  ▲"
    } else {
        "Código genético  ▼"
    };
    if button(
        Rect::new(left, genome_button_y, width, 34.0),
        genome_label,
        session.show_genome,
    ) {
        session.show_genome = !session.show_genome;
    }

    let neural_title_y = if session.show_genome { 664.0 } else { 462.0 };
    if session.show_genome {
        draw_binary_genome(
            Rect::new(left, 434.0, width, 210.0),
            &selected.genome.binary_string(),
        );
    }
    draw_text("Rede neural ao vivo", left, neural_title_y, 17.0, TEXT);
    draw_text(
        "verde: sinal positivo · vermelho: negativo",
        left,
        neural_title_y + 19.0,
        13.0,
        MUTED,
    );
    let viewer = Rect::new(
        left,
        neural_title_y + 32.0,
        width,
        (screen_height() - neural_title_y - 52.0).max(180.0),
    );
    draw_brain_viewer(viewer, selected);
}

fn refresh_genetics_report(session: &mut SessionUi, world: &World) {
    let stale = session
        .genetics_report
        .as_ref()
        .map(|report| world.tick.saturating_sub(report.tick) >= GENETICS_REFRESH_TICKS)
        .unwrap_or(true);
    if stale {
        session.genetics_report = Some(build_genetics_report(world));
    }
}

struct ClusterBuilder {
    representative: usize,
    members: Vec<usize>,
}

#[derive(Clone, Copy, Default)]
struct ConnectionAggregate {
    organisms_using: usize,
    weight_sum: f32,
    weight_count: usize,
}

fn build_genetics_report(world: &World) -> GeneticsReport {
    let population = world.organisms.len();
    if population == 0 {
        return GeneticsReport {
            tick: world.tick,
            population: 0,
            consensus_agreement: 0.0,
            clusters: Vec::new(),
            connections: Vec::new(),
            neurons: Vec::new(),
        };
    }

    let mut builders: Vec<ClusterBuilder> = Vec::new();
    let mut band_index: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut organism_cluster = vec![0usize; population];
    for organism_index in 0..population {
        let organism = &world.organisms[organism_index];
        let mut candidates = HashSet::new();
        for band in 0..8 {
            if let Some(cluster_indices) = band_index.get(&genome_band_key(organism, band)) {
                candidates.extend(cluster_indices.iter().copied());
            }
        }
        let best = candidates
            .into_iter()
            .filter_map(|cluster_index| {
                let representative = &world.organisms[builders[cluster_index].representative];
                let similarity = organism.genome.similarity_to(&representative.genome);
                (similarity >= GENETIC_GROUP_SIMILARITY).then_some((cluster_index, similarity))
            })
            .max_by(|left, right| left.1.total_cmp(&right.1));
        let cluster_index = match best {
            Some((index, _)) => {
                builders[index].members.push(organism_index);
                index
            }
            None => {
                let index = builders.len();
                builders.push(ClusterBuilder {
                    representative: organism_index,
                    members: vec![organism_index],
                });
                for band in 0..8 {
                    band_index
                        .entry(genome_band_key(organism, band))
                        .or_default()
                        .push(index);
                }
                index
            }
        };
        organism_cluster[organism_index] = cluster_index;
    }

    let mut ones_per_bit = [0u32; GENOME_BIT_COUNT];
    let mut connection_totals = vec![ConnectionAggregate::default(); SLOT_COUNT * SLOT_COUNT];
    let mut groups_per_connection = HashSet::new();
    let mut used_by = [0usize; SLOT_COUNT];
    let mut outgoing_by = [0usize; SLOT_COUNT];
    let mut incoming_by = [0usize; SLOT_COUNT];
    for (organism_index, organism) in world.organisms.iter().enumerate() {
        for (byte_index, byte) in organism.genome.bytes().iter().enumerate() {
            for bit_in_byte in 0..8 {
                let bit_index = byte_index * 8 + bit_in_byte;
                if bit_index >= GENOME_BIT_COUNT {
                    continue;
                }
                ones_per_bit[bit_index] += ((byte >> (7 - bit_in_byte)) & 1) as u32;
            }
        }
        let mut seen_pairs = [false; SLOT_COUNT * SLOT_COUNT];
        let mut used_slots = [false; SLOT_COUNT];
        let mut outgoing_slots = [false; SLOT_COUNT];
        let mut incoming_slots = [false; SLOT_COUNT];
        for connection in organism.genome.decode_connections() {
            let key = connection.source * SLOT_COUNT + connection.destination;
            if !seen_pairs[key] {
                seen_pairs[key] = true;
                connection_totals[key].organisms_using += 1;
                groups_per_connection.insert((organism_cluster[organism_index], key));
            }
            connection_totals[key].weight_sum += connection.weight;
            connection_totals[key].weight_count += 1;
            used_slots[connection.source] = true;
            used_slots[connection.destination] = true;
            outgoing_slots[connection.source] = true;
            incoming_slots[connection.destination] = true;
        }
        for slot in 0..SLOT_COUNT {
            used_by[slot] += used_slots[slot] as usize;
            outgoing_by[slot] += outgoing_slots[slot] as usize;
            incoming_by[slot] += incoming_slots[slot] as usize;
        }
    }

    let mut group_counts_per_connection = vec![0usize; SLOT_COUNT * SLOT_COUNT];
    for (_, key) in groups_per_connection {
        group_counts_per_connection[key] += 1;
    }
    let mut connections: Vec<ConnectionKpi> = connection_totals
        .iter()
        .enumerate()
        .filter_map(|(key, aggregate)| {
            (aggregate.organisms_using > 0).then_some(ConnectionKpi {
                source: key / SLOT_COUNT,
                destination: key % SLOT_COUNT,
                organisms_using: aggregate.organisms_using,
                average_weight: aggregate.weight_sum / aggregate.weight_count as f32,
                groups_using: group_counts_per_connection[key],
            })
        })
        .collect();
    connections.sort_by(|left, right| {
        right
            .organisms_using
            .cmp(&left.organisms_using)
            .then_with(|| {
                right
                    .average_weight
                    .abs()
                    .total_cmp(&left.average_weight.abs())
            })
    });
    let mut neurons: Vec<NeuronKpi> = (0..SLOT_COUNT)
        .map(|slot| NeuronKpi {
            slot,
            used_by: used_by[slot],
            outgoing_by: outgoing_by[slot],
            incoming_by: incoming_by[slot],
        })
        .collect();
    neurons.sort_by(|left, right| right.used_by.cmp(&left.used_by));

    let mut clusters: Vec<GeneticCluster> = builders
        .iter()
        .map(|builder| {
            let representative = &world.organisms[builder.representative];
            let members = builder.members.len();
            let average_similarity = builder
                .members
                .iter()
                .map(|index| {
                    representative
                        .genome
                        .similarity_to(&world.organisms[*index].genome)
                })
                .sum::<f32>()
                / members as f32;
            let average_generation = builder
                .members
                .iter()
                .map(|index| world.organisms[*index].generation as f32)
                .sum::<f32>()
                / members as f32;
            let [red, green, blue] = representative.genome.genetic_rgb();
            GeneticCluster {
                representative_id: representative.id,
                members,
                average_generation,
                average_similarity,
                color: Color::from_rgba(red, green, blue, 255),
            }
        })
        .collect();
    clusters.sort_by(|left, right| right.members.cmp(&left.members));

    let consensus_agreement = ones_per_bit
        .iter()
        .map(|ones| {
            let frequency = *ones as f32 / population as f32;
            frequency.max(1.0 - frequency)
        })
        .sum::<f32>()
        / ones_per_bit.len() as f32;
    GeneticsReport {
        tick: world.tick,
        population,
        consensus_agreement,
        clusters,
        connections,
        neurons,
    }
}

fn genome_band_key(organism: &Organism, band: usize) -> u32 {
    let bytes = organism.genome.bytes();
    let byte_index = band * 12;
    ((band as u32) << 16) | ((bytes[byte_index] as u32) << 8) | bytes[byte_index + 1] as u32
}

fn draw_genetics_view(left: f32, width: f32, session: &mut SessionUi, world: &World) {
    draw_text(
        &format!(
            "{} células vivas · tick {}",
            world.organisms.len(),
            world.tick
        ),
        left,
        109.0,
        14.0,
        MUTED,
    );
    for (index, (label, view)) in [
        ("Resumo", GeneticsView::Summary),
        ("Grupos", GeneticsView::Groups),
        ("Circuitos", GeneticsView::Circuits),
    ]
    .iter()
    .enumerate()
    {
        if button(
            Rect::new(left + index as f32 * 105.0, 122.0, 99.0, 29.0),
            label,
            session.genetics_view == *view,
        ) {
            session.genetics_view = *view;
        }
    }
    let Some(report) = session.genetics_report.as_ref() else {
        return;
    };
    match session.genetics_view {
        GeneticsView::Summary => draw_genetics_summary(left, width, report),
        GeneticsView::Groups => {
            if let Some(representative_id) = draw_genetics_groups(left, width, report) {
                session.selected_cell_id = Some(representative_id);
                session.cells_view = CellsView::Individual;
            }
        }
        GeneticsView::Circuits => {
            if button(
                Rect::new(left, 164.0, width / 2.0 - 3.0, 28.0),
                "Conexões",
                session.circuit_view == CircuitView::Connections,
            ) {
                session.circuit_view = CircuitView::Connections;
            }
            if button(
                Rect::new(left + width / 2.0 + 3.0, 164.0, width / 2.0 - 3.0, 28.0),
                "Neurônios",
                session.circuit_view == CircuitView::Neurons,
            ) {
                session.circuit_view = CircuitView::Neurons;
            }
            match session.circuit_view {
                CircuitView::Connections => draw_connection_ranking(left, width, report),
                CircuitView::Neurons => draw_neuron_ranking(left, width, report),
            }
        }
    }
}

fn draw_genetics_summary(left: f32, width: f32, report: &GeneticsReport) {
    let dominant_share = report
        .clusters
        .first()
        .map(|cluster| cluster.members as f32 / report.population as f32)
        .unwrap_or(0.0);
    draw_value_row(
        left,
        164.0,
        width,
        "População analisada",
        &report.population.to_string(),
    );
    draw_value_row(
        left,
        204.0,
        width,
        "Grupos genéticos",
        &report.clusters.len().to_string(),
    );
    draw_value_row(
        left,
        244.0,
        width,
        "Grupo dominante",
        &format!("{:.1}%", dominant_share * 100.0),
    );
    draw_value_row(
        left,
        284.0,
        width,
        "Concordância consenso",
        &format!("{:.1}%", report.consensus_agreement * 100.0),
    );
    if let Some(cluster) = report.clusters.first() {
        draw_text("Grupo dominante", left, 348.0, 16.0, TEXT);
        draw_rectangle(left, 360.0, 10.0, 42.0, cluster.color);
        draw_text(
            &format!(
                "{} células · geração média {:.1}",
                cluster.members, cluster.average_generation
            ),
            left + 20.0,
            379.0,
            14.0,
            TEXT,
        );
        draw_text(
            &format!("coesão genética {:.2}%", cluster.average_similarity * 100.0),
            left + 20.0,
            397.0,
            12.0,
            MUTED,
        );
    }
    draw_text("Circuito mais recorrente", left, 434.0, 16.0, TEXT);
    if let Some(connection) = report.connections.first() {
        draw_connection_card(left, 446.0, width, connection, report.population);
    }
    draw_text("Slot mais usado", left, 516.0, 16.0, TEXT);
    if let Some(neuron) = report.neurons.first() {
        draw_value_row(
            left,
            528.0,
            width,
            slot_label(neuron.slot),
            &format!(
                "{:.1}% dos cérebros",
                neuron.used_by as f32 / report.population as f32 * 100.0
            ),
        );
    }
}

fn draw_genetics_groups(left: f32, width: f32, report: &GeneticsReport) -> Option<u64> {
    draw_text(
        "Grupos por similaridade mínima de 98,5%",
        left,
        174.0,
        13.0,
        MUTED,
    );
    let mut opened_representative = None;
    for (index, cluster) in report.clusters.iter().take(8).enumerate() {
        let y = 188.0 + index as f32 * 56.0;
        if button(
            Rect::new(left, y, width, 50.0),
            &format!(
                "Grupo {} · {} células ({:.1}%)",
                index + 1,
                cluster.members,
                cluster.members as f32 / report.population as f32 * 100.0
            ),
            false,
        ) {
            opened_representative = Some(cluster.representative_id);
        }
        draw_rectangle(left + 8.0, y + 8.0, 7.0, 34.0, cluster.color);
        draw_text(
            &format!(
                "gen. {:.1} · coesão {:.2}% · abrir representante",
                cluster.average_generation,
                cluster.average_similarity * 100.0
            ),
            left + 22.0,
            y + 42.0,
            11.0,
            MUTED,
        );
    }
    if report.clusters.len() > 8 {
        draw_text(
            &format!("+ {} grupos menores", report.clusters.len() - 8),
            left,
            188.0 + 8.0 * 56.0 + 16.0,
            13.0,
            MUTED,
        );
    }
    opened_representative
}

fn draw_connection_ranking(left: f32, width: f32, report: &GeneticsReport) {
    draw_text(
        "Pares origem -> destino mais presentes",
        left,
        212.0,
        14.0,
        MUTED,
    );
    for (index, connection) in report.connections.iter().take(6).enumerate() {
        draw_connection_card(
            left,
            224.0 + index as f32 * 66.0,
            width,
            connection,
            report.population,
        );
    }
}

fn draw_connection_card(
    left: f32,
    y: f32,
    width: f32,
    connection: &ConnectionKpi,
    population: usize,
) {
    draw_rectangle(left, y, width, 58.0, SURFACE);
    draw_text(
        &format!(
            "{} -> {}",
            slot_label(connection.source),
            slot_label(connection.destination)
        ),
        left + 10.0,
        y + 21.0,
        15.0,
        TEXT,
    );
    let color = if connection.average_weight >= 0.0 {
        ACCENT
    } else {
        RED
    };
    draw_text(
        &format!("peso {:+.2}", connection.average_weight),
        left + width - 98.0,
        y + 21.0,
        13.0,
        color,
    );
    draw_text(
        &format!(
            "{:.1}% dos cérebros · {} grupos",
            connection.organisms_using as f32 / population as f32 * 100.0,
            connection.groups_using
        ),
        left + 10.0,
        y + 43.0,
        12.0,
        MUTED,
    );
}

fn draw_neuron_ranking(left: f32, width: f32, report: &GeneticsReport) {
    let source = report
        .neurons
        .iter()
        .max_by_key(|neuron| neuron.outgoing_by);
    let destination = report
        .neurons
        .iter()
        .max_by_key(|neuron| neuron.incoming_by);
    let motor = report
        .neurons
        .iter()
        .filter(|neuron| neuron.slot >= MOTOR_START)
        .max_by_key(|neuron| neuron.incoming_by);
    if let Some(neuron) = source {
        draw_value_row(
            left,
            204.0,
            width,
            "Origem mais usada",
            &format!(
                "{} · {:.1}%",
                slot_label(neuron.slot),
                neuron.outgoing_by as f32 / report.population as f32 * 100.0
            ),
        );
    }
    if let Some(neuron) = destination {
        draw_value_row(
            left,
            244.0,
            width,
            "Destino mais recrutado",
            &format!(
                "{} · {:.1}%",
                slot_label(neuron.slot),
                neuron.incoming_by as f32 / report.population as f32 * 100.0
            ),
        );
    }
    if let Some(neuron) = motor {
        draw_value_row(
            left,
            284.0,
            width,
            "Motor mais alimentado",
            &format!(
                "{} · {:.1}%",
                slot_label(neuron.slot),
                neuron.incoming_by as f32 / report.population as f32 * 100.0
            ),
        );
    }
    draw_text("Participação por slot", left, 340.0, 16.0, TEXT);
    for (index, neuron) in report.neurons.iter().take(7).enumerate() {
        draw_value_row(
            left,
            352.0 + index as f32 * 38.0,
            width,
            slot_label(neuron.slot),
            &format!(
                "uso {:.0}% · sai {:.0}% · entra {:.0}%",
                neuron.used_by as f32 / report.population as f32 * 100.0,
                neuron.outgoing_by as f32 / report.population as f32 * 100.0,
                neuron.incoming_by as f32 / report.population as f32 * 100.0,
            ),
        );
    }
}

fn draw_demographics_dashboard(
    rect: Rect,
    telemetry: &[TelemetrySample],
    data: &DemographicsWindowData,
    window: &mut DemographicWindow,
) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.052, 0.064, 0.086, 1.0),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    draw_text(
        "Demografia da sessão",
        rect.x + 20.0,
        rect.y + 32.0,
        24.0,
        TEXT,
    );
    let filter_width = 58.0;
    let filter_gap = 4.0;
    let filters = [
        ("5k", DemographicWindow::Last5k),
        ("30k", DemographicWindow::Last30k),
        ("100k", DemographicWindow::Last100k),
        ("25%", DemographicWindow::LastQuarter),
        ("Tudo", DemographicWindow::EntireSession),
    ];
    let filter_start = rect.x + rect.w - 100.0 - (filter_width + filter_gap) * filters.len() as f32;
    for (index, (label, candidate)) in filters.iter().enumerate() {
        if button(
            Rect::new(
                filter_start + index as f32 * (filter_width + filter_gap),
                rect.y + 10.0,
                filter_width,
                28.0,
            ),
            label,
            *window == *candidate,
        ) {
            *window = *candidate;
        }
    }

    let samples = telemetry_window(telemetry, *window);
    if samples.is_empty() {
        draw_text(
            "Coletando telemetria da sessão...",
            rect.x + 24.0,
            rect.y + 66.0,
            20.0,
            TEXT,
        );
        return;
    }
    let first = samples.first().expect("amostras não vazias");
    let last = samples.last().expect("amostras não vazias");
    draw_text(
        &format!(
            "{} amostras | ticks {} a {} | nasc. {} | mortes {}",
            samples.len(),
            first.tick,
            last.tick,
            last.births_total,
            last.deaths_total,
        ),
        rect.x + 20.0,
        rect.y + 54.0,
        14.0,
        MUTED,
    );

    let card_y = rect.y + 72.0;
    let card_gap = 12.0;
    let card_w = (rect.w - 40.0 - card_gap * 3.0) / 4.0;
    draw_dashboard_card(
        Rect::new(rect.x + 20.0, card_y, card_w, 54.0),
        "Células vivas",
        &last.population.to_string(),
        ACCENT,
    );
    draw_dashboard_card(
        Rect::new(rect.x + 20.0 + (card_w + card_gap), card_y, card_w, 54.0),
        "Variação na janela",
        &format!("{:+}", last.population as i64 - first.population as i64),
        if last.population >= first.population {
            ACCENT
        } else {
            RED
        },
    );
    draw_dashboard_card(
        Rect::new(
            rect.x + 20.0 + (card_w + card_gap) * 2.0,
            card_y,
            card_w,
            54.0,
        ),
        "Gerações",
        &format!(
            "máx {} · méd {:.1}",
            last.max_generation, last.average_generation
        ),
        Color::new(0.67, 0.46, 0.93, 1.0),
    );
    draw_dashboard_card(
        Rect::new(
            rect.x + 20.0 + (card_w + card_gap) * 3.0,
            card_y,
            card_w,
            54.0,
        ),
        "Diversidade genética",
        &format!("{:.1}%", last.genetic_diversity * 100.0),
        Color::new(0.31, 0.84, 0.62, 1.0),
    );

    let chart_x = rect.x + 20.0;
    let chart_w = rect.w - 40.0;
    let primary_y = card_y + 70.0;
    let primary_h = (rect.h * 0.22).clamp(125.0, 170.0);
    draw_line_chart(
        Rect::new(chart_x, primary_y, chart_w, primary_h),
        "População viva",
        &samples,
        |sample| sample.population as f32,
        ACCENT,
        "células",
    );

    let gap = 12.0;
    let secondary_y = primary_y + primary_h + gap;
    let secondary_h = (rect.h * 0.17).clamp(100.0, 125.0);
    let small_w = (chart_w - gap * 3.0) / 4.0;
    draw_line_chart(
        Rect::new(chart_x, secondary_y, small_w, secondary_h),
        "Energia média",
        &samples,
        |sample| sample.average_energy,
        Color::new(0.94, 0.69, 0.24, 1.0),
        "energia",
    );
    draw_energy_histogram(
        Rect::new(chart_x + small_w + gap, secondary_y, small_w, secondary_h),
        &data.energy_distribution,
        data.energy_bucket_width,
        Color::new(0.96, 0.51, 0.20, 1.0),
    );
    draw_line_chart(
        Rect::new(
            chart_x + (small_w + gap) * 2.0,
            secondary_y,
            small_w,
            secondary_h,
        ),
        "Idade média",
        &samples,
        |sample| sample.average_age,
        Color::new(0.38, 0.66, 0.96, 1.0),
        "ticks",
    );
    draw_line_chart(
        Rect::new(
            chart_x + (small_w + gap) * 3.0,
            secondary_y,
            small_w,
            secondary_h,
        ),
        "Diversidade genética",
        &samples,
        |sample| sample.genetic_diversity * 100.0,
        Color::new(0.31, 0.84, 0.62, 1.0),
        "%",
    );

    let rates_y = secondary_y + secondary_h + gap;
    let rates_h = (rect.y + rect.h - rates_y - 18.0).max(184.0);
    let family_w = (chart_w - gap) * 0.5;
    let within_family_gap = 8.0;
    let total_w = (family_w - within_family_gap) * 0.5;
    let cause_w = family_w - total_w - within_family_gap;
    let birth_cause_h = (rates_h - within_family_gap) * 0.5;
    let death_cause_h = (rates_h - within_family_gap * 2.0) / 3.0;
    // Como no esquema aprovado: natalidade à esquerda e mortalidade à
    // direita. Em cada família, o total ocupa uma carta alta; as causas
    // ficam empilhadas ao lado, com tons distintos da mesma cor-base.
    draw_event_rate_chart(
        Rect::new(chart_x, rates_y, total_w, rates_h),
        "Natalidade total",
        &samples,
        |sample| sample.births_total,
        Color::new(0.30, 0.85, 0.55, 1.0),
    );
    draw_event_rate_chart(
        Rect::new(
            chart_x + total_w + within_family_gap,
            rates_y,
            cause_w,
            birth_cause_h,
        ),
        "Divisão por decisão neural",
        &samples,
        |sample| sample.births_by_neural_decision_total,
        Color::new(0.18, 0.66, 0.43, 1.0),
    );
    draw_event_rate_chart(
        Rect::new(
            chart_x + total_w + within_family_gap,
            rates_y + birth_cause_h + within_family_gap,
            cause_w,
            birth_cause_h,
        ),
        "Divisão por limite energético",
        &samples,
        |sample| sample.births_by_energy_limit_total,
        Color::new(0.58, 0.88, 0.32, 1.0),
    );
    let deaths_x = chart_x + family_w + gap;
    draw_event_rate_chart(
        Rect::new(deaths_x, rates_y, total_w, rates_h),
        "Mortalidade total",
        &samples,
        |sample| sample.deaths_total,
        Color::new(0.93, 0.30, 0.35, 1.0),
    );
    draw_event_rate_chart(
        Rect::new(
            deaths_x + total_w + within_family_gap,
            rates_y,
            cause_w,
            death_cause_h,
        ),
        "Morte por idade",
        &samples,
        |sample| sample.deaths_by_age_total,
        Color::new(0.78, 0.24, 0.33, 1.0),
    );
    draw_event_rate_chart(
        Rect::new(
            deaths_x + total_w + within_family_gap,
            rates_y + death_cause_h + within_family_gap,
            cause_w,
            death_cause_h,
        ),
        "Morte por fome",
        &samples,
        |sample| sample.deaths_by_starvation_total,
        Color::new(0.93, 0.45, 0.22, 1.0),
    );
    draw_event_rate_chart(
        Rect::new(
            deaths_x + total_w + within_family_gap,
            rates_y + (death_cause_h + within_family_gap) * 2.0,
            cause_w,
            death_cause_h,
        ),
        "Morte por fagocitose",
        &samples,
        |sample| sample.deaths_by_phagocytosis_total,
        Color::new(0.89, 0.18, 0.47, 1.0),
    );
}

fn telemetry_window(
    samples: &[TelemetrySample],
    window: DemographicWindow,
) -> Vec<TelemetrySample> {
    let Some(last) = samples.last() else {
        return Vec::new();
    };
    let first_tick = samples
        .first()
        .map(|sample| sample.tick)
        .unwrap_or(last.tick);
    let start_tick = match window {
        DemographicWindow::Last5k => last.tick.saturating_sub(5_000),
        DemographicWindow::Last30k => last.tick.saturating_sub(30_000),
        DemographicWindow::Last100k => last.tick.saturating_sub(100_000),
        DemographicWindow::LastQuarter => last.tick.saturating_sub(last.tick / 4),
        DemographicWindow::EntireSession => first_tick,
    };
    samples
        .iter()
        .copied()
        .filter(|sample| sample.tick >= start_tick)
        .collect()
}

fn draw_dashboard_card(rect: Rect, label: &str, value: &str, color: Color) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, SURFACE);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    draw_text(label, rect.x + 10.0, rect.y + 18.0, 12.0, MUTED);
    draw_text(value, rect.x + 10.0, rect.y + 42.0, 21.0, color);
}

fn draw_energy_histogram(rect: Rect, counts: &[usize], bucket_width: f32, color: Color) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.04, 0.05, 0.07, 1.0),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    draw_text("Energia atual", rect.x + 12.0, rect.y + 20.0, 14.0, TEXT);
    if counts.is_empty() {
        return;
    }
    let maximum = counts.iter().copied().max().unwrap_or(1).max(1) as f32;
    let plot = Rect::new(rect.x + 28.0, rect.y + 32.0, rect.w - 40.0, rect.h - 53.0);
    for line in 0..=2 {
        let y = plot.y + plot.h * line as f32 / 2.0;
        draw_line(
            plot.x,
            y,
            plot.x + plot.w,
            y,
            1.0,
            Color::new(0.16, 0.19, 0.24, 1.0),
        );
    }
    let bar_width = (plot.w / counts.len() as f32 * 0.72).max(2.0);
    for (index, count) in counts.iter().enumerate() {
        let center_x = plot.x + (index as f32 + 0.5) * plot.w / counts.len() as f32;
        let height = *count as f32 / maximum * plot.h;
        draw_rectangle(
            center_x - bar_width / 2.0,
            plot.y + plot.h - height,
            bar_width,
            height,
            color,
        );
    }
    draw_text(
        &maximum.round().to_string(),
        rect.x + 4.0,
        plot.y + 4.0,
        9.0,
        MUTED,
    );
    draw_text("0", plot.x, rect.y + rect.h - 10.0, 9.0, MUTED);
    let energy_max = bucket_width * counts.len() as f32;
    let upper_label = format!("{energy_max:.0}");
    let label_width = measure_text(&upper_label, None, 9, 1.0).width;
    draw_text(
        &upper_label,
        plot.x + plot.w - label_width,
        rect.y + rect.h - 10.0,
        9.0,
        MUTED,
    );
}

fn draw_line_chart(
    rect: Rect,
    title: &str,
    samples: &[TelemetrySample],
    value_of: impl Fn(&TelemetrySample) -> f32,
    color: Color,
    unit: &str,
) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.04, 0.05, 0.07, 1.0),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    if rect.h < 76.0 {
        draw_text(title, rect.x + 7.0, rect.y + 13.0, 10.0, TEXT);
        if samples.len() < 2 {
            return;
        }
        let values: Vec<f32> = samples.iter().map(&value_of).collect();
        let minimum = values.iter().copied().fold(f32::INFINITY, f32::min);
        let maximum = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let span = (maximum - minimum).max(0.001);
        let plot = Rect::new(rect.x + 7.0, rect.y + 18.0, rect.w - 14.0, rect.h - 24.0);
        for (index, pair) in values.windows(2).enumerate() {
            let x1 = plot.x + index as f32 / (values.len() - 1).max(1) as f32 * plot.w;
            let x2 = plot.x + (index + 1) as f32 / (values.len() - 1).max(1) as f32 * plot.w;
            let y1 = plot.y + plot.h - (pair[0] - minimum) / span * plot.h;
            let y2 = plot.y + plot.h - (pair[1] - minimum) / span * plot.h;
            draw_line(x1, y1, x2, y2, 1.4, color);
        }
        return;
    }
    draw_text(title, rect.x + 12.0, rect.y + 20.0, 14.0, TEXT);
    if samples.is_empty() {
        return;
    }
    let plot = Rect::new(rect.x + 44.0, rect.y + 31.0, rect.w - 58.0, rect.h - 53.0);
    let mut minimum = samples.iter().map(&value_of).fold(f32::INFINITY, f32::min);
    let mut maximum = samples
        .iter()
        .map(&value_of)
        .fold(f32::NEG_INFINITY, f32::max);
    if (maximum - minimum).abs() < 0.000_1 {
        let padding = maximum.abs().max(1.0) * 0.12;
        minimum -= padding;
        maximum += padding;
    } else {
        let padding = (maximum - minimum) * 0.08;
        minimum -= padding;
        maximum += padding;
    }
    for step in 0..=2 {
        let fraction = step as f32 / 2.0;
        let y = plot.y + plot.h * fraction;
        draw_line(
            plot.x,
            y,
            plot.x + plot.w,
            y,
            1.0,
            Color::new(0.16, 0.19, 0.24, 1.0),
        );
        let value = maximum - (maximum - minimum) * fraction;
        let label = format_chart_value(value, unit);
        let label_width = measure_text(&label, None, 10, 1.0).width;
        draw_text(&label, plot.x - label_width - 6.0, y + 4.0, 10.0, MUTED);
    }
    let first_tick = samples.first().map(|sample| sample.tick).unwrap_or(0);
    let last_tick = samples
        .last()
        .map(|sample| sample.tick)
        .unwrap_or(first_tick + 1);
    let tick_span = last_tick.saturating_sub(first_tick).max(1) as f32;
    for pair in samples.windows(2) {
        let first = pair[0];
        let second = pair[1];
        let x1 = plot.x + (first.tick.saturating_sub(first_tick) as f32 / tick_span) * plot.w;
        let x2 = plot.x + (second.tick.saturating_sub(first_tick) as f32 / tick_span) * plot.w;
        let y1 = plot.y + plot.h - (value_of(&first) - minimum) / (maximum - minimum) * plot.h;
        let y2 = plot.y + plot.h - (value_of(&second) - minimum) / (maximum - minimum) * plot.h;
        draw_line(x1, y1, x2, y2, 2.0, color);
    }
    let tick_label = format!("{} a {}", first_tick, last_tick);
    draw_text(&tick_label, plot.x, rect.y + rect.h - 10.0, 10.0, MUTED);
    if let Some(last) = samples.last() {
        let label = format_chart_value(value_of(last), unit);
        let width = measure_text(&label, None, 11, 1.0).width;
        draw_text(
            &label,
            rect.x + rect.w - width - 10.0,
            rect.y + 20.0,
            11.0,
            color,
        );
    }
}

fn format_chart_value(value: f32, unit: &str) -> String {
    if value.abs() >= 1_000.0 {
        format!("{:.1}k {unit}", value / 1_000.0)
    } else if value.abs() >= 100.0 {
        format!("{value:.0} {unit}")
    } else {
        format!("{value:.1} {unit}")
    }
}

/// Taxa instantânea suavizada por cada intervalo de telemetria, expressa em
/// eventos por mil ticks. Usar deltas dos totais evita que o gráfico seja só
/// uma segunda cópia das curvas acumuladas de nascimento e morte.
fn draw_event_rate_chart(
    rect: Rect,
    title: &str,
    samples: &[TelemetrySample],
    total_of: impl Fn(&TelemetrySample) -> u64,
    color: Color,
) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.04, 0.05, 0.07, 1.0),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    if rect.h < 76.0 {
        draw_text(title, rect.x + 7.0, rect.y + 13.0, 10.0, TEXT);
        let total = samples.last().map(&total_of).unwrap_or(0);
        let total_text = format!("T {total}");
        let total_width = measure_text(&total_text, None, 9, 1.0).width;
        draw_text(
            &total_text,
            rect.x + rect.w - total_width - 7.0,
            rect.y + 13.0,
            9.0,
            color,
        );
        if samples.len() < 2 {
            return;
        }
        let rates: Vec<f32> = samples
            .windows(2)
            .map(|pair| {
                let ticks = pair[1].tick.saturating_sub(pair[0].tick).max(1) as f32;
                total_of(&pair[1]).saturating_sub(total_of(&pair[0])) as f32 / ticks
            })
            .collect();
        let maximum = rates.iter().copied().fold(0.0f32, f32::max).max(0.001);
        let plot = Rect::new(rect.x + 7.0, rect.y + 18.0, rect.w - 14.0, rect.h - 24.0);
        for (index, pair) in rates.windows(2).enumerate() {
            let x1 = plot.x + index as f32 / (rates.len() - 1).max(1) as f32 * plot.w;
            let x2 = plot.x + (index + 1) as f32 / (rates.len() - 1).max(1) as f32 * plot.w;
            let y1 = plot.y + plot.h - pair[0] / maximum * plot.h;
            let y2 = plot.y + plot.h - pair[1] / maximum * plot.h;
            draw_line(x1, y1, x2, y2, 1.4, color);
        }
        return;
    }
    draw_text(title, rect.x + 12.0, rect.y + 20.0, 14.0, TEXT);
    if samples.len() < 2 {
        draw_text(
            "Coletando intervalos...",
            rect.x + 12.0,
            rect.y + 45.0,
            13.0,
            MUTED,
        );
        return;
    }
    let rates: Vec<(u64, f32)> = samples
        .windows(2)
        .map(|pair| {
            let ticks = pair[1].tick.saturating_sub(pair[0].tick).max(1) as f32;
            let events = total_of(&pair[1]).saturating_sub(total_of(&pair[0])) as f32;
            (pair[1].tick, events / ticks * 1_000.0)
        })
        .collect();
    let maximum = rates
        .iter()
        .map(|(_, rate)| *rate)
        .fold(0.0f32, f32::max)
        .max(0.1);
    let plot = Rect::new(rect.x + 42.0, rect.y + 31.0, rect.w - 54.0, rect.h - 53.0);
    for step in 0..=2 {
        let fraction = step as f32 / 2.0;
        let y = plot.y + plot.h * fraction;
        draw_line(
            plot.x,
            y,
            plot.x + plot.w,
            y,
            1.0,
            Color::new(0.16, 0.19, 0.24, 1.0),
        );
        let label = format!("{:.1}", maximum * (1.0 - fraction));
        let width = measure_text(&label, None, 10, 1.0).width;
        draw_text(&label, plot.x - width - 6.0, y + 4.0, 10.0, MUTED);
    }
    let first_tick = rates.first().map(|(tick, _)| *tick).unwrap_or(0);
    let last_tick = rates
        .last()
        .map(|(tick, _)| *tick)
        .unwrap_or(first_tick + 1);
    let tick_span = last_tick.saturating_sub(first_tick).max(1) as f32;
    for pair in rates.windows(2) {
        let x1 = plot.x + (pair[0].0.saturating_sub(first_tick) as f32 / tick_span) * plot.w;
        let x2 = plot.x + (pair[1].0.saturating_sub(first_tick) as f32 / tick_span) * plot.w;
        let y1 = plot.y + plot.h - pair[0].1 / maximum * plot.h;
        let y2 = plot.y + plot.h - pair[1].1 / maximum * plot.h;
        draw_line(x1, y1, x2, y2, 2.0, color);
    }
    draw_text(
        "eventos / 1k ticks",
        plot.x,
        rect.y + rect.h - 10.0,
        10.0,
        MUTED,
    );
    let last_rate = rates.last().map(|(_, rate)| *rate).unwrap_or(0.0);
    let total = samples.last().map(&total_of).unwrap_or(0);
    let value = format!("{last_rate:.2} · T {total}");
    let width = measure_text(&value, None, 11, 1.0).width;
    draw_text(
        &value,
        rect.x + rect.w - width - 10.0,
        rect.y + 20.0,
        11.0,
        color,
    );
}

fn draw_binary_genome(rect: Rect, binary: &str) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.045, 0.055, 0.075, 1.0),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    draw_text(
        &format!(
            "{} bits · {} registros neurais",
            GENOME_BIT_COUNT, CONNECTION_RECORDS
        ),
        rect.x + 10.0,
        rect.y + 17.0,
        12.0,
        MUTED,
    );

    let chunks: Vec<_> = binary.as_bytes().chunks(48).collect();
    let line_height = ((rect.h - 38.0) / chunks.len().max(1) as f32).clamp(6.0, 10.0);
    let font_size = if line_height < 8.0 { 7.0 } else { 9.0 };
    for (line, chunk) in chunks.into_iter().enumerate() {
        let mut grouped = String::with_capacity(53);
        for (index, byte) in chunk.iter().enumerate() {
            if index > 0 && index % 8 == 0 {
                grouped.push(' ');
            }
            grouped.push(*byte as char);
        }
        draw_text(
            &grouped,
            rect.x + 10.0,
            rect.y + 32.0 + line as f32 * line_height,
            font_size,
            Color::new(0.66, 0.84, 0.73, 1.0),
        );
    }
}

fn draw_brain_viewer(rect: Rect, organism: &Organism) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.045, 0.055, 0.075, 1.0),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    let mut positions = [Vec2::ZERO; SLOT_COUNT];
    let node_top = rect.y + 34.0;
    let node_bottom = rect.y + rect.h - 20.0;
    let node_radius = (rect.h / 44.0).clamp(4.5, 9.0);
    let label_size = if rect.h < 270.0 { 7 } else { 9 };
    for slot in 0..EXTERNAL_SLOT_COUNT {
        let column = slot / 12;
        let row = slot % 12;
        positions[slot] = vec2(
            rect.x + 22.0 + column as f32 * 38.0,
            node_top + (node_bottom - node_top) * row as f32 / 11.0,
        );
    }
    // Os 16 internos rápidos ocupam um anel oval: conexões entre eles deixam
    // de formar uma coluna indecifrável e passam a expor ciclos e atalhos.
    let internal_center = vec2(rect.x + rect.w * 0.55, (node_top + node_bottom) * 0.5);
    let internal_radius_x = (rect.w * 0.18).clamp(34.0, 86.0);
    let internal_radius_y = ((node_bottom - node_top) * 0.38).clamp(30.0, 112.0);
    for slot in INTERNAL_START..INTERNAL_START + TIMER_INTERNAL_START {
        let angle = -std::f32::consts::FRAC_PI_2
            + (slot - INTERNAL_START) as f32 / TIMER_INTERNAL_START as f32 * std::f32::consts::TAU;
        positions[slot] = vec2(
            internal_center.x + angle.cos() * internal_radius_x,
            internal_center.y + angle.sin() * internal_radius_y,
        );
    }
    // Os temporizadores ficam no miolo: eles são internos, mas não competem
    // visualmente com os ciclos rápidos do anel.
    for slot in INTERNAL_START + TIMER_INTERNAL_START..MOTOR_START {
        positions[slot] = vec2(
            internal_center.x,
            internal_center.y
                + (slot - (INTERNAL_START + TIMER_INTERNAL_START + 1)) as f32 * node_radius * 2.45,
        );
    }
    for slot in MOTOR_START..SLOT_COUNT {
        positions[slot] = vec2(
            rect.x + rect.w - 31.0,
            node_top
                + (node_bottom - node_top) * (slot - MOTOR_START + 1) as f32
                    / (SLOT_COUNT - MOTOR_START + 1) as f32,
        );
    }
    for (connection, signal) in organism.brain.visual_connections() {
        let magnitude = (signal.abs() / 3.0).clamp(0.0, 1.0);
        let color = if signal > 0.0 {
            Color::new(0.18, 0.92, 0.40, 0.20 + 0.75 * magnitude)
        } else if signal < 0.0 {
            Color::new(0.93, 0.20, 0.25, 0.20 + 0.75 * magnitude)
        } else {
            // Mesmo sem sinal neste tick, a ligação estrutural continua
            // visível. A espessura codifica a força genética da conexão.
            Color::new(0.43, 0.55, 0.72, 0.52)
        };
        let structural_weight = connection.weight.abs().clamp(0.0, 1.0);
        let width = 1.0 + 2.0 * magnitude.max(structural_weight * 0.45);
        let internal_link = (INTERNAL_START..MOTOR_START).contains(&connection.source)
            && (INTERNAL_START..MOTOR_START).contains(&connection.destination);
        if internal_link {
            signal_curve(
                positions[connection.source],
                positions[connection.destination],
                connection.source + connection.destination,
                color,
                width,
            );
        } else {
            signal_arrow(
                positions[connection.source],
                positions[connection.destination],
                color,
                width,
            );
        }
    }
    for slot in 0..SLOT_COUNT {
        let position = positions[slot];
        let value = organism.brain.slot_value(slot);
        let node_color = if slot >= MOTOR_START {
            Color::new(0.08, 0.30 + 0.70 * value, 0.22 + 0.48 * value, 1.0)
        } else if value >= 0.0 {
            Color::new(0.15, 0.34 + 0.42 * value, 0.55 + 0.26 * value, 1.0)
        } else {
            Color::new(0.58 + 0.30 * -value, 0.18, 0.25, 1.0)
        };
        draw_circle(position.x, position.y, node_radius, node_color);
        draw_circle_lines(
            position.x,
            position.y,
            node_radius,
            1.0,
            Color::new(0.01, 0.02, 0.03, 1.0),
        );
        let label = slot_label(slot);
        let measured = measure_text(label, None, label_size, 1.0);
        draw_text(
            label,
            position.x - measured.width / 2.0,
            position.y + label_size as f32 * 0.34,
            label_size as f32,
            TEXT,
        );
    }
    let mouse = vec2(mouse_position().0, mouse_position().1);
    if let Some(slot) = (0..SLOT_COUNT)
        .filter(|slot| positions[*slot].distance(mouse) <= node_radius + 4.0)
        .min_by(|left, right| {
            positions[*left]
                .distance_squared(mouse)
                .total_cmp(&positions[*right].distance_squared(mouse))
        })
    {
        draw_neuron_tooltip(mouse, slot);
    }
    draw_text("sensores", rect.x + 8.0, rect.y + 18.0, 12.0, MUTED);
    draw_text(
        "internos",
        rect.x + rect.w * 0.53 - 24.0,
        rect.y + 18.0,
        12.0,
        MUTED,
    );
    draw_text(
        "motores",
        rect.x + rect.w - 60.0,
        rect.y + 18.0,
        12.0,
        MUTED,
    );
}

fn signal_arrow(start: Vec2, end: Vec2, color: Color, width: f32) {
    let direction = (end - start).normalize_or_zero();
    let line_start = start + direction * 11.0;
    let line_end = end - direction * 13.0;
    draw_line(
        line_start.x,
        line_start.y,
        line_end.x,
        line_end.y,
        width,
        color,
    );
    draw_triangle(
        line_end,
        line_end - direction * 7.0 + direction.rotate(vec2(0.0, 1.0)) * 4.0,
        line_end - direction * 7.0 - direction.rotate(vec2(0.0, 1.0)) * 4.0,
        color,
    );
}

/// Curva quadrática curta para conexões internas. A curvatura alternada
/// separa visualmente ligações recíprocas e evita que todas passem pelo centro.
fn signal_curve(start: Vec2, end: Vec2, key: usize, color: Color, width: f32) {
    let direction = end - start;
    let length = direction.length();
    if length < 1.0 {
        draw_circle_lines(start.x, start.y, 12.0, width, color);
        return;
    }
    let normal = vec2(-direction.y, direction.x).normalize_or_zero();
    let curvature_sign = if key % 2 == 0 { 1.0 } else { -1.0 };
    let control = (start + end) * 0.5 + normal * (length * 0.18).clamp(10.0, 32.0) * curvature_sign;
    let point = |t: f32| {
        let inverse = 1.0 - t;
        start * (inverse * inverse) + control * (2.0 * inverse * t) + end * (t * t)
    };
    let mut previous = point(0.0);
    for step in 1..=10 {
        let current = point(step as f32 / 10.0);
        draw_line(previous.x, previous.y, current.x, current.y, width, color);
        previous = current;
    }
    let arrow_end = point(0.92);
    let arrow_tip = point(0.98);
    let tangent = (arrow_tip - arrow_end).normalize_or_zero();
    draw_triangle(
        arrow_tip,
        arrow_tip - tangent * 7.0 + tangent.rotate(vec2(0.0, 1.0)) * 4.0,
        arrow_tip - tangent * 7.0 - tangent.rotate(vec2(0.0, 1.0)) * 4.0,
        color,
    );
}

fn slot_label(slot: usize) -> &'static str {
    [
        "F<", "F>", "F^", "Fv", "Cx", "Cy", "En", "+1", "Ct", "PG", "PGx", "PGy", "P1", "P1x",
        "P1y", "P2", "P2x", "P2y", "P3", "P3x", "P3y", "Fx", "Fy", "I0", "I1", "I2", "I3", "I4",
        "I5", "I6", "I7", "I8", "I9", "I10", "I11", "I12", "I13", "I14", "I15", "T1", "T2", "T3",
        "L", "R", "U", "D", "Re", "Fa", "EPG", "EP1", "EP2", "EP3", "Do",
    ][slot]
}

fn neuron_name(slot: usize) -> String {
    match slot {
        0 => "Sensor · alimento à esquerda".to_owned(),
        1 => "Sensor · alimento à direita".to_owned(),
        2 => "Sensor · alimento acima".to_owned(),
        3 => "Sensor · alimento abaixo".to_owned(),
        4 => "Sensor · corrente no eixo X".to_owned(),
        5 => "Sensor · corrente no eixo Y".to_owned(),
        6 => "Sensor · energia relativa".to_owned(),
        7 => "Sensor · sinal constante".to_owned(),
        8 => "Sensor · contato com outra célula".to_owned(),
        9 => "Sensor · intensidade de PG (assinatura genética)".to_owned(),
        10 => "Sensor · gradiente X de PG".to_owned(),
        11 => "Sensor · gradiente Y de PG".to_owned(),
        12 => "Sensor · intensidade de P1".to_owned(),
        13 => "Sensor · gradiente X de P1".to_owned(),
        14 => "Sensor · gradiente Y de P1".to_owned(),
        15 => "Sensor · intensidade de P2".to_owned(),
        16 => "Sensor · gradiente X de P2".to_owned(),
        17 => "Sensor · gradiente Y de P2".to_owned(),
        18 => "Sensor · intensidade de P3".to_owned(),
        19 => "Sensor · gradiente X de P3".to_owned(),
        20 => "Sensor · gradiente Y de P3".to_owned(),
        21 => "Sensor · força externa no eixo X".to_owned(),
        22 => "Sensor · força externa no eixo Y".to_owned(),
        23..=38 => format!("Neurônio interno rápido I{}", slot - INTERNAL_START),
        39..=41 => format!("Neurônio temporizador T{}", slot - 38),
        42 => "Motor · nadar à esquerda".to_owned(),
        43 => "Motor · nadar à direita".to_owned(),
        44 => "Motor · nadar acima".to_owned(),
        45 => "Motor · nadar abaixo".to_owned(),
        46 => "Motor · reproduzir".to_owned(),
        47 => "Motor · fagocitar".to_owned(),
        48 => "Motor · reforçar emissão de PG".to_owned(),
        49 => "Motor · emitir P1".to_owned(),
        50 => "Motor · emitir P2".to_owned(),
        51 => "Motor · emitir P3".to_owned(),
        52 => "Motor · doar energia à célula em contato".to_owned(),
        _ => "Neurônio desconhecido".to_owned(),
    }
}

fn draw_neuron_tooltip(mouse: Vec2, slot: usize) {
    let title = format!("{} · {}", slot_label(slot), neuron_name(slot));
    let font_size = 14;
    let padding = 10.0;
    let width = measure_text(&title, None, font_size, 1.0).width + padding * 2.0;
    let height = 31.0;
    let x = if mouse.x + width + 18.0 <= screen_width() {
        mouse.x + 14.0
    } else {
        (mouse.x - width - 14.0).max(4.0)
    };
    let y = (mouse.y - height - 10.0).clamp(4.0, screen_height() - height - 4.0);
    draw_rectangle(x, y, width, height, Color::new(0.015, 0.022, 0.035, 0.96));
    draw_rectangle_lines(x, y, width, height, 1.0, ACCENT);
    draw_text(&title, x + padding, y + 20.5, font_size as f32, TEXT);
}

fn ensure_selected_cell(session: &mut SessionUi, world: &World) {
    if selected_index(session.selected_cell_id, &world.organisms).is_none() {
        session.selected_cell_id = world.organisms.first().map(|organism| organism.id);
    }
}

fn selected_index(selected_id: Option<u64>, organisms: &[Organism]) -> Option<usize> {
    selected_id.and_then(|id| organisms.iter().position(|organism| organism.id == id))
}

fn checkbox_row(rect: Rect, label: &str, checked: bool) -> bool {
    let (mouse_x, mouse_y) = mouse_position();
    let hovered = rect.contains(vec2(mouse_x, mouse_y));
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hovered { SURFACE_HOVER } else { SURFACE },
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    let check_rect = Rect::new(rect.x + 10.0, rect.y + 8.0, 16.0, 16.0);
    draw_rectangle(
        check_rect.x,
        check_rect.y,
        check_rect.w,
        check_rect.h,
        if checked { ACCENT } else { BACKGROUND },
    );
    draw_rectangle_lines(
        check_rect.x,
        check_rect.y,
        check_rect.w,
        check_rect.h,
        1.0,
        if checked { ACCENT } else { MUTED },
    );
    if checked {
        draw_line(
            check_rect.x + 3.0,
            check_rect.y + 8.0,
            check_rect.x + 7.0,
            check_rect.y + 12.0,
            1.8,
            BACKGROUND,
        );
        draw_line(
            check_rect.x + 7.0,
            check_rect.y + 12.0,
            check_rect.x + 14.0,
            check_rect.y + 4.0,
            1.8,
            BACKGROUND,
        );
    }
    draw_text(label, rect.x + 36.0, rect.y + 21.0, 14.0, TEXT);
    hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn editable_number_row(rect: Rect, label: &str, value: &str, editing: bool, dirty: bool) -> bool {
    let (mouse_x, mouse_y) = mouse_position();
    let hovered = rect.contains(vec2(mouse_x, mouse_y));
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if editing { SURFACE_HOVER } else { SURFACE },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if dirty {
            RED
        } else if editing {
            ACCENT
        } else {
            BORDER
        },
    );
    draw_text(label, rect.x + 10.0, rect.y + 24.0, 15.0, MUTED);
    let displayed = if value.is_empty() {
        "digite um valor"
    } else {
        value
    };
    let label_width = measure_text(label, None, 15, 1.0).width;
    let value_x = rect.x + label_width + 24.0;
    draw_text(
        displayed,
        value_x,
        rect.y + 24.0,
        15.0,
        if dirty { RED } else { TEXT },
    );
    if editing {
        let cursor_x = value_x + 2.0 + measure_text(displayed, None, 15, 1.0).width;
        draw_line(cursor_x, rect.y + 9.0, cursor_x, rect.y + 29.0, 1.0, ACCENT);
    }
    hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn begin_editing(session: &mut SessionUi, field: EditingField) {
    if session.editing_field == Some(field) {
        return;
    }
    session.editing_field = Some(field);
    match field {
        EditingField::Seed => session.seed_input.clear(),
        EditingField::InitialCells => session.initial_cells_input.clear(),
        EditingField::ReproductionEnergy => session.reproduction_energy_input.clear(),
        EditingField::ReproductionLimit => session.reproduction_limit_input.clear(),
        EditingField::DesertEmission => session.desert_emission_input.clear(),
        EditingField::FertileEmission => session.fertile_emission_input.clear(),
    }
}

fn update_numeric_input(session: &mut SessionUi) {
    let Some(field) = session.editing_field else {
        return;
    };
    let input = match field {
        EditingField::Seed => &mut session.seed_input,
        EditingField::InitialCells => &mut session.initial_cells_input,
        EditingField::ReproductionEnergy => &mut session.reproduction_energy_input,
        EditingField::ReproductionLimit => &mut session.reproduction_limit_input,
        EditingField::DesertEmission => &mut session.desert_emission_input,
        EditingField::FertileEmission => &mut session.fertile_emission_input,
    };
    while let Some(character) = get_char_pressed() {
        let accepts_float = matches!(
            field,
            EditingField::ReproductionEnergy
                | EditingField::ReproductionLimit
                | EditingField::DesertEmission
                | EditingField::FertileEmission
        );
        let allowed = character.is_ascii_digit()
            || (accepts_float && matches!(character, '.' | 'e' | 'E' | '+' | '-'));
        if allowed && input.len() < 24 {
            input.push(character);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        input.pop();
    }
    if is_key_pressed(KeyCode::Enter) {
        session.editing_field = None;
    }
}

fn commit_field(session: &mut SessionUi, field: EditingField) -> bool {
    match field {
        EditingField::Seed => {
            if let Ok(seed) = session.seed_input.parse::<u64>() {
                session.seed = seed;
                session.seed_input = seed.to_string();
                true
            } else {
                false
            }
        }
        EditingField::InitialCells => {
            if let Ok(initial_cells) = session.initial_cells_input.parse::<usize>() {
                session.initial_cells = initial_cells;
                session.initial_cells_input = initial_cells.to_string();
                true
            } else {
                false
            }
        }
        EditingField::ReproductionEnergy => {
            if let Ok(energy) = session.reproduction_energy_input.parse::<f32>() {
                if energy.is_finite() {
                    session.reproduction_energy = energy;
                    session.reproduction_energy_input = energy.to_string();
                    return true;
                }
            }
            false
        }
        EditingField::ReproductionLimit => {
            if let Ok(limit) = session.reproduction_limit_input.parse::<f32>() {
                if limit.is_finite() {
                    session.reproduction_limit = limit.max(session.reproduction_energy);
                    session.reproduction_limit_input = session.reproduction_limit.to_string();
                    return true;
                }
            }
            false
        }
        EditingField::DesertEmission => {
            if let Ok(emission) = session.desert_emission_input.parse::<f32>() {
                if emission.is_finite() && emission >= 0.0 {
                    session.desert_emission = emission_from_display(emission);
                    session.desert_emission_input =
                        format_emission_display(session.desert_emission);
                    return true;
                }
            }
            false
        }
        EditingField::FertileEmission => {
            if let Ok(emission) = session.fertile_emission_input.parse::<f32>() {
                if emission.is_finite() && emission >= 0.0 {
                    session.fertile_emission = emission_from_display(emission);
                    session.fertile_emission_input =
                        format_emission_display(session.fertile_emission);
                    return true;
                }
            }
            false
        }
    }
}

fn commit_all_numeric_inputs(session: &mut SessionUi) -> bool {
    [
        EditingField::Seed,
        EditingField::InitialCells,
        EditingField::ReproductionEnergy,
        EditingField::ReproductionLimit,
        EditingField::DesertEmission,
        EditingField::FertileEmission,
    ]
    .into_iter()
    .fold(false, |changed, field| {
        commit_field(session, field) || changed
    })
}

fn stepper_row(
    x: f32,
    y: f32,
    width: f32,
    label: &str,
    value: &str,
    slider_fraction: Option<f32>,
    dirty: bool,
) -> Option<i8> {
    draw_rectangle(x, y, width, 38.0, SURFACE);
    draw_text(
        label,
        x + 10.0,
        y + 24.0,
        15.0,
        if dirty { RED } else { MUTED },
    );
    if let Some(fraction) = slider_fraction {
        let track_x = x + 84.0;
        let track_w = 54.0;
        let fraction = fraction.clamp(0.0, 1.0);
        draw_line(track_x, y + 21.0, track_x + track_w, y + 21.0, 3.0, BORDER);
        draw_line(
            track_x,
            y + 21.0,
            track_x + track_w * fraction,
            y + 21.0,
            3.0,
            ACCENT,
        );
        draw_circle(track_x + track_w * fraction, y + 21.0, 4.0, ACCENT);
        draw_text(
            value,
            x + 148.0,
            y + 24.0,
            14.0,
            if dirty { RED } else { TEXT },
        );
    } else {
        draw_text(
            value,
            x + 108.0,
            y + 24.0,
            14.0,
            if dirty { RED } else { TEXT },
        );
    }
    if button(Rect::new(x + width - 74.0, y + 4.0, 32.0, 30.0), "-", false) {
        Some(-1)
    } else if button(Rect::new(x + width - 36.0, y + 4.0, 32.0, 30.0), "+", false) {
        Some(1)
    } else {
        None
    }
}

fn slider_row(
    x: f32,
    y: f32,
    width: f32,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    value_label: &str,
    dirty: bool,
) -> Option<f32> {
    draw_rectangle(x, y, width, 38.0, SURFACE);
    let active_color = if dirty { RED } else { ACCENT };
    draw_text(
        label,
        x + 10.0,
        y + 24.0,
        15.0,
        if dirty { RED } else { MUTED },
    );
    let track_x = x + 92.0;
    let track_width = width - 172.0;
    let fraction = ((value - min) / (max - min)).clamp(0.0, 1.0);
    draw_line(
        track_x,
        y + 21.0,
        track_x + track_width,
        y + 21.0,
        4.0,
        BORDER,
    );
    draw_line(
        track_x,
        y + 21.0,
        track_x + track_width * fraction,
        y + 21.0,
        4.0,
        active_color,
    );
    draw_circle(
        track_x + track_width * fraction,
        y + 21.0,
        6.0,
        active_color,
    );
    draw_text(
        value_label,
        x + width - 66.0,
        y + 24.0,
        14.0,
        if dirty { RED } else { TEXT },
    );

    let (mouse_x, mouse_y) = mouse_position();
    let hit_area = Rect::new(track_x - 7.0, y + 4.0, track_width + 14.0, 34.0);
    if hit_area.contains(vec2(mouse_x, mouse_y)) && is_mouse_button_down(MouseButton::Left) {
        Some(min + ((mouse_x - track_x) / track_width).clamp(0.0, 1.0) * (max - min))
    } else {
        None
    }
}

fn section_title(x: f32, y: f32, text: &str) {
    draw_text(text, x, y, 16.0, TEXT);
    draw_line(x, y + 8.0, x + PANEL_WIDTH - 24.0, y + 8.0, 1.0, BORDER);
}

fn draw_value_row(x: f32, y: f32, width: f32, label: &str, value: &str) {
    draw_rectangle(x, y, width, 34.0, SURFACE);
    draw_text(label, x + 10.0, y + 22.0, 14.0, MUTED);
    let measured = measure_text(value, None, 14, 1.0);
    draw_text(
        value,
        x + width - measured.width - 12.0,
        y + 22.0,
        14.0,
        TEXT,
    );
}

fn speed_label(index: usize) -> &'static str {
    ["1×", "10×", "100×", "1k×", "10k×", "100k×", "1M×"][index]
}

fn button(rect: Rect, label: &str, selected: bool) -> bool {
    button_dirty(rect, label, selected, false)
}

/// Botão realmente indisponível: além de parecer apagado, não reage ao
/// clique. É usado para Reverter quando nada foi alterado.
fn button_enabled(rect: Rect, label: &str, enabled: bool) -> bool {
    if !enabled {
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.052, 0.062, 0.082, 1.0),
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::new(0.12, 0.14, 0.18, 1.0),
        );
        let measured = measure_text(label, None, 14, 1.0);
        draw_text(
            label,
            rect.x + (rect.w - measured.width) / 2.0,
            rect.y + (rect.h + measured.height) / 2.0 - 2.0,
            14.0,
            Color::new(0.34, 0.38, 0.44, 1.0),
        );
        false
    } else {
        button(rect, label, false)
    }
}

fn button_dirty(rect: Rect, label: &str, selected: bool, dirty: bool) -> bool {
    let (mouse_x, mouse_y) = mouse_position();
    let hovered = rect.contains(vec2(mouse_x, mouse_y));
    let color = if selected {
        ACCENT
    } else if hovered {
        SURFACE_HOVER
    } else {
        SURFACE
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if dirty {
            RED
        } else if selected {
            ACCENT
        } else {
            BORDER
        },
    );
    let measured = measure_text(label, None, 14, 1.0);
    draw_text(
        label,
        rect.x + (rect.w - measured.width) / 2.0,
        rect.y + (rect.h + measured.height) / 2.0 - 2.0,
        14.0,
        if selected && !dirty {
            BACKGROUND
        } else if dirty {
            RED
        } else {
            TEXT
        },
    );
    hovered && is_mouse_button_pressed(MouseButton::Left)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_toggle_overrides_the_slider_without_losing_its_value() {
        let mut session = SessionUi::default();
        session.flow_intensity = 0.63;
        assert_eq!(session.effective_flow_intensity(), 0.63);

        session.flow_enabled = false;
        assert_eq!(session.effective_flow_intensity(), 0.0);
        assert_eq!(session.flow_intensity, 0.63);

        session.flow_enabled = true;
        assert_eq!(session.effective_flow_intensity(), 0.63);
    }

    #[test]
    fn mental_cost_slider_uses_human_scale_without_changing_engine_value() {
        assert_eq!(mental_cost_display(DEFAULT_MENTAL_ENERGY_COST), 100.0);
        assert!((mental_cost_from_display(100.0) - DEFAULT_MENTAL_ENERGY_COST).abs() < 0.000_001);
    }

    #[test]
    fn desert_emission_field_accepts_tiny_scientific_values() {
        let mut session = SessionUi::default();
        session.desert_emission_input = "1e-8".to_owned();

        assert!(commit_field(&mut session, EditingField::DesertEmission));
        assert!(
            (session.desert_emission - 0.000_000_01 * EMISSION_DISPLAY_UNIT).abs() < f32::EPSILON
        );
    }
}
