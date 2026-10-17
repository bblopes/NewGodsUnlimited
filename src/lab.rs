use std::{
    fs,
    io::{self, Cursor, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use macroquad::prelude::*;

use crate::{
    config::{WORLD_HEIGHT, WORLD_WIDTH},
    environment::{
        flow::{FlowField, FlowPreset},
        map::{MAP_GRID_HEIGHT, MAP_GRID_WIDTH, MapDefinition},
    },
    simulation::{
        genome::{GENOME_BYTES, Genome, PRE_FOOD_SENSOR_RANGE_GENOME_BYTES},
        world::{
            AntiCrammingConfig, DEFAULT_MENTAL_ENERGY_COST, FOOD_ENERGY,
            InitialNeuralConfiguration, MAX_ENERGY, World,
        },
    },
};

const PANEL_WIDTH: f32 = 360.0;
const BACKGROUND: Color = Color::new(0.035, 0.043, 0.059, 1.0);
const SURFACE: Color = Color::new(0.075, 0.09, 0.12, 1.0);
const BORDER: Color = Color::new(0.20, 0.235, 0.30, 1.0);
const TEXT: Color = Color::new(0.91, 0.93, 0.96, 1.0);
const MUTED: Color = Color::new(0.56, 0.62, 0.70, 1.0);
const ACCENT: Color = Color::new(0.20, 0.66, 0.93, 1.0);
const MAX_SNAPSHOTS: u64 = 120;
const MAX_CAPTURED_CELLS: usize = 3_500;
const MAX_CAPTURED_FOOD: usize = 2_500;
const PRESET_MAGIC: [u8; 8] = *b"ECOPRE01";
const PRESET_FORMAT_VERSION: u32 = 2;
const MAX_PRESET_GENOMES: usize = 20_000;
static SEED_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, PartialEq, Eq)]
enum CampaignMode {
    SeedSweep,
    EnvironmentStress,
}

impl CampaignMode {
    fn label(self) -> &'static str {
        match self {
            Self::SeedSweep => "Varredura de seeds",
            Self::EnvironmentStress => "Estresse ambiental",
        }
    }

    fn to_byte(self) -> u8 {
        match self {
            Self::SeedSweep => 0,
            Self::EnvironmentStress => 1,
        }
    }

    fn from_byte(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::SeedSweep),
            1 => Some(Self::EnvironmentStress),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TerminationCriterion {
    Ticks,
    Generation,
    Extinction,
    Population,
    Births,
}

impl TerminationCriterion {
    fn to_byte(self) -> u8 {
        match self {
            Self::Ticks => 0,
            Self::Generation => 1,
            Self::Extinction => 2,
            Self::Population => 3,
            Self::Births => 4,
        }
    }

    fn from_byte(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Ticks),
            1 => Some(Self::Generation),
            2 => Some(Self::Extinction),
            3 => Some(Self::Population),
            4 => Some(Self::Births),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct CampaignConfig {
    mode: CampaignMode,
    source_seed: Option<u64>,
    worlds: usize,
    initial_cells: usize,
    termination: TerminationCriterion,
    termination_target: u64,
    max_ticks: u64,
    /// Condição opcional e independente dos ticks máximos.
    stop_on_extinction: bool,
    emission: f32,
    food_lifetime: u32,
    food_energy: f32,
    reproduction_energy: f32,
    flow_intensity: f32,
    vary_emission: bool,
    emission_min: f32,
    emission_max: f32,
    vary_food_lifetime: bool,
    food_lifetime_min: u32,
    food_lifetime_max: u32,
    vary_food_energy: bool,
    food_energy_min: f32,
    food_energy_max: f32,
    vary_flow_intensity: bool,
    flow_intensity_min: f32,
    flow_intensity_max: f32,
    fertile_emission: f32,
    reproduction_limit: f32,
    mental_energy_cost: f32,
    flow_preset: FlowPreset,
    anti_cramming: AntiCrammingConfig,
    map: MapDefinition,
    initial_neural_configuration: InitialNeuralConfiguration,
    validated_genomes: Vec<Genome>,
    amadeus_mode: bool,
    imported_preset_name: Option<String>,
    preset_feedback: Option<String>,
}

impl Default for CampaignConfig {
    fn default() -> Self {
        Self {
            mode: CampaignMode::SeedSweep,
            source_seed: None,
            worlds: 10,
            initial_cells: 1_000,
            termination: TerminationCriterion::Ticks,
            termination_target: 50_000,
            max_ticks: 50_000,
            stop_on_extinction: false,
            emission: 1.0,
            food_lifetime: 600,
            food_energy: FOOD_ENERGY,
            reproduction_energy: MAX_ENERGY,
            flow_intensity: 1.0,
            vary_emission: true,
            emission_min: 0.20,
            emission_max: 3.00,
            vary_food_lifetime: true,
            food_lifetime_min: 300,
            food_lifetime_max: 4_000,
            vary_food_energy: false,
            food_energy_min: FOOD_ENERGY,
            food_energy_max: FOOD_ENERGY,
            vary_flow_intensity: true,
            flow_intensity_min: 0.20,
            flow_intensity_max: 1.0,
            fertile_emission: 2.0,
            reproduction_limit: MAX_ENERGY * 5.0,
            mental_energy_cost: DEFAULT_MENTAL_ENERGY_COST,
            flow_preset: FlowPreset::Oasis,
            anti_cramming: AntiCrammingConfig::default(),
            map: MapDefinition::default(),
            initial_neural_configuration: InitialNeuralConfiguration::Classic,
            validated_genomes: Vec::new(),
            amadeus_mode: false,
            imported_preset_name: None,
            preset_feedback: None,
        }
    }
}

#[derive(Clone, Copy)]
struct EffectiveEnvironment {
    emission: f32,
    food_lifetime: u32,
    food_energy: f32,
    flow_intensity: f32,
}

struct RunPlan {
    world_index: usize,
    seed: u64,
    environment: EffectiveEnvironment,
}

#[derive(Clone)]
struct CellFrame {
    id: u64,
    position: Vec2,
    energy: f32,
    age_ticks: u64,
    generation: u32,
    children: u32,
    connections: usize,
    color: [u8; 3],
}

#[derive(Clone)]
struct WorldFrame {
    tick: u64,
    cells: Vec<CellFrame>,
    food: Vec<Vec2>,
    population: usize,
    births: u64,
    average_energy: f32,
    average_generation: f32,
}

#[derive(Clone)]
struct WorldResult {
    world_index: usize,
    seed: u64,
    frames: Vec<WorldFrame>,
    final_population: usize,
    births: u64,
    average_energy: f32,
    average_age: f32,
    average_generation: f32,
    average_connections: f32,
    ended_at_tick: u64,
    ended_by: String,
    environment: EffectiveEnvironment,
}

#[derive(Clone)]
struct StoredCampaign {
    name: String,
    saved_at: u64,
    config: CampaignConfig,
    results: Vec<WorldResult>,
}

enum WorkerMessage {
    Progress { world_index: usize, tick: u64 },
    Complete(WorldResult),
}

enum Screen {
    Home,
    Setup,
    Running,
    Results,
    Library,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Session,
    Cells,
    Demographics,
}

struct Playback {
    world_index: usize,
    frame_index: usize,
    selected_cell_id: Option<u64>,
    playing: bool,
    frame_credit: f32,
    tab: Tab,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            world_index: 0,
            frame_index: 0,
            selected_cell_id: None,
            playing: true,
            frame_credit: 0.0,
            tab: Tab::Session,
        }
    }
}

pub async fn run() {
    let mut screen = Screen::Home;
    let mut config = CampaignConfig::default();
    let mut receiver: Option<Receiver<WorkerMessage>> = None;
    let mut progress = Vec::new();
    let mut results = Vec::new();
    let mut playback = Playback::default();
    let mut active_campaign: Option<StoredCampaign> = None;
    let mut active_campaign_is_saved = false;
    let mut saved_paths = saved_campaign_paths();

    loop {
        clear_background(BACKGROUND);
        match screen {
            Screen::Home => match draw_home(saved_paths.len()) {
                HomeAction::SeedSweep => {
                    config = CampaignConfig::default();
                    config.mode = CampaignMode::SeedSweep;
                    screen = Screen::Setup;
                }
                HomeAction::EnvironmentStress => {
                    config.mode = CampaignMode::EnvironmentStress;
                    config.source_seed = None;
                    screen = Screen::Library;
                }
                HomeAction::Library => {
                    saved_paths = saved_campaign_paths();
                    screen = Screen::Library;
                }
                HomeAction::None => {}
            },
            Screen::Setup => {
                draw_setup_header();
                if draw_setup(&mut config) {
                    if config.mode == CampaignMode::EnvironmentStress
                        && config.source_seed.is_none()
                    {
                        screen = Screen::Library;
                        continue;
                    }
                    let (sender, new_receiver) = mpsc::channel();
                    progress = vec![0; config.worlds];
                    results.clear();
                    active_campaign = None;
                    active_campaign_is_saved = false;
                    launch_campaign(config.clone(), sender);
                    receiver = Some(new_receiver);
                    screen = Screen::Running;
                }
                if is_key_pressed(KeyCode::Escape) {
                    screen = Screen::Home;
                }
            }
            Screen::Running => {
                if let Some(messages) = &receiver {
                    while let Ok(message) = messages.try_recv() {
                        match message {
                            WorkerMessage::Progress { world_index, tick } => {
                                progress[world_index] = tick;
                            }
                            WorkerMessage::Complete(result) => {
                                progress[result.world_index] = result.ended_at_tick;
                                results.push(result);
                            }
                        }
                    }
                }
                draw_running(&config, &progress, results.len());
                if results.len() == config.worlds {
                    results.sort_by_key(|result| result.world_index);
                    receiver = None;
                    playback = Playback::default();
                    let saved_at = timestamp();
                    active_campaign = Some(StoredCampaign {
                        name: format!("Campanha {saved_at}"),
                        saved_at,
                        config: config.clone(),
                        results: std::mem::take(&mut results),
                    });
                    screen = Screen::Results;
                }
            }
            Screen::Results => {
                let Some(campaign) = active_campaign.as_mut() else {
                    screen = Screen::Home;
                    continue;
                };
                update_playback(&mut playback, &campaign.results);
                let action = draw_results(
                    &campaign.config,
                    &campaign.results,
                    &mut playback,
                    active_campaign_is_saved,
                );
                match action {
                    ResultsAction::Save if !active_campaign_is_saved => {
                        active_campaign_is_saved = save_campaign(campaign).is_ok();
                        saved_paths = saved_campaign_paths();
                    }
                    ResultsAction::UseSeed(seed) => {
                        config = campaign.config.clone();
                        config.mode = CampaignMode::EnvironmentStress;
                        config.source_seed = Some(seed);
                        screen = Screen::Setup;
                    }
                    ResultsAction::None | ResultsAction::Save => {}
                }
                if is_key_pressed(KeyCode::Escape) {
                    screen = Screen::Home;
                }
            }
            Screen::Library => match draw_library(&saved_paths) {
                LibraryAction::Back => screen = Screen::Home,
                LibraryAction::Open(path) => {
                    if let Ok(campaign) = load_campaign(&path) {
                        config = campaign.config.clone();
                        active_campaign = Some(campaign);
                        active_campaign_is_saved = true;
                        playback = Playback::default();
                        screen = Screen::Results;
                    }
                }
                LibraryAction::Duplicate(path) => {
                    if let Ok(campaign) = load_campaign(&path) {
                        config = campaign.config;
                        screen = Screen::Setup;
                    }
                }
            },
        }
        next_frame().await;
    }
}

enum HomeAction {
    None,
    SeedSweep,
    EnvironmentStress,
    Library,
}

enum LibraryAction {
    Back,
    Open(PathBuf),
    Duplicate(PathBuf),
}

enum ResultsAction {
    None,
    Save,
    UseSeed(u64),
}

fn launch_campaign(config: CampaignConfig, sender: Sender<WorkerMessage>) {
    for plan in build_run_plans(&config) {
        let world_config = config.clone();
        let worker_sender = sender.clone();
        thread::spawn(move || run_world(plan, world_config, worker_sender));
    }
}

fn build_run_plans(config: &CampaignConfig) -> Vec<RunPlan> {
    let mut random_state = fresh_seed();
    (0..config.worlds)
        .map(|world_index| RunPlan {
            world_index,
            seed: match config.mode {
                CampaignMode::SeedSweep => next_random_u64(&mut random_state),
                CampaignMode::EnvironmentStress => config.source_seed.unwrap_or_else(fresh_seed),
            },
            environment: effective_environment(config, &mut random_state),
        })
        .collect()
}

fn effective_environment(config: &CampaignConfig, random_state: &mut u64) -> EffectiveEnvironment {
    let choose_f32 = |vary: bool, fixed: f32, min: f32, max: f32, state: &mut u64| {
        if vary {
            min + (max - min) * next_random_unit(state)
        } else {
            fixed
        }
    };
    let choose_u32 = |vary: bool, fixed: u32, min: u32, max: u32, state: &mut u64| {
        if vary {
            (min as f32 + (max - min) as f32 * next_random_unit(state)).round() as u32
        } else {
            fixed
        }
    };
    let stress = config.mode == CampaignMode::EnvironmentStress;
    EffectiveEnvironment {
        emission: choose_f32(
            stress && config.vary_emission,
            config.emission,
            config.emission_min,
            config.emission_max,
            random_state,
        ),
        food_lifetime: choose_u32(
            stress && config.vary_food_lifetime,
            config.food_lifetime,
            config.food_lifetime_min,
            config.food_lifetime_max,
            random_state,
        ),
        food_energy: choose_f32(
            stress && config.vary_food_energy,
            config.food_energy,
            config.food_energy_min,
            config.food_energy_max,
            random_state,
        ),
        flow_intensity: choose_f32(
            stress && config.vary_flow_intensity,
            config.flow_intensity,
            config.flow_intensity_min,
            config.flow_intensity_max,
            random_state,
        ),
    }
}

fn fresh_seed() -> u64 {
    let counter = SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
    let clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    splitmix64(clock ^ counter.rotate_left(23))
}

fn next_random_u64(state: &mut u64) -> u64 {
    *state = splitmix64(*state);
    *state
}

fn next_random_unit(state: &mut u64) -> f32 {
    ((next_random_u64(state) >> 32) as u32) as f32 / u32::MAX as f32
}

fn splitmix64(value: u64) -> u64 {
    let mut value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn run_world(plan: RunPlan, config: CampaignConfig, sender: Sender<WorkerMessage>) {
    let flow = FlowField::new(plan.seed)
        .with_intensity(plan.environment.flow_intensity)
        .with_preset(config.flow_preset);
    let mut world = if config.amadeus_mode {
        World::new_amadeus_with_initial_population(
            plan.seed,
            config.map.clone(),
            config.initial_neural_configuration,
            &config.validated_genomes,
        )
    } else {
        World::new_with_initial_population(
            plan.seed,
            config.initial_cells,
            config.map.clone(),
            config.initial_neural_configuration,
            &config.validated_genomes,
        )
    };
    world.set_anti_cramming(config.anti_cramming);
    let snapshot_interval = (config.max_ticks / MAX_SNAPSHOTS).max(1);
    let progress_interval = (config.max_ticks / 100).max(1_000);
    let mut frames = vec![capture_frame(&world)];
    let mut ended_by = "Ticks máximos".to_owned();

    for tick in 1..=config.max_ticks {
        world.tick(
            &flow,
            plan.environment.emission,
            config.fertile_emission,
            plan.environment.food_lifetime,
            plan.environment.food_energy,
            config.mental_energy_cost,
            config.reproduction_energy,
            config.reproduction_limit,
        );
        if tick % snapshot_interval == 0 || tick == config.max_ticks {
            frames.push(capture_frame(&world));
        }
        if simulation_goal_reached(&world, &config) {
            ended_by = "Extinção".to_owned();
            if frames.last().is_none_or(|frame| frame.tick != world.tick) {
                frames.push(capture_frame(&world));
            }
            let _ = sender.send(WorkerMessage::Progress {
                world_index: plan.world_index,
                tick,
            });
            break;
        }
        if tick % progress_interval == 0 || tick == config.max_ticks {
            let _ = sender.send(WorkerMessage::Progress {
                world_index: plan.world_index,
                tick,
            });
        }
    }

    let result = WorldResult {
        world_index: plan.world_index,
        seed: plan.seed,
        frames,
        final_population: world.organisms.len(),
        births: world.births_total,
        average_energy: world.average_energy(),
        average_age: world.average_age_ticks(),
        average_generation: world.average_generation(),
        average_connections: world.average_connection_count(),
        ended_at_tick: world.tick,
        ended_by,
        environment: plan.environment,
    };
    let _ = sender.send(WorkerMessage::Complete(result));
}

fn simulation_goal_reached(world: &World, config: &CampaignConfig) -> bool {
    config.stop_on_extinction && world.organisms.is_empty()
}

fn capture_frame(world: &World) -> WorldFrame {
    let cell_stride = (world.organisms.len() / MAX_CAPTURED_CELLS).max(1);
    let food_stride = (world.food.len() / MAX_CAPTURED_FOOD).max(1);
    let cells = world
        .organisms
        .iter()
        .step_by(cell_stride)
        .take(MAX_CAPTURED_CELLS)
        .map(|cell| CellFrame {
            id: cell.id,
            position: cell.position,
            energy: cell.energy,
            age_ticks: cell.age_ticks,
            generation: cell.generation,
            children: cell.children_produced,
            connections: cell.brain.connection_count(),
            color: cell.genome.genetic_rgb(),
        })
        .collect();
    let food = world
        .food
        .iter()
        .step_by(food_stride)
        .take(MAX_CAPTURED_FOOD)
        .map(|particle| particle.position)
        .collect();
    WorldFrame {
        tick: world.tick,
        cells,
        food,
        population: world.organisms.len(),
        births: world.births_total,
        average_energy: world.average_energy(),
        average_generation: world.average_generation(),
    }
}

fn draw_home(saved_count: usize) -> HomeAction {
    draw_text("Laboratório de Experimentos", 42.0, 58.0, 32.0, TEXT);
    draw_text(
        "Campanhas offline, reprodução gravada e resultados persistentes.",
        42.0,
        84.0,
        16.0,
        MUTED,
    );
    let panel = Rect::new(42.0, 126.0, 640.0, 420.0);
    draw_rectangle(panel.x, panel.y, panel.w, panel.h, SURFACE);
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 1.0, BORDER);
    draw_text(
        "O que você quer fazer?",
        panel.x + 24.0,
        panel.y + 42.0,
        22.0,
        TEXT,
    );
    if button(
        Rect::new(panel.x + 24.0, panel.y + 76.0, panel.w - 48.0, 62.0),
        "Varredura de seeds aleatórias",
        true,
    ) {
        return HomeAction::SeedSweep;
    }
    draw_text(
        "Mesmo ambiente; múltiplos começos aleatórios.",
        panel.x + 32.0,
        panel.y + 162.0,
        14.0,
        MUTED,
    );
    if button(
        Rect::new(panel.x + 24.0, panel.y + 190.0, panel.w - 48.0, 62.0),
        "Estressar uma seed por ambiente",
        false,
    ) {
        return HomeAction::EnvironmentStress;
    }
    draw_text(
        "Escolha um mundo salvo; mantenha sua seed e varie somente o meio.",
        panel.x + 32.0,
        panel.y + 276.0,
        14.0,
        MUTED,
    );
    if button(
        Rect::new(panel.x + 24.0, panel.y + 304.0, panel.w - 48.0, 50.0),
        &format!("Gerenciar simulações salvas ({saved_count})"),
        false,
    ) {
        return HomeAction::Library;
    }
    draw_text(
        "Abra, reproduza ou duplique configurações já concluídas.",
        panel.x + 32.0,
        panel.y + 384.0,
        14.0,
        MUTED,
    );
    HomeAction::None
}

fn draw_library(paths: &[PathBuf]) -> LibraryAction {
    draw_text("Simulações salvas", 42.0, 58.0, 32.0, TEXT);
    draw_text(
        "Cada arquivo preserva configuração, resultados e reprodução visual.",
        42.0,
        84.0,
        16.0,
        MUTED,
    );
    if button(Rect::new(42.0, 106.0, 120.0, 34.0), "‹ Voltar", false) {
        return LibraryAction::Back;
    }
    if paths.is_empty() {
        draw_text("Ainda não há campanhas salvas.", 42.0, 190.0, 18.0, MUTED);
        return LibraryAction::Back;
    }
    for (index, path) in paths.iter().take(10).enumerate() {
        let y = 164.0 + index as f32 * 64.0;
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("campanha");
        draw_rectangle(42.0, y, 720.0, 50.0, SURFACE);
        draw_rectangle_lines(42.0, y, 720.0, 50.0, 1.0, BORDER);
        draw_text(name, 58.0, y + 30.0, 16.0, TEXT);
        if button(Rect::new(556.0, y + 8.0, 92.0, 34.0), "Abrir", true) {
            return LibraryAction::Open(path.clone());
        }
        if button(Rect::new(654.0, y + 8.0, 92.0, 34.0), "Duplicar", false) {
            return LibraryAction::Duplicate(path.clone());
        }
    }
    LibraryAction::Back
}

fn campaign_storage_dir() -> PathBuf {
    ecosystem_root().join("campanhas")
}

fn saved_campaign_paths() -> Vec<PathBuf> {
    let directory = campaign_storage_dir();
    let _ = fs::create_dir_all(&directory);
    let mut paths: Vec<_> = fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "ecos")
        })
        .collect();
    paths.sort();
    paths.reverse();
    paths
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn save_campaign(campaign: &StoredCampaign) -> io::Result<()> {
    let directory = campaign_storage_dir();
    fs::create_dir_all(&directory)?;
    let path = directory.join(format!("campanha_{}.ecos", campaign.saved_at));
    fs::write(path, encode_campaign(campaign))
}

fn load_campaign(path: &PathBuf) -> io::Result<StoredCampaign> {
    let data = fs::read(path)?;
    decode_campaign(&data)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "arquivo de campanha inválido"))
}

/// O arquivo de presets pertence ao aplicativo principal. O laboratório lê a
/// mesma estrutura, sem depender dos tipos privados da interface principal.
fn ecosystem_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn session_presets_path() -> PathBuf {
    ecosystem_root().join("presets_sessao.ecopreset")
}

fn import_session_preset(target_name: &str) -> io::Result<CampaignConfig> {
    let mut reader = io::BufReader::new(fs::File::open(session_presets_path())?);
    let mut magic = [0; PRESET_MAGIC.len()];
    reader.read_exact(&mut magic)?;
    let version = read_u32_preset(&mut reader)?;
    if magic != PRESET_MAGIC || !(1..=PRESET_FORMAT_VERSION).contains(&version) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "formato de preset incompatível",
        ));
    }
    let preset_count = read_u32_preset(&mut reader)? as usize;
    if preset_count > 80 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "quantidade de presets inválida",
        ));
    }

    for _ in 0..preset_count {
        let name = read_preset_string(&mut reader, 80)?;
        let settings = read_session_preset_settings(&mut reader, version)?;
        let map = MapDefinition::read_from(&mut reader)?;
        if name.eq_ignore_ascii_case(target_name) {
            return Ok(CampaignConfig {
                // Uma importação é intencionalmente um único mundo, com a
                // seed exata e sem sorteio de ambiente: é uma reprodução.
                mode: CampaignMode::EnvironmentStress,
                source_seed: Some(settings.seed),
                worlds: 1,
                initial_cells: settings.initial_cells,
                termination: TerminationCriterion::Ticks,
                termination_target: 100_000,
                max_ticks: 100_000,
                stop_on_extinction: false,
                emission: settings.desert_emission,
                food_lifetime: settings.lifetime,
                food_energy: settings.food_energy,
                reproduction_energy: settings.reproduction_energy,
                flow_intensity: settings.flow_intensity,
                vary_emission: false,
                emission_min: settings.desert_emission,
                emission_max: settings.desert_emission,
                vary_food_lifetime: false,
                food_lifetime_min: settings.lifetime,
                food_lifetime_max: settings.lifetime,
                vary_food_energy: false,
                food_energy_min: settings.food_energy,
                food_energy_max: settings.food_energy,
                vary_flow_intensity: false,
                flow_intensity_min: settings.flow_intensity,
                flow_intensity_max: settings.flow_intensity,
                fertile_emission: settings.fertile_emission,
                reproduction_limit: settings.reproduction_limit,
                mental_energy_cost: settings.mental_energy_cost,
                // A reprodução solicitada do Preset Mania usa a corrente
                // Clássica, independentemente do último modo visual salvo.
                flow_preset: FlowPreset::Classic,
                anti_cramming: settings.anti_cramming,
                map,
                initial_neural_configuration: settings.initial_neural_configuration,
                validated_genomes: settings.validated_genomes,
                amadeus_mode: settings.amadeus_mode,
                imported_preset_name: Some(name),
                preset_feedback: None,
            });
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("preset \"{target_name}\" não encontrado"),
    ))
}

struct SessionPresetSettings {
    seed: u64,
    amadeus_mode: bool,
    flow_intensity: f32,
    anti_cramming: AntiCrammingConfig,
    mental_energy_cost: f32,
    desert_emission: f32,
    fertile_emission: f32,
    lifetime: u32,
    food_energy: f32,
    reproduction_energy: f32,
    reproduction_limit: f32,
    initial_cells: usize,
    initial_neural_configuration: InitialNeuralConfiguration,
    validated_genomes: Vec<Genome>,
}

fn read_session_preset_settings(
    reader: &mut impl Read,
    preset_version: u32,
) -> io::Result<SessionPresetSettings> {
    let seed = read_u64_preset(reader)?;
    let amadeus_mode = read_bool_preset(reader)?;
    let initial_neural_configuration =
        initial_neural_configuration_from_wire(read_u32_preset(reader)?)?;
    let _saved_flow_preset = flow_preset_from_wire(read_u32_preset(reader)?)?;
    let flow_intensity = read_f32_preset(reader)?;
    let anti_cramming = AntiCrammingConfig {
        enabled: read_bool_preset(reader)?,
        force: read_f32_preset(reader)?,
        exclusion_distance: read_f32_preset(reader)?,
    };
    let mental_energy_cost = read_f32_preset(reader)?;
    let desert_emission = read_f32_preset(reader)?;
    let fertile_emission = read_f32_preset(reader)?;
    let lifetime = read_u32_preset(reader)?;
    let food_energy = read_f32_preset(reader)?;
    let reproduction_energy = read_f32_preset(reader)?;
    let reproduction_limit = read_f32_preset(reader)?;
    let initial_cells = read_u64_preset(reader)? as usize;
    let genome_count = read_u32_preset(reader)? as usize;
    if genome_count > MAX_PRESET_GENOMES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "pool validado grande demais",
        ));
    }
    let mut validated_genomes = Vec::with_capacity(genome_count);
    for _ in 0..genome_count {
        if preset_version >= 2 {
            let mut bits = [0; GENOME_BYTES];
            reader.read_exact(&mut bits)?;
            validated_genomes.push(Genome::from_bytes(bits));
        } else {
            let mut bits = [0; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES];
            reader.read_exact(&mut bits)?;
            validated_genomes.push(Genome::from_pre_food_sensor_range_bytes(bits));
        }
    }
    Ok(SessionPresetSettings {
        seed,
        amadeus_mode,
        flow_intensity,
        anti_cramming,
        mental_energy_cost,
        desert_emission,
        fertile_emission,
        lifetime,
        food_energy,
        reproduction_energy,
        reproduction_limit,
        initial_cells,
        initial_neural_configuration,
        validated_genomes,
    })
}

fn read_u32_preset(reader: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64_preset(reader: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_f32_preset(reader: &mut impl Read) -> io::Result<f32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(f32::from_le_bytes(bytes))
}

fn read_bool_preset(reader: &mut impl Read) -> io::Result<bool> {
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

fn read_preset_string(reader: &mut impl Read, maximum: usize) -> io::Result<String> {
    let length = read_u32_preset(reader)? as usize;
    if length > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "texto longo demais",
        ));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "texto inválido"))
}

fn flow_preset_from_wire(value: u32) -> io::Result<FlowPreset> {
    match value {
        0 => Ok(FlowPreset::Oasis),
        1 => Ok(FlowPreset::Classic),
        2 => Ok(FlowPreset::Left),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "corrente inválida",
        )),
    }
}

fn initial_neural_configuration_from_wire(value: u32) -> io::Result<InitialNeuralConfiguration> {
    match value {
        0 => Ok(InitialNeuralConfiguration::Classic),
        1 => Ok(InitialNeuralConfiguration::MotorMinimum),
        2 => Ok(InitialNeuralConfiguration::ValidatedV1),
        3 => Ok(InitialNeuralConfiguration::ValidatedV2),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "configuração neural inválida",
        )),
    }
}

fn encode_campaign(campaign: &StoredCampaign) -> Vec<u8> {
    let mut bytes = b"ECOSLAB6".to_vec();
    push_string(&mut bytes, &campaign.name);
    push_u64(&mut bytes, campaign.saved_at);
    let config = &campaign.config;
    bytes.push(config.mode.to_byte());
    push_u64(&mut bytes, config.source_seed.unwrap_or(u64::MAX));
    push_u64(&mut bytes, config.worlds as u64);
    push_u64(&mut bytes, config.initial_cells as u64);
    bytes.push(config.termination.to_byte());
    push_u64(&mut bytes, config.termination_target);
    push_u64(&mut bytes, config.max_ticks);
    push_f32(&mut bytes, config.emission);
    push_u32(&mut bytes, config.food_lifetime);
    push_f32(&mut bytes, config.food_energy);
    push_f32(&mut bytes, config.reproduction_energy);
    push_f32(&mut bytes, config.flow_intensity);
    bytes.push(config.vary_emission as u8);
    push_f32(&mut bytes, config.emission_min);
    push_f32(&mut bytes, config.emission_max);
    bytes.push(config.vary_food_lifetime as u8);
    push_u32(&mut bytes, config.food_lifetime_min);
    push_u32(&mut bytes, config.food_lifetime_max);
    bytes.push(config.vary_food_energy as u8);
    push_f32(&mut bytes, config.food_energy_min);
    push_f32(&mut bytes, config.food_energy_max);
    bytes.push(config.vary_flow_intensity as u8);
    push_f32(&mut bytes, config.flow_intensity_min);
    push_f32(&mut bytes, config.flow_intensity_max);
    bytes.push(config.stop_on_extinction as u8);
    push_f32(&mut bytes, config.fertile_emission);
    push_f32(&mut bytes, config.reproduction_limit);
    push_f32(&mut bytes, config.mental_energy_cost);
    push_u32(&mut bytes, flow_preset_to_wire(config.flow_preset));
    bytes.push(config.anti_cramming.enabled as u8);
    push_f32(&mut bytes, config.anti_cramming.force);
    push_f32(&mut bytes, config.anti_cramming.exclusion_distance);
    push_u32(
        &mut bytes,
        initial_neural_configuration_to_wire(config.initial_neural_configuration),
    );
    bytes.push(config.amadeus_mode as u8);
    push_string(
        &mut bytes,
        config.imported_preset_name.as_deref().unwrap_or_default(),
    );
    push_u32(
        &mut bytes,
        config.validated_genomes.len().min(MAX_PRESET_GENOMES) as u32,
    );
    for genome in config.validated_genomes.iter().take(MAX_PRESET_GENOMES) {
        bytes.extend(genome.bytes());
    }
    let _ = config.map.write_to(&mut bytes);
    push_u32(&mut bytes, campaign.results.len() as u32);
    for result in &campaign.results {
        push_u64(&mut bytes, result.world_index as u64);
        push_u64(&mut bytes, result.seed);
        push_u64(&mut bytes, result.final_population as u64);
        push_u64(&mut bytes, result.births);
        push_f32(&mut bytes, result.average_energy);
        push_f32(&mut bytes, result.average_age);
        push_f32(&mut bytes, result.average_generation);
        push_f32(&mut bytes, result.average_connections);
        push_u64(&mut bytes, result.ended_at_tick);
        push_string(&mut bytes, &result.ended_by);
        push_f32(&mut bytes, result.environment.emission);
        push_u32(&mut bytes, result.environment.food_lifetime);
        push_f32(&mut bytes, result.environment.food_energy);
        push_f32(&mut bytes, result.environment.flow_intensity);
        push_u32(&mut bytes, result.frames.len() as u32);
        for frame in &result.frames {
            push_u64(&mut bytes, frame.tick);
            push_u64(&mut bytes, frame.population as u64);
            push_u64(&mut bytes, frame.births);
            push_f32(&mut bytes, frame.average_energy);
            push_f32(&mut bytes, frame.average_generation);
            push_u32(&mut bytes, frame.cells.len() as u32);
            for cell in &frame.cells {
                push_u64(&mut bytes, cell.id);
                push_f32(&mut bytes, cell.position.x);
                push_f32(&mut bytes, cell.position.y);
                push_f32(&mut bytes, cell.energy);
                push_u64(&mut bytes, cell.age_ticks);
                push_u32(&mut bytes, cell.generation);
                push_u32(&mut bytes, cell.children);
                push_u64(&mut bytes, cell.connections as u64);
                bytes.extend(cell.color);
            }
            push_u32(&mut bytes, frame.food.len() as u32);
            for food in &frame.food {
                push_f32(&mut bytes, food.x);
                push_f32(&mut bytes, food.y);
            }
        }
    }
    bytes
}

fn decode_campaign(bytes: &[u8]) -> Option<StoredCampaign> {
    let mut reader = ByteReader::new(bytes);
    let magic = reader.take_exact(8)?;
    let newest_format = magic == b"ECOSLAB6";
    let current_format = newest_format || magic == b"ECOSLAB5";
    let has_extended_settings = current_format || magic == b"ECOSLAB4";
    (has_extended_settings || magic == b"ECOSLAB3").then_some(())?;
    let name = reader.string()?;
    let saved_at = reader.u64()?;
    let mode = CampaignMode::from_byte(reader.u8()?)?;
    let source_seed = reader.u64()?;
    let mut config = CampaignConfig {
        mode,
        source_seed: (source_seed != u64::MAX).then_some(source_seed),
        worlds: reader.u64()? as usize,
        initial_cells: reader.u64()? as usize,
        termination: TerminationCriterion::from_byte(reader.u8()?)?,
        termination_target: reader.u64()?,
        max_ticks: reader.u64()?,
        stop_on_extinction: false,
        emission: reader.f32()?,
        food_lifetime: reader.u32()?,
        food_energy: reader.f32()?,
        reproduction_energy: reader.f32()?,
        flow_intensity: reader.f32()?,
        vary_emission: reader.u8()? != 0,
        emission_min: reader.f32()?,
        emission_max: reader.f32()?,
        vary_food_lifetime: reader.u8()? != 0,
        food_lifetime_min: reader.u32()?,
        food_lifetime_max: reader.u32()?,
        vary_food_energy: reader.u8()? != 0,
        food_energy_min: reader.f32()?,
        food_energy_max: reader.f32()?,
        vary_flow_intensity: reader.u8()? != 0,
        flow_intensity_min: reader.f32()?,
        flow_intensity_max: reader.f32()?,
        ..CampaignConfig::default()
    };
    if current_format {
        config.stop_on_extinction = reader.u8()? != 0;
    }
    if has_extended_settings {
        config.fertile_emission = reader.f32()?;
        config.reproduction_limit = reader.f32()?;
        config.mental_energy_cost = reader.f32()?;
        config.flow_preset = flow_preset_from_wire(reader.u32()?).ok()?;
        config.anti_cramming = AntiCrammingConfig {
            enabled: reader.u8()? != 0,
            force: reader.f32()?,
            exclusion_distance: reader.f32()?,
        };
        config.initial_neural_configuration =
            initial_neural_configuration_from_wire(reader.u32()?).ok()?;
        config.amadeus_mode = reader.u8()? != 0;
        let imported_name = reader.string()?;
        config.imported_preset_name = (!imported_name.is_empty()).then_some(imported_name);
        let genome_count = reader.u32()? as usize;
        if genome_count > MAX_PRESET_GENOMES {
            return None;
        }
        config.validated_genomes = Vec::with_capacity(genome_count);
        for _ in 0..genome_count {
            if newest_format {
                let bits: [u8; GENOME_BYTES] = reader.take_exact(GENOME_BYTES)?.try_into().ok()?;
                config.validated_genomes.push(Genome::from_bytes(bits));
            } else {
                let bits: [u8; PRE_FOOD_SENSOR_RANGE_GENOME_BYTES] = reader
                    .take_exact(PRE_FOOD_SENSOR_RANGE_GENOME_BYTES)?
                    .try_into()
                    .ok()?;
                config
                    .validated_genomes
                    .push(Genome::from_pre_food_sensor_range_bytes(bits));
            }
        }
        let map_bytes = reader.take_exact(MAP_GRID_WIDTH * MAP_GRID_HEIGHT * 2)?;
        let mut map_reader = Cursor::new(map_bytes);
        config.map = MapDefinition::read_from(&mut map_reader).ok()?;
    }
    let result_count = reader.u32()? as usize;
    let mut results = Vec::with_capacity(result_count);
    for _ in 0..result_count {
        results.push(decode_result(&mut reader)?);
    }
    Some(StoredCampaign {
        name,
        saved_at,
        config,
        results,
    })
}

fn decode_result(reader: &mut ByteReader<'_>) -> Option<WorldResult> {
    let world_index = reader.u64()? as usize;
    let seed = reader.u64()?;
    let final_population = reader.u64()? as usize;
    let births = reader.u64()?;
    let average_energy = reader.f32()?;
    let average_age = reader.f32()?;
    let average_generation = reader.f32()?;
    let average_connections = reader.f32()?;
    let ended_at_tick = reader.u64()?;
    let ended_by = reader.string()?;
    let environment = EffectiveEnvironment {
        emission: reader.f32()?,
        food_lifetime: reader.u32()?,
        food_energy: reader.f32()?,
        flow_intensity: reader.f32()?,
    };
    let frame_count = reader.u32()? as usize;
    let mut frames = Vec::with_capacity(frame_count);
    for _ in 0..frame_count {
        let tick = reader.u64()?;
        let population = reader.u64()? as usize;
        let births = reader.u64()?;
        let average_energy = reader.f32()?;
        let average_generation = reader.f32()?;
        let cell_count = reader.u32()? as usize;
        let mut cells = Vec::with_capacity(cell_count);
        for _ in 0..cell_count {
            cells.push(CellFrame {
                id: reader.u64()?,
                position: vec2(reader.f32()?, reader.f32()?),
                energy: reader.f32()?,
                age_ticks: reader.u64()?,
                generation: reader.u32()?,
                children: reader.u32()?,
                connections: reader.u64()? as usize,
                color: [reader.u8()?, reader.u8()?, reader.u8()?],
            });
        }
        let food_count = reader.u32()? as usize;
        let mut food = Vec::with_capacity(food_count);
        for _ in 0..food_count {
            food.push(vec2(reader.f32()?, reader.f32()?));
        }
        frames.push(WorldFrame {
            tick,
            cells,
            food,
            population,
            births,
            average_energy,
            average_generation,
        });
    }
    Some(WorldResult {
        world_index,
        seed,
        frames,
        final_population,
        births,
        average_energy,
        average_age,
        average_generation,
        average_connections,
        ended_at_tick,
        ended_by,
        environment,
    })
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend(value.to_le_bytes());
}
fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend(value.to_le_bytes());
}
fn push_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend(value.to_le_bytes());
}
fn push_string(bytes: &mut Vec<u8>, value: &str) {
    push_u32(bytes, value.len() as u32);
    bytes.extend(value.as_bytes());
}

fn flow_preset_to_wire(preset: FlowPreset) -> u32 {
    match preset {
        FlowPreset::Oasis => 0,
        FlowPreset::Classic => 1,
        FlowPreset::Left => 2,
    }
}

fn initial_neural_configuration_to_wire(configuration: InitialNeuralConfiguration) -> u32 {
    match configuration {
        InitialNeuralConfiguration::Classic => 0,
        InitialNeuralConfiguration::MotorMinimum => 1,
        InitialNeuralConfiguration::ValidatedV1 => 2,
        InitialNeuralConfiguration::ValidatedV2 => 3,
    }
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }
    fn take_exact(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.index.checked_add(count)?;
        let slice = self.bytes.get(self.index..end)?;
        self.index = end;
        Some(slice)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take_exact(1)?.first().copied()
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take_exact(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take_exact(8)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take_exact(4)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<String> {
        let length = self.u32()? as usize;
        String::from_utf8(self.take_exact(length)?.to_vec()).ok()
    }
}

fn draw_setup_header() {
    draw_text("Laboratório de Experimentos", 42.0, 52.0, 30.0, TEXT);
    draw_text(
        "Execute mundos em paralelo, sem renderizar durante o cálculo.",
        42.0,
        78.0,
        16.0,
        MUTED,
    );
}

fn draw_setup(config: &mut CampaignConfig) -> bool {
    let panel = Rect::new(42.0, 112.0, (screen_width() - 84.0).min(760.0), 742.0);
    draw_rectangle(panel.x, panel.y, panel.w, panel.h, SURFACE);
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 1.0, BORDER);
    draw_text(
        config.mode.label(),
        panel.x + 22.0,
        panel.y + 36.0,
        22.0,
        TEXT,
    );
    if button(
        Rect::new(panel.x + panel.w - 202.0, panel.y + 14.0, 180.0, 30.0),
        "Importar Preset Mania",
        false,
    ) {
        match import_session_preset("Preset Mania") {
            Ok(mut imported) => {
                imported.preset_feedback = Some(
                    "Preset Mania importado: mapa e população preservados; corrente Clássica selecionada."
                        .to_owned(),
                );
                *config = imported;
            }
            Err(error) => {
                config.preset_feedback = Some(format!("Não foi possível importar: {error}"));
            }
        }
    }
    draw_text(
        match config.mode {
            CampaignMode::SeedSweep => {
                "Mesmo ambiente; cada mundo receberá uma seed inédita automaticamente."
            }
            CampaignMode::EnvironmentStress => {
                "Mesma seed de origem; cada mundo recebe um ambiente sorteado."
            }
        },
        panel.x + 22.0,
        panel.y + 60.0,
        14.0,
        MUTED,
    );

    let x = panel.x + 22.0;
    let width = panel.w - 44.0;
    if let Some(value) = slider_row(
        x,
        panel.y + 82.0,
        width,
        "Ticks máximos",
        config.max_ticks as f32,
        1_000.0,
        10_000_000.0,
        &format_ticks(config.max_ticks),
    ) {
        config.max_ticks = (value / 1_000.0).round().max(1.0) as u64 * 1_000;
        // Mantido para conseguir abrir campanhas antigas; não é mais uma
        // segunda meta de encerramento.
        config.termination = TerminationCriterion::Ticks;
        config.termination_target = config.max_ticks;
    }
    checkbox_row(
        x,
        panel.y + 138.0,
        "Encerrar se todas as células morrerem",
        &mut config.stop_on_extinction,
    );
    if let Some(value) = slider_row(
        x,
        panel.y + 187.0,
        width,
        if config.mode == CampaignMode::SeedSweep {
            "Seeds aleatórias"
        } else {
            "Ambientes sorteados"
        },
        config.worlds as f32,
        1.0,
        16.0,
        &config.worlds.to_string(),
    ) {
        config.worlds = value.round() as usize;
    }
    match config.mode {
        CampaignMode::SeedSweep => {
            draw_fixed_environment_controls(x, panel.y + 242.0, width, config)
        }
        CampaignMode::EnvironmentStress => {
            draw_environment_stress_controls(x, panel.y + 242.0, width, config)
        }
    }
    let summary = config.preset_feedback.as_deref().unwrap_or_else(|| {
        if config.imported_preset_name.is_some() {
            "Preset importado; esta execução usa seus parâmetros sem sorteio."
        } else {
            ""
        }
    });
    if !summary.is_empty() {
        draw_text(summary, x, panel.y + 622.0, 13.0, ACCENT);
    }
    draw_text(
        &format!(
            "{} mundos independentes serão iniciados em paralelo.",
            config.worlds
        ),
        x,
        panel.y + 652.0,
        14.0,
        MUTED,
    );
    button(
        Rect::new(x, panel.y + 680.0, width, 38.0),
        "Executar campanha offline",
        true,
    )
}

fn draw_fixed_environment_controls(x: f32, y: f32, width: f32, config: &mut CampaignConfig) {
    if let Some(value) = slider_row(
        x,
        y,
        width,
        "Células iniciais",
        config.initial_cells as f32,
        10.0,
        1_000.0,
        &config.initial_cells.to_string(),
    ) {
        config.initial_cells = value.round() as usize;
    }
    if let Some(value) = slider_row(
        x,
        y + 55.0,
        width,
        "Emissão",
        config.emission,
        0.02,
        4.0,
        &format!("{:.2} / tick", config.emission),
    ) {
        config.emission = value;
    }
    if let Some(value) = slider_row(
        x,
        y + 110.0,
        width,
        "Vida fito",
        config.food_lifetime as f32,
        100.0,
        20_000.0,
        &format_ticks(config.food_lifetime as u64),
    ) {
        config.food_lifetime = value.round() as u32;
    }
    if let Some(value) = slider_row(
        x,
        y + 165.0,
        width,
        "Energia da comida",
        config.food_energy,
        5.0,
        200.0,
        &format!("{:.0}", config.food_energy),
    ) {
        config.food_energy = value;
    }
    if let Some(value) = slider_row(
        x,
        y + 220.0,
        width,
        "Meta de reprodução",
        config.reproduction_energy,
        80.0,
        240.0,
        &format!("{:.0}", config.reproduction_energy),
    ) {
        config.reproduction_energy = value;
    }
    if let Some(value) = slider_row(
        x,
        y + 275.0,
        width,
        "Intensidade da corrente",
        config.flow_intensity,
        0.0,
        1.0,
        &format!("{:.0}%", config.flow_intensity * 100.0),
    ) {
        config.flow_intensity = value;
    }
}

fn draw_environment_stress_controls(x: f32, y: f32, width: f32, config: &mut CampaignConfig) {
    match config.source_seed {
        Some(seed) => draw_text(
            &format!("Seed de origem: {seed}"),
            x,
            y + 15.0,
            15.0,
            ACCENT,
        ),
        None => draw_text(
            "Escolha um mundo salvo para definir a seed de origem.",
            x,
            y + 15.0,
            15.0,
            Color::new(0.96, 0.65, 0.25, 1.0),
        ),
    };
    draw_text(
        &format!(
            "Células iniciais fixas: {} · reprodução fixa: {:.0}",
            config.initial_cells, config.reproduction_energy
        ),
        x,
        y + 37.0,
        13.0,
        MUTED,
    );
    draw_f32_variation(
        x,
        y + 52.0,
        width,
        "Emissão",
        &mut config.vary_emission,
        &mut config.emission_min,
        &mut config.emission_max,
        config.emission,
        0.02,
        4.0,
        |value| format!("{value:.2}"),
    );
    draw_u32_variation(
        x,
        y + 120.0,
        width,
        "Vida fito",
        &mut config.vary_food_lifetime,
        &mut config.food_lifetime_min,
        &mut config.food_lifetime_max,
        config.food_lifetime,
        100,
        20_000,
    );
    draw_f32_variation(
        x,
        y + 188.0,
        width,
        "Energia alimento",
        &mut config.vary_food_energy,
        &mut config.food_energy_min,
        &mut config.food_energy_max,
        config.food_energy,
        5.0,
        200.0,
        |value| format!("{value:.0}"),
    );
    draw_f32_variation(
        x,
        y + 256.0,
        width,
        "Intensidade corrente",
        &mut config.vary_flow_intensity,
        &mut config.flow_intensity_min,
        &mut config.flow_intensity_max,
        config.flow_intensity,
        0.0,
        1.0,
        |value| format!("{:.0}%", value * 100.0),
    );
}

fn draw_f32_variation<F: Fn(f32) -> String>(
    x: f32,
    y: f32,
    width: f32,
    label: &str,
    enabled: &mut bool,
    min: &mut f32,
    max: &mut f32,
    fixed: f32,
    lower: f32,
    upper: f32,
    format: F,
) {
    if button(
        Rect::new(x, y, 112.0, 28.0),
        if *enabled { "[x] variar" } else { "[ ] variar" },
        *enabled,
    ) {
        *enabled = !*enabled;
    }
    draw_text(label, x + 124.0, y + 19.0, 15.0, TEXT);
    if *enabled {
        let half = (width - 8.0) / 2.0;
        if let Some(value) =
            slider_row(x, y + 31.0, half, "mín.", *min, lower, upper, &format(*min))
        {
            *min = value;
        }
        if let Some(value) = slider_row(
            x + half + 8.0,
            y + 31.0,
            half,
            "máx.",
            *max,
            lower,
            upper,
            &format(*max),
        ) {
            *max = value;
        }
        if *min > *max {
            std::mem::swap(min, max);
        }
    } else {
        draw_text(
            &format!("Fixo: {}", format(fixed)),
            x + 124.0,
            y + 45.0,
            14.0,
            MUTED,
        );
    }
}

fn draw_u32_variation(
    x: f32,
    y: f32,
    width: f32,
    label: &str,
    enabled: &mut bool,
    min: &mut u32,
    max: &mut u32,
    fixed: u32,
    lower: u32,
    upper: u32,
) {
    if button(
        Rect::new(x, y, 112.0, 28.0),
        if *enabled { "[x] variar" } else { "[ ] variar" },
        *enabled,
    ) {
        *enabled = !*enabled;
    }
    draw_text(label, x + 124.0, y + 19.0, 15.0, TEXT);
    if *enabled {
        let half = (width - 8.0) / 2.0;
        if let Some(value) = slider_row(
            x,
            y + 31.0,
            half,
            "mín.",
            *min as f32,
            lower as f32,
            upper as f32,
            &min.to_string(),
        ) {
            *min = value.round() as u32;
        }
        if let Some(value) = slider_row(
            x + half + 8.0,
            y + 31.0,
            half,
            "máx.",
            *max as f32,
            lower as f32,
            upper as f32,
            &max.to_string(),
        ) {
            *max = value.round() as u32;
        }
        if *min > *max {
            std::mem::swap(min, max);
        }
    } else {
        draw_text(
            &format!("Fixo: {fixed} ticks"),
            x + 124.0,
            y + 45.0,
            14.0,
            MUTED,
        );
    }
}

fn draw_running(config: &CampaignConfig, progress: &[u64], completed: usize) {
    draw_text("Campanha em execução", 42.0, 52.0, 30.0, TEXT);
    draw_text(
        "Os mundos estão calculando em paralelo. Nenhuma tela está limitando os ticks.",
        42.0,
        78.0,
        16.0,
        MUTED,
    );
    let panel = Rect::new(42.0, 112.0, (screen_width() - 84.0).min(820.0), 620.0);
    draw_rectangle(panel.x, panel.y, panel.w, panel.h, SURFACE);
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 1.0, BORDER);
    draw_text(
        &format!("{} de {} mundos concluídos", completed, config.worlds),
        panel.x + 22.0,
        panel.y + 38.0,
        21.0,
        TEXT,
    );
    for (index, tick) in progress.iter().enumerate() {
        let y = panel.y + 76.0 + index as f32 * 32.0;
        let fraction = *tick as f32 / config.max_ticks as f32;
        draw_text(
            &format!("Mundo {:02}", index + 1),
            panel.x + 22.0,
            y + 15.0,
            15.0,
            TEXT,
        );
        draw_rectangle(panel.x + 130.0, y, panel.w - 178.0, 16.0, BACKGROUND);
        draw_rectangle(
            panel.x + 130.0,
            y,
            (panel.w - 178.0) * fraction,
            16.0,
            ACCENT,
        );
        draw_text(
            &format!("{:>3.0}%", fraction * 100.0),
            panel.x + panel.w - 40.0,
            y + 14.0,
            14.0,
            MUTED,
        );
    }
}

fn update_playback(playback: &mut Playback, results: &[WorldResult]) {
    let Some(result) = results.get(playback.world_index) else {
        return;
    };
    if playback.playing && result.frames.len() > 1 {
        playback.frame_credit += get_frame_time() * 12.0;
        let advance = playback.frame_credit.floor() as usize;
        playback.frame_credit -= advance as f32;
        playback.frame_index = (playback.frame_index + advance) % result.frames.len();
    }
    playback.frame_index = playback
        .frame_index
        .min(result.frames.len().saturating_sub(1));
}

fn draw_results(
    config: &CampaignConfig,
    results: &[WorldResult],
    playback: &mut Playback,
    saved: bool,
) -> ResultsAction {
    let panel_x = (screen_width() - PANEL_WIDTH).max(860.0);
    let arena = arena_rect(panel_x);
    let result = &results[playback.world_index];
    let frame = &result.frames[playback.frame_index];
    ensure_selected_cell(playback, frame);

    draw_text("Laboratório de Experimentos", 32.0, 40.0, 28.0, TEXT);
    draw_text(
        "Reprodução de quadros gravados · cálculo já concluído",
        32.0,
        66.0,
        16.0,
        MUTED,
    );
    draw_line(
        panel_x - 18.0,
        20.0,
        panel_x - 18.0,
        screen_height() - 20.0,
        1.0,
        BORDER,
    );
    let selected_cell = if playback.tab == Tab::Cells {
        playback.selected_cell_id
    } else {
        None
    };
    draw_arena(arena, frame, selected_cell);
    draw_results_panel(panel_x, config, results, playback, saved)
}

fn draw_results_panel(
    panel_x: f32,
    config: &CampaignConfig,
    results: &[WorldResult],
    playback: &mut Playback,
    saved: bool,
) -> ResultsAction {
    let left = panel_x + 20.0;
    let width = screen_width() - left - 20.0;
    for (index, (label, tab)) in [
        ("Sessão", Tab::Session),
        ("Células", Tab::Cells),
        ("Demografia", Tab::Demographics),
    ]
    .iter()
    .enumerate()
    {
        if button(
            Rect::new(
                left + index as f32 * (width / 3.0),
                42.0,
                width / 3.0 - 5.0,
                36.0,
            ),
            label,
            playback.tab == *tab,
        ) {
            playback.tab = *tab;
        }
    }
    let result = &results[playback.world_index];
    let frame = &result.frames[playback.frame_index];
    if button(Rect::new(left, 96.0, 34.0, 32.0), "<", false) {
        playback.world_index = (playback.world_index + results.len() - 1) % results.len();
        playback.frame_index = 0;
        playback.selected_cell_id = None;
    }
    if button(Rect::new(left + width - 34.0, 96.0, 34.0, 32.0), ">", false) {
        playback.world_index = (playback.world_index + 1) % results.len();
        playback.frame_index = 0;
        playback.selected_cell_id = None;
    }
    let heading = format!("Mundo {} · seed {}", result.world_index + 1, result.seed);
    centered_text(&heading, left + 38.0, 118.0, width - 76.0, 16.0, TEXT);

    match playback.tab {
        Tab::Session => draw_read_only_session(left, width, config, result, frame, playback, saved),
        Tab::Cells => {
            draw_cells(left, width, frame, playback);
            ResultsAction::None
        }
        Tab::Demographics => {
            draw_demographics(left, width, result, frame);
            ResultsAction::None
        }
    }
}

fn draw_read_only_session(
    left: f32,
    width: f32,
    config: &CampaignConfig,
    result: &WorldResult,
    frame: &WorldFrame,
    playback: &mut Playback,
    saved: bool,
) -> ResultsAction {
    draw_text("Sessão gravada", left, 160.0, 22.0, TEXT);
    draw_text(
        "Valores fixados antes da execução",
        left,
        182.0,
        14.0,
        MUTED,
    );
    value_row(left, 210.0, width, "Modo", config.mode.label());
    value_row(
        left,
        242.0,
        width,
        "Seed deste mundo",
        &result.seed.to_string(),
    );
    value_row(
        left,
        274.0,
        width,
        "Ticks máximos",
        &format_ticks(config.max_ticks),
    );
    value_row(
        left,
        306.0,
        width,
        "Encerrar por extinção",
        if config.stop_on_extinction {
            "Sim"
        } else {
            "Não"
        },
    );
    value_row(
        left,
        338.0,
        width,
        "Emissão",
        &format!("{:.2} / tick", result.environment.emission),
    );
    value_row(
        left,
        370.0,
        width,
        "Vida fito",
        &format_ticks(result.environment.food_lifetime as u64),
    );
    value_row(
        left,
        402.0,
        width,
        "Corrente",
        &format!("{:.0}%", result.environment.flow_intensity * 100.0),
    );
    draw_text("Reprodução visual", left, 446.0, 18.0, TEXT);
    value_row(
        left,
        474.0,
        width,
        "Tick no quadro",
        &format_ticks(frame.tick),
    );
    value_row(
        left,
        506.0,
        width,
        "População",
        &frame.population.to_string(),
    );
    if button(
        Rect::new(left, 534.0, width, 34.0),
        if playback.playing {
            "Pausar reprodução"
        } else {
            "Reproduzir"
        },
        playback.playing,
    ) {
        playback.playing = !playback.playing;
    }
    draw_text(
        "Pressione Esc para voltar à configuração.",
        left,
        620.0,
        14.0,
        MUTED,
    );
    draw_text(
        &format!(
            "Resultado final: {} células · {} nascimentos",
            result.final_population, result.births
        ),
        left,
        646.0,
        14.0,
        MUTED,
    );
    draw_text(
        &format!(
            "Encerrado por: {} · tick {}",
            result.ended_by, result.ended_at_tick
        ),
        left,
        670.0,
        14.0,
        MUTED,
    );
    if saved {
        draw_text("Campanha salva no laboratório.", left, 712.0, 15.0, ACCENT);
    } else if button(
        Rect::new(left, 690.0, width, 34.0),
        "Salvar no laboratório",
        true,
    ) {
        return ResultsAction::Save;
    }
    if button(
        Rect::new(left, 748.0, width, 34.0),
        "Estressar a seed deste mundo",
        false,
    ) {
        return ResultsAction::UseSeed(result.seed);
    }
    ResultsAction::None
}

fn draw_cells(left: f32, width: f32, frame: &WorldFrame, playback: &mut Playback) {
    draw_text("Células no quadro", left, 160.0, 22.0, TEXT);
    draw_text(
        "Amostra visual gravada desta etapa da execução",
        left,
        182.0,
        14.0,
        MUTED,
    );
    if frame.cells.is_empty() {
        draw_text(
            "Nenhuma célula viva neste quadro.",
            left,
            220.0,
            16.0,
            MUTED,
        );
        return;
    }
    let selected_index = frame
        .cells
        .iter()
        .position(|cell| Some(cell.id) == playback.selected_cell_id)
        .unwrap_or(0);
    if button(Rect::new(left, 206.0, 34.0, 34.0), "<", false) {
        playback.selected_cell_id =
            Some(frame.cells[(selected_index + frame.cells.len() - 1) % frame.cells.len()].id);
    }
    if button(
        Rect::new(left + width - 34.0, 206.0, 34.0, 34.0),
        ">",
        false,
    ) {
        playback.selected_cell_id = Some(frame.cells[(selected_index + 1) % frame.cells.len()].id);
    }
    let cell = &frame.cells[frame
        .cells
        .iter()
        .position(|cell| Some(cell.id) == playback.selected_cell_id)
        .unwrap_or(selected_index)];
    centered_text(
        &format!("Célula #{}", cell.id),
        left + 38.0,
        229.0,
        width - 76.0,
        17.0,
        TEXT,
    );
    value_row(
        left,
        272.0,
        width,
        "Energia",
        &format!("{:.1}", cell.energy),
    );
    value_row(left, 304.0, width, "Geração", &cell.generation.to_string());
    value_row(left, 336.0, width, "Idade", &format_ticks(cell.age_ticks));
    value_row(left, 368.0, width, "Filhos", &cell.children.to_string());
    value_row(
        left,
        400.0,
        width,
        "Conexões",
        &cell.connections.to_string(),
    );
    let color = Color::from_rgba(cell.color[0], cell.color[1], cell.color[2], 255);
    draw_circle(left + 18.0, 446.0, 10.0, color);
    draw_text(
        &format!(
            "Cor genética #{:02X}{:02X}{:02X}",
            cell.color[0], cell.color[1], cell.color[2]
        ),
        left + 38.0,
        451.0,
        15.0,
        TEXT,
    );
}

fn draw_demographics(left: f32, width: f32, result: &WorldResult, frame: &WorldFrame) {
    draw_text("Relatório demográfico", left, 160.0, 22.0, TEXT);
    value_row(
        left,
        198.0,
        width,
        "População neste quadro",
        &frame.population.to_string(),
    );
    value_row(
        left,
        230.0,
        width,
        "Nascimentos até o quadro",
        &frame.births.to_string(),
    );
    value_row(
        left,
        262.0,
        width,
        "Energia média",
        &format!("{:.1}", frame.average_energy),
    );
    value_row(
        left,
        294.0,
        width,
        "Geração média",
        &format!("{:.2}", frame.average_generation),
    );
    draw_text("Resultado final", left, 346.0, 18.0, TEXT);
    value_row(
        left,
        374.0,
        width,
        "População",
        &result.final_population.to_string(),
    );
    value_row(
        left,
        406.0,
        width,
        "Nascimentos totais",
        &result.births.to_string(),
    );
    value_row(
        left,
        438.0,
        width,
        "Idade média",
        &format_ticks(result.average_age as u64),
    );
    value_row(
        left,
        470.0,
        width,
        "Geração média",
        &format!("{:.2}", result.average_generation),
    );
    value_row(
        left,
        502.0,
        width,
        "Energia média final",
        &format!("{:.1}", result.average_energy),
    );
    value_row(
        left,
        534.0,
        width,
        "Conexões médias",
        &format!("{:.1}", result.average_connections),
    );
    draw_text("População ao longo da simulação", left, 586.0, 16.0, TEXT);
    draw_population_chart(Rect::new(left, 602.0, width, 118.0), &result.frames);
}

fn draw_arena(arena: Rect, frame: &WorldFrame, selected: Option<u64>) {
    draw_rectangle(
        arena.x,
        arena.y,
        arena.w,
        arena.h,
        Color::new(0.98, 0.985, 0.99, 1.0),
    );
    draw_rectangle_lines(arena.x, arena.y, arena.w, arena.h, 2.0, BORDER);
    draw_text(
        &format!("Tick {} · {} células", frame.tick, frame.population),
        arena.x + 14.0,
        arena.y + 24.0,
        15.0,
        MUTED,
    );
    for food in &frame.food {
        let point = world_to_screen(arena, *food);
        draw_circle(point.x, point.y, 1.5, Color::new(0.1, 0.85, 0.67, 0.58));
    }
    for cell in &frame.cells {
        let point = world_to_screen(arena, cell.position);
        let color = Color::from_rgba(cell.color[0], cell.color[1], cell.color[2], 255);
        draw_circle(point.x, point.y, 3.8, color);
        draw_circle_lines(
            point.x,
            point.y,
            3.8,
            1.0,
            Color::new(0.06, 0.08, 0.11, 0.85),
        );
        if Some(cell.id) == selected {
            draw_triangle(
                vec2(point.x, point.y - 16.0),
                vec2(point.x - 6.0, point.y - 25.0),
                vec2(point.x + 6.0, point.y - 25.0),
                Color::new(0.91, 0.12, 0.16, 1.0),
            );
        }
    }
}

fn draw_population_chart(rect: Rect, frames: &[WorldFrame]) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, BACKGROUND);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    let max_population = frames
        .iter()
        .map(|frame| frame.population)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    for pair in frames.windows(2) {
        let first = &pair[0];
        let second = &pair[1];
        let x1 = rect.x + rect.w * first.tick as f32 / frames.last().unwrap().tick.max(1) as f32;
        let x2 = rect.x + rect.w * second.tick as f32 / frames.last().unwrap().tick.max(1) as f32;
        let y1 = rect.y + rect.h - rect.h * first.population as f32 / max_population;
        let y2 = rect.y + rect.h - rect.h * second.population as f32 / max_population;
        draw_line(x1, y1, x2, y2, 2.0, ACCENT);
    }
}

fn arena_rect(panel_x: f32) -> Rect {
    let available_width = panel_x - 64.0;
    let available_height = screen_height() - 132.0;
    let width = available_width.min(available_height * 16.0 / 9.0);
    Rect::new(
        32.0,
        92.0 + (available_height - width * 9.0 / 16.0).max(0.0) / 2.0,
        width,
        width * 9.0 / 16.0,
    )
}

fn world_to_screen(arena: Rect, position: Vec2) -> Vec2 {
    vec2(
        arena.x + position.x / WORLD_WIDTH * arena.w,
        arena.y + position.y / WORLD_HEIGHT * arena.h,
    )
}

fn ensure_selected_cell(playback: &mut Playback, frame: &WorldFrame) {
    if !frame
        .cells
        .iter()
        .any(|cell| Some(cell.id) == playback.selected_cell_id)
    {
        playback.selected_cell_id = frame.cells.first().map(|cell| cell.id);
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
    displayed: &str,
) -> Option<f32> {
    draw_text(label, x, y + 15.0, 15.0, MUTED);
    let value_width = measure_text(displayed, None, 15, 1.0).width;
    draw_text(displayed, x + width - value_width, y + 15.0, 15.0, TEXT);
    let track = Rect::new(x, y + 28.0, width, 6.0);
    draw_rectangle(track.x, track.y, track.w, track.h, BACKGROUND);
    let fraction = ((value - min) / (max - min)).clamp(0.0, 1.0);
    draw_rectangle(track.x, track.y, track.w * fraction, track.h, ACCENT);
    draw_circle(
        track.x + track.w * fraction,
        track.y + track.h / 2.0,
        7.0,
        ACCENT,
    );
    let mouse = vec2(mouse_position().0, mouse_position().1);
    let interactive_area = Rect::new(
        track.x - 10.0,
        track.y - 10.0,
        track.w + 20.0,
        track.h + 20.0,
    );
    (is_mouse_button_pressed(MouseButton::Left) && interactive_area.contains(mouse)).then(|| {
        let fraction = ((mouse.x - track.x) / track.w).clamp(0.0, 1.0);
        min + (max - min) * fraction
    })
}

fn checkbox_row(x: f32, y: f32, label: &str, value: &mut bool) {
    let box_rect = Rect::new(x, y, 20.0, 20.0);
    let mouse = vec2(mouse_position().0, mouse_position().1);
    let hit_area = Rect::new(x, y - 4.0, 360.0, 28.0);
    if is_mouse_button_pressed(MouseButton::Left) && hit_area.contains(mouse) {
        *value = !*value;
    }
    draw_rectangle(box_rect.x, box_rect.y, box_rect.w, box_rect.h, BACKGROUND);
    draw_rectangle_lines(box_rect.x, box_rect.y, box_rect.w, box_rect.h, 1.0, BORDER);
    if *value {
        draw_rectangle(
            box_rect.x + 4.0,
            box_rect.y + 4.0,
            box_rect.w - 8.0,
            box_rect.h - 8.0,
            ACCENT,
        );
    }
    draw_text(label, x + 30.0, y + 16.0, 15.0, TEXT);
}

fn button(rect: Rect, label: &str, active: bool) -> bool {
    let mouse = vec2(mouse_position().0, mouse_position().1);
    let hovered = rect.contains(mouse);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if active {
            ACCENT
        } else if hovered {
            Color::new(0.12, 0.145, 0.19, 1.0)
        } else {
            SURFACE
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if active { ACCENT } else { BORDER },
    );
    centered_text(
        label,
        rect.x,
        rect.y + rect.h / 2.0 + 5.0,
        rect.w,
        14.0,
        if active { BACKGROUND } else { TEXT },
    );
    hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn value_row(x: f32, y: f32, width: f32, label: &str, value: &str) {
    draw_text(label, x, y, 15.0, MUTED);
    let value_width = measure_text(value, None, 15, 1.0).width;
    draw_text(value, x + width - value_width, y, 15.0, TEXT);
    draw_line(x, y + 10.0, x + width, y + 10.0, 1.0, BORDER);
}

fn centered_text(text: &str, x: f32, baseline: f32, width: f32, size: f32, color: Color) {
    let text_width = measure_text(text, None, size as u16, 1.0).width;
    draw_text(text, x + (width - text_width) / 2.0, baseline, size, color);
}

fn format_ticks(ticks: u64) -> String {
    if ticks >= 1_000_000 {
        format!("{:.2} milhões de ticks", ticks as f32 / 1_000_000.0)
    } else if ticks >= 1_000 {
        format!("{:.1} mil ticks", ticks as f32 / 1_000.0)
    } else {
        format!("{} ticks", ticks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_campaign_preserves_the_visual_playback() {
        let mut config = CampaignConfig::default();
        config.fertile_emission = 0.125;
        config.reproduction_limit = 760.0;
        config.mental_energy_cost = 0.015;
        config.flow_preset = FlowPreset::Classic;
        config.anti_cramming = AntiCrammingConfig {
            enabled: true,
            force: 1.8,
            exclusion_distance: 48.0,
        };
        config.initial_neural_configuration = InitialNeuralConfiguration::MotorMinimum;
        config.amadeus_mode = true;
        config.imported_preset_name = Some("Preset Mania".to_owned());
        config.validated_genomes = vec![Genome::random(&mut 77)];
        let campaign = StoredCampaign {
            name: "Campanha de teste".to_owned(),
            saved_at: 42,
            config,
            results: vec![WorldResult {
                world_index: 0,
                seed: 7,
                frames: vec![WorldFrame {
                    tick: 100,
                    cells: vec![CellFrame {
                        id: 9,
                        position: vec2(12.0, 34.0),
                        energy: 56.0,
                        age_ticks: 100,
                        generation: 3,
                        children: 2,
                        connections: 24,
                        color: [12, 34, 56],
                    }],
                    food: vec![vec2(8.0, 13.0)],
                    population: 1,
                    births: 2,
                    average_energy: 56.0,
                    average_generation: 3.0,
                }],
                final_population: 1,
                births: 2,
                average_energy: 56.0,
                average_age: 100.0,
                average_generation: 3.0,
                average_connections: 24.0,
                ended_at_tick: 100,
                ended_by: "Geração máxima".to_owned(),
                environment: EffectiveEnvironment {
                    emission: 1.0,
                    food_lifetime: 600,
                    food_energy: 50.0,
                    flow_intensity: 1.0,
                },
            }],
        };

        let decoded = decode_campaign(&encode_campaign(&campaign)).expect("campanha válida");
        assert_eq!(decoded.name, campaign.name);
        assert_eq!(decoded.results[0].frames[0].cells[0].id, 9);
        assert_eq!(decoded.results[0].frames[0].food.len(), 1);
        assert_eq!(decoded.results[0].ended_by, "Geração máxima");
        assert_eq!(decoded.config.flow_preset, FlowPreset::Classic);
        assert!(decoded.config.anti_cramming.enabled);
        assert_eq!(
            decoded.config.imported_preset_name.as_deref(),
            Some("Preset Mania")
        );
        assert_eq!(decoded.config.validated_genomes.len(), 1);
        assert!(decoded.config.amadeus_mode);
    }
}
