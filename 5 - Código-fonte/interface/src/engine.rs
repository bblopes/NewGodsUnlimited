//! Ponte entre a interface e o motor de simulação.
//!
//! O `World` vive só na thread do worker. A interface nunca recebe o mundo
//! inteiro: o worker publica uma fotografia compacta (`Frame`) no ritmo de
//! desenho e guarda a telemetria a cada 250 ticks. As regras da simulação são
//! as mesmas do executável antigo; este arquivo só reproduz o laço de execução
//! que antes ficava em `app.rs`.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use macroquad::prelude::Vec2;
use serde::{Deserialize, Serialize};

use new_gods_unlimited::config::{WORLD_HEIGHT, WORLD_WIDTH};
use new_gods_unlimited::environment::flow::{FlowField, FlowPreset};
use new_gods_unlimited::environment::map::{MapDefinition, MAP_GRID_HEIGHT, MAP_GRID_WIDTH};
use new_gods_unlimited::simulation::brain::{INTERNAL_START, MOTOR_START, SLOT_COUNT};
use new_gods_unlimited::simulation::genome::{
    Genome, BITS_PER_RECORD, CONNECTION_RECORDS, GENOME_BIT_COUNT, GENOME_BYTES,
};
use new_gods_unlimited::simulation::pheromone::{PheromoneDisplay, FIELD_HEIGHT, FIELD_WIDTH};
use new_gods_unlimited::simulation::world::{
    AntiCrammingConfig, InitialNeuralConfiguration, World, CELL_RADIUS,
    DEFAULT_MENTAL_ENERGY_COST, DEFAULT_PHYTOPLANKTON_REPRODUCTION_PER_TICK, FOOD_ENERGY,
    MAX_ENERGY, MAX_EXCLUSION_DISTANCE, MAX_PHYTOPLANKTON_REPRODUCTION_PER_TICK,
    MIN_EXCLUSION_DISTANCE,
};

use crate::banco::{genoma_de, Banco};

pub const SPEED_MULTIPLIERS: [f32; 7] = [
    1.0, 10.0, 100.0, 1_000.0, 10_000.0, 100_000.0, 1_000_000.0,
];
/// Intervalo padrão de coleta da Demografia; a interface pode trocar (mínimo 250).
const DEFAULT_TELEMETRY_INTERVAL_TICKS: u64 = 500;

fn max_threads() -> usize {
    thread::available_parallelism().map_or(1, |count| count.get())
}

/// 0 = um thread por núcleo lógico.
fn build_pool(threads: usize) -> Result<rayon::ThreadPool, rayon::ThreadPoolBuildError> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .thread_name(|index| format!("motor-{index}"))
        .build()
}
const MIN_TELEMETRY_INTERVAL_TICKS: u64 = 250;
const MAX_TELEMETRY_INTERVAL_TICKS: u64 = 100_000;
const MAX_TELEMETRY_SAMPLES: usize = 12_000;
const DEFAULT_SEED: u64 = 847_291_530;
const DEFAULT_REPRODUCTION_LIMIT: f32 = 5_000.0;
const GENETIC_GROUP_THRESHOLD: f32 = 0.985;

// ---------------------------------------------------------------------------
// Configuração vinda da interface
// ---------------------------------------------------------------------------

/// Regras do mundo que valem ao clicar em Aplicar.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvSettings {
    pub min_repro: f32,
    pub limit: f32,
    /// Custo físico por conexão/tick (a interface mostra ×40.000).
    pub mental: f32,
    /// Nascimento por abiogênese, em partículas por tick.
    pub emission: f32,
    pub lifetime: u32,
    pub food_energy: f32,
    /// Probabilidade por tick de multiplicação sob luz (0 a 0,02).
    pub phyto_rate: f32,
    pub flow_on: bool,
    /// 0 a 1.
    pub flow_intensity: f32,
    pub flow_preset: String,
    pub ac_on: bool,
    pub ac_force: f32,
    /// Distância mínima entre núcleos, em unidades do mundo.
    pub ac_distance: f32,
}

impl Default for EnvSettings {
    fn default() -> Self {
        Self {
            min_repro: MAX_ENERGY,
            limit: DEFAULT_REPRODUCTION_LIMIT,
            mental: DEFAULT_MENTAL_ENERGY_COST,
            emission: 0.12,
            lifetime: 600,
            food_energy: FOOD_ENERGY,
            phyto_rate: DEFAULT_PHYTOPLANKTON_REPRODUCTION_PER_TICK,
            flow_on: true,
            flow_intensity: 1.0,
            flow_preset: "Clássico".to_owned(),
            ac_on: true,
            ac_force: 1.0,
            ac_distance: MIN_EXCLUSION_DISTANCE,
        }
    }
}

impl EnvSettings {
    fn sanitized(mut self) -> Self {
        self.min_repro = self.min_repro.max(1.0);
        self.limit = self.limit.max(self.min_repro);
        self.mental = self.mental.max(0.0);
        self.emission = self.emission.max(0.0);
        self.lifetime = self.lifetime.max(1);
        self.food_energy = self.food_energy.max(0.0);
        self.phyto_rate = self.phyto_rate.clamp(0.0, MAX_PHYTOPLANKTON_REPRODUCTION_PER_TICK);
        self.flow_intensity = self.flow_intensity.clamp(0.0, 1.0);
        self.ac_force = self.ac_force.clamp(0.0, 1.0);
        self.ac_distance = self
            .ac_distance
            .clamp(MIN_EXCLUSION_DISTANCE, MAX_EXCLUSION_DISTANCE);
        self
    }

    fn flow_preset(&self) -> FlowPreset {
        flow_preset_from(&self.flow_preset)
    }

    fn effective_flow_intensity(&self) -> f32 {
        if self.flow_on {
            self.flow_intensity
        } else {
            0.0
        }
    }

    fn anti_cramming(&self) -> AntiCrammingConfig {
        AntiCrammingConfig {
            enabled: self.ac_on,
            force: self.ac_force,
            exclusion_distance: self.ac_distance,
        }
    }

    fn flow_field(&self, seed: u64) -> FlowField {
        FlowField::new(seed)
            .with_preset(self.flow_preset())
            .with_intensity(self.effective_flow_intensity())
    }
}

/// Condições que só valem ao reiniciar.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InitSettings {
    /// Texto, porque seeds de 64 bits não cabem num número do JavaScript.
    pub seed: String,
    pub cells: usize,
    pub neural: String,
    pub amadeus: bool,
}

impl Default for InitSettings {
    fn default() -> Self {
        Self {
            seed: DEFAULT_SEED.to_string(),
            cells: 20,
            neural: "classica".to_owned(),
            amadeus: false,
        }
    }
}

impl InitSettings {
    fn seed(&self) -> u64 {
        self.seed.trim().parse().unwrap_or(DEFAULT_SEED)
    }
}

pub fn flow_preset_from(name: &str) -> FlowPreset {
    match name {
        "Oásis" | "Oasis" | "oasis" => FlowPreset::Oasis,
        "Esquerda" | "esquerda" | "left" => FlowPreset::Left,
        _ => FlowPreset::Classic,
    }
}

fn neural_from(name: &str) -> InitialNeuralConfiguration {
    match name {
        "motor" => InitialNeuralConfiguration::MotorMinimum,
        "val1" => InitialNeuralConfiguration::ValidatedV1,
        "val2" => InitialNeuralConfiguration::ValidatedV2,
        "pfguia" => InitialNeuralConfiguration::FoodGradient,
        _ => InitialNeuralConfiguration::Classic,
    }
}

fn chem_display_from(name: &str) -> Option<PheromoneDisplay> {
    match name {
        "PG" => Some(PheromoneDisplay::Pg),
        "PF" => Some(PheromoneDisplay::Food),
        "Todos" => Some(PheromoneDisplay::All),
        other => other
            .strip_prefix('C')
            .and_then(|channel| channel.parse::<u8>().ok())
            .map(PheromoneDisplay::Generic),
    }
}

/// Pedidos da interface. Chegam do JavaScript como `{ "type": "...", ... }`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiCommand {
    Pause { value: bool },
    Speed { value: usize },
    Headless { value: bool },
    KeepAwake { value: bool },
    Apply { env: EnvSettings },
    Restart { init: InitSettings, env: EnvSettings },
    Insert { x: f32, y: f32, neural: String },
    Select { id: Option<u64> },
    Chem { display: Option<String> },
    MapStroke { erase: bool, x: f32, y: f32, radius: u8 },
    MapSun { points: Vec<[f32; 2]> },
    MapClearSun,
    MapApply,
    MapRevert,
    MapUndo,
    WantGenetics,
    /// Salva a célula viva `id` no banco de indivíduos.
    SaveIndividual { id: u64, name: String },
    /// Intervalo de coleta da Demografia, em ticks.
    TelemetryInterval { ticks: u64 },
    /// Threads de cálculo do motor; 0 = todos os núcleos.
    Threads { count: u64 },
    DeleteIndividual { key: String },
}

enum Command {
    Ui(UiCommand),
}

// ---------------------------------------------------------------------------
// O que a interface lê
// ---------------------------------------------------------------------------

/// Um trecho do genoma com o significado dos seus bits.
#[derive(Clone, Default, Serialize)]
pub struct GenomeSection {
    pub key: String,
    pub name: String,
    /// Primeiro bit (0 = primeiro bit do genoma) e quantidade de bits.
    pub start: usize,
    pub length: usize,
    pub bits: String,
    pub summary: String,
    pub note: String,
}

/// Um dos 64 registros de fiação: [origem:6][destino:6][peso:6][ativo:1].
#[derive(Clone, Default, Serialize)]
pub struct GenomeRecord {
    pub index: usize,
    pub start: usize,
    pub bits: String,
    pub source: usize,
    pub destination: usize,
    pub weight: f32,
    pub active: bool,
    pub legal: bool,
    /// Ativo, legal e com os dois nós ligados: entra de fato no cérebro.
    pub effective: bool,
}

#[derive(Clone, Default, Serialize)]
pub struct GenomeMap {
    pub total_bits: usize,
    /// Substância de P1, P2 e P3 (0 a 7 → C0 a C7).
    pub channels: [u8; 3],
    pub sections: Vec<GenomeSection>,
    pub records: Vec<GenomeRecord>,
}

#[derive(Clone, Default, Serialize)]
pub struct CellDetail {
    pub id: u64,
    pub energy: f32,
    pub capacity: f32,
    pub age: u64,
    pub max_age: u64,
    pub generation: u32,
    pub children: u32,
    pub radius: f32,
    pub sensor_range: f32,
    pub sensor_cost: f32,
    pub connections: usize,
    pub active_nodes: usize,
    pub color: String,
    pub similar_id: Option<u64>,
    pub similar: Option<f32>,
    pub slots: Vec<f32>,
    pub active: Vec<bool>,
    /// [origem, destino, peso, sinal]
    pub wires: Vec<[f32; 4]>,
    pub genome: String,
    pub genome_map: GenomeMap,
}

#[derive(Clone, Default, Serialize)]
pub struct ChemLayer {
    pub width: usize,
    pub height: usize,
    pub max: f32,
    /// Valores normalizados de 0 a 255, linha a linha.
    pub values: Vec<u8>,
}

#[derive(Clone, Default, Serialize)]
pub struct Notice {
    pub seq: u64,
    pub text: String,
    pub ok: bool,
}

#[derive(Clone, Default, Serialize)]
pub struct Frame {
    pub tick: u64,
    pub population: usize,
    pub food_count: usize,
    /// Ticks por segundo realmente simulados (relógio de parede).
    pub tps: f32,
    /// Teto de cálculo: quantos ticks/s o motor faria sem o limitador.
    pub capacity_tps: f32,
    pub publish_hz: f32,
    pub paused: bool,
    pub speed: usize,
    pub headless: bool,
    pub run_id: u64,
    pub epoch: u64,
    pub births_total: u64,
    pub deaths_total: u64,
    pub energy_capacity: f32,
    pub reproduction_minimum: f32,
    pub crowding_pairs: usize,
    pub crowding_min: Option<f32>,
    pub ids: Vec<u64>,
    pub xs: Vec<f32>,
    pub ys: Vec<f32>,
    pub radii: Vec<f32>,
    pub colors: Vec<u32>,
    pub energies: Vec<f32>,
    pub ranges: Vec<f32>,
    pub crowded: Vec<u8>,
    /// x0, y0, x1, y1, ...
    pub food: Vec<f32>,
    pub chem: Option<ChemLayer>,
    pub detail: Option<CellDetail>,
    pub map_version: u64,
    pub map_dirty: bool,
    pub bank_version: u64,
    pub telemetry_interval: u64,
    /// Threads de cálculo em uso, o pedido (0 = automático) e o máximo.
    pub threads: usize,
    pub threads_requested: usize,
    pub max_threads: usize,
    pub notice: Notice,
}

#[derive(Clone, Copy, Default, Serialize)]
pub struct Sample {
    pub tick: u64,
    pub population: usize,
    pub births_total: u64,
    pub deaths_total: u64,
    pub births_neural: u64,
    pub births_limit: u64,
    pub deaths_starvation: u64,
    pub deaths_phagocytosis: u64,
    pub deaths_age: u64,
    pub average_energy: f32,
    pub average_age: f32,
    pub average_generation: f32,
    pub max_generation: u32,
    pub genetic_diversity: f32,
    pub reproduction_minimum: f32,
    /// Fitoplâncton vivo no momento da amostra.
    pub phytoplankton: usize,
    /// Velocidade da simulação (ticks/s estimados pelo worker).
    pub tps: f32,
}

#[derive(Default, Serialize)]
pub struct TelemetryBatch {
    pub epoch: u64,
    pub total: usize,
    pub start: usize,
    pub samples: Vec<Sample>,
}

#[derive(Clone, Default, Serialize)]
pub struct MapData {
    pub version: u64,
    pub width: usize,
    pub height: usize,
    pub world_width: f32,
    pub world_height: f32,
    pub walls: Vec<u8>,
    pub sun: Vec<u8>,
    pub dirty: bool,
}

#[derive(Clone, Default, Serialize)]
pub struct GroupInfo {
    pub color: String,
    pub count: usize,
    pub share: f32,
    pub cohesion: f32,
    pub average_generation: f32,
    pub representative: u64,
}

#[derive(Clone, Default, Serialize)]
pub struct ConnectionInfo {
    pub source: usize,
    pub destination: usize,
    pub frequency: f32,
    pub mean_weight: f32,
    pub groups: usize,
}

#[derive(Clone, Default, Serialize)]
pub struct SlotInfo {
    pub slot: usize,
    pub used: f32,
    pub as_source: f32,
    pub as_destination: f32,
}

#[derive(Clone, Default, Serialize)]
pub struct Genetics {
    pub tick: u64,
    pub population: usize,
    pub group_count: usize,
    pub dominant_share: f32,
    pub dominant_cohesion: f32,
    pub consensus: f32,
    pub groups: Vec<GroupInfo>,
    pub connections: Vec<ConnectionInfo>,
    pub slots: Vec<SlotInfo>,
}

/// Uma opção de configuração neural: geradores fixos ou indivíduos salvos.
#[derive(Clone, Serialize)]
pub struct NeuralOption {
    /// "classica", "motor", "pfguia" ou "ind:<id>".
    pub key: String,
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub saved: bool,
    pub compatible: bool,
}

#[derive(Clone, Serialize)]
pub struct NeuralCatalog {
    pub version: u64,
    pub bank_path: String,
    pub bank_error: Option<String>,
    pub options: Vec<NeuralOption>,
}

#[derive(Clone, Serialize)]
pub struct IndividualView {
    pub key: String,
    pub name: String,
    pub description: String,
    pub origin: String,
    pub generation: Option<u32>,
    pub created_at: Option<u64>,
    pub color: String,
    pub connections: usize,
    pub active: Vec<bool>,
    /// [origem, destino, peso]
    pub wires: Vec<[f32; 3]>,
    pub genome: String,
    pub genome_map: GenomeMap,
    pub saved: bool,
}

#[derive(Clone, Serialize)]
pub struct EngineInfo {
    pub world_width: f32,
    pub world_height: f32,
    pub cell_radius: f32,
    pub min_exclusion: f32,
    pub max_exclusion: f32,
    pub max_phyto_rate: f32,
    pub speeds: Vec<f32>,
    pub env: EnvSettings,
    pub init: InitSettings,
    pub version: String,
}

// ---------------------------------------------------------------------------
// Estado compartilhado
// ---------------------------------------------------------------------------

struct Shared {
    frame: Mutex<Arc<Frame>>,
    telemetry: Mutex<(u64, Vec<Sample>)>,
    map: Mutex<(u64, Arc<MapDefinition>, bool)>,
    flow: Mutex<(u64, EnvSettings)>,
    genetics: Mutex<Option<Arc<Genetics>>>,
    genetics_wanted_at: AtomicU64,
    started: Instant,
    banco: Mutex<Banco>,
    bank_version: AtomicU64,
}

pub struct Engine {
    commands: Mutex<mpsc::Sender<Command>>,
    shared: Arc<Shared>,
}

impl Engine {
    pub fn start(caminho_do_banco: Option<PathBuf>) -> Self {
        let env = EnvSettings::default();
        let init = InitSettings::default();
        let map = MapDefinition::default();
        let shared = Arc::new(Shared {
            frame: Mutex::new(Arc::new(Frame::default())),
            telemetry: Mutex::new((0, Vec::new())),
            map: Mutex::new((1, Arc::new(map.clone()), false)),
            flow: Mutex::new((init.seed(), env.clone())),
            genetics: Mutex::new(None),
            genetics_wanted_at: AtomicU64::new(0),
            started: Instant::now(),
            banco: Mutex::new(Banco::abrir(caminho_do_banco)),
            bank_version: AtomicU64::new(1),
        });
        let (sender, receiver) = mpsc::channel();
        let worker_shared = Arc::clone(&shared);
        thread::Builder::new()
            .name("simulacao".to_owned())
            .spawn(move || worker(env, init, map, receiver, worker_shared))
            .expect("não foi possível iniciar a thread da simulação");
        Self {
            commands: Mutex::new(sender),
            shared,
        }
    }

    pub fn send(&self, command: UiCommand) {
        if let UiCommand::DeleteIndividual { key } = &command {
            self.delete_individual(key);
            return;
        }
        if let UiCommand::WantGenetics = command {
            let now = self.shared.started.elapsed().as_millis() as u64;
            self.shared.genetics_wanted_at.store(now.max(1), Ordering::Relaxed);
            return;
        }
        if let Ok(sender) = self.commands.lock() {
            let _ = sender.send(Command::Ui(command));
        }
    }

    pub fn frame(&self) -> Frame {
        self.shared
            .frame
            .lock()
            .map(|frame| (**frame).clone())
            .unwrap_or_default()
    }

    pub fn telemetry(&self, from: usize, epoch: u64) -> TelemetryBatch {
        let Ok(log) = self.shared.telemetry.lock() else {
            return TelemetryBatch::default();
        };
        let start = if epoch == log.0 { from.min(log.1.len()) } else { 0 };
        TelemetryBatch {
            epoch: log.0,
            total: log.1.len(),
            start,
            samples: log.1[start..].to_vec(),
        }
    }

    pub fn map_data(&self) -> MapData {
        let Ok(map) = self.shared.map.lock() else {
            return MapData::default();
        };
        let mut walls = vec![0u8; MAP_GRID_WIDTH * MAP_GRID_HEIGHT];
        let mut sun = vec![0u8; MAP_GRID_WIDTH * MAP_GRID_HEIGHT];
        for (x, y) in map.1.wall_cells() {
            walls[y * MAP_GRID_WIDTH + x] = 1;
        }
        for (x, y) in map.1.sunlight_cells() {
            sun[y * MAP_GRID_WIDTH + x] = 1;
        }
        MapData {
            version: map.0,
            width: MAP_GRID_WIDTH,
            height: MAP_GRID_HEIGHT,
            world_width: WORLD_WIDTH,
            world_height: WORLD_HEIGHT,
            walls,
            sun,
            dirty: map.2,
        }
    }

    /// Amostra a corrente numa grade `columns × rows`; devolve [x, y, fx, fy, ...].
    pub fn flow_vectors(&self, columns: usize, rows: usize) -> Vec<f32> {
        let (seed, env) = match self.shared.flow.lock() {
            Ok(flow) => (flow.0, flow.1.clone()),
            Err(_) => return Vec::new(),
        };
        let map = match self.shared.map.lock() {
            Ok(map) => Arc::clone(&map.1),
            Err(_) => return Vec::new(),
        };
        let field = env.flow_field(seed);
        let columns = columns.clamp(4, 120);
        let rows = rows.clamp(3, 80);
        let mut result = Vec::with_capacity(columns * rows * 4);
        for row in 0..rows {
            for column in 0..columns {
                let x = (column as f32 + 0.5) / columns as f32 * WORLD_WIDTH;
                let y = (row as f32 + 0.5) / rows as f32 * WORLD_HEIGHT;
                let position = Vec2::new(x, y);
                if map.is_wall(position) {
                    continue;
                }
                let flow = field.sample_with_map(position, Some(&map));
                result.extend_from_slice(&[x, y, flow.x, flow.y]);
            }
        }
        result
    }

    pub fn genetics(&self) -> Option<Genetics> {
        let now = self.shared.started.elapsed().as_millis() as u64;
        self.shared.genetics_wanted_at.store(now.max(1), Ordering::Relaxed);
        self.shared
            .genetics
            .lock()
            .ok()
            .and_then(|genetics| genetics.as_ref().map(|g| (**g).clone()))
    }

    fn delete_individual(&self, key: &str) {
        let id = key.strip_prefix("ind:").unwrap_or(key);
        if let Ok(mut banco) = self.shared.banco.lock() {
            if let Err(erro) = banco.remover(id) {
                banco.erro = Some(format!("não foi possível apagar: {erro}"));
            } else {
                banco.erro = None;
            }
        }
        self.shared.bank_version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn neural_catalog(&self) -> NeuralCatalog {
        let mut options = vec![
            NeuralOption {
                key: "classica".to_owned(),
                name: "Clássica".to_owned(),
                description: "Genomas iniciais totalmente aleatórios.".to_owned(),
                color: None,
                saved: false,
                compatible: true,
            },
            NeuralOption {
                key: "motor".to_owned(),
                name: "Motor".to_owned(),
                description: "4 caminhos diretos visão → motor; resto aleatório.".to_owned(),
                color: None,
                saved: false,
                compatible: true,
            },
            NeuralOption {
                key: "pfguia".to_owned(),
                name: "PF guia".to_owned(),
                description: "Fundador experimental: segue PFx ou PFy dominante, sem diagonal."
                    .to_owned(),
                color: None,
                saved: false,
                compatible: true,
            },
        ];
        let (bank_path, bank_error) = match self.shared.banco.lock() {
            Ok(banco) => {
                for individuo in banco.dados.individuos.iter().rev() {
                    let compatible = genoma_de(individuo).is_some();
                    options.push(NeuralOption {
                        key: format!("ind:{}", individuo.id),
                        name: individuo.nome.clone(),
                        description: if compatible {
                            format!("Indivíduo salvo · geração {} · {}", individuo.geracao, individuo.origem)
                        } else {
                            "Genoma salvo num formato antigo; precisa de migração.".to_owned()
                        },
                        color: Some(individuo.cor.clone()),
                        saved: true,
                        compatible,
                    });
                }
                (banco.caminho_texto(), banco.erro.clone())
            }
            Err(_) => (String::new(), Some("banco indisponível".to_owned())),
        };
        NeuralCatalog {
            version: self.shared.bank_version.load(Ordering::Relaxed),
            bank_path,
            bank_error,
            options,
        }
    }

    pub fn individual_view(&self, key: &str) -> Option<IndividualView> {
        let mut random_state = 0x5EED_1234_ABCD_0001u64;
        let (genome, name, description, origin, generation, created_at, saved) = match key {
            "classica" => (
                Genome::random(&mut random_state),
                "Clássica".to_owned(),
                "Exemplo de fundador: cada célula nasce com um genoma aleatório diferente.".to_owned(),
                "gerador fixo".to_owned(),
                None,
                None,
                false,
            ),
            "motor" => (
                Genome::random_with_motor_minimum(&mut random_state),
                "Motor".to_owned(),
                "Exemplo de fundador: 4 caminhos visão → motor garantidos, o resto aleatório.".to_owned(),
                "gerador fixo".to_owned(),
                None,
                None,
                false,
            ),
            "pfguia" => (
                Genome::food_gradient_ideal(),
                "PF guia".to_owned(),
                "Fundador fixo: todas as células começam com este mesmo genoma.".to_owned(),
                "gerador fixo".to_owned(),
                None,
                None,
                false,
            ),
            other => {
                let id = other.strip_prefix("ind:")?;
                let banco = self.shared.banco.lock().ok()?;
                let individuo = banco.buscar(id)?.clone();
                let genome = genoma_de(&individuo)?;
                (
                    genome,
                    individuo.nome.clone(),
                    "Indivíduo salvo: todas as células da run começam com este genoma.".to_owned(),
                    individuo.origem.clone(),
                    Some(individuo.geracao),
                    Some(individuo.criado_em),
                    true,
                )
            }
        };
        let connections = genome.decode_connections();
        let active = (0..SLOT_COUNT)
            .map(|slot| slot < INTERNAL_START || genome.neural_slot_is_active(slot))
            .collect();
        Some(IndividualView {
            key: key.to_owned(),
            name,
            description,
            origin,
            generation,
            created_at,
            color: genome.genetic_hex(),
            connections: connections.len(),
            active,
            wires: connections
                .iter()
                .map(|connection| {
                    [
                        connection.source as f32,
                        connection.destination as f32,
                        connection.weight,
                    ]
                })
                .collect(),
            genome: genome.binary_string(),
            genome_map: genome_map(&genome, None),
            saved,
        })
    }

    pub fn info() -> EngineInfo {
        EngineInfo {
            world_width: WORLD_WIDTH,
            world_height: WORLD_HEIGHT,
            cell_radius: CELL_RADIUS,
            min_exclusion: MIN_EXCLUSION_DISTANCE,
            max_exclusion: MAX_EXCLUSION_DISTANCE,
            max_phyto_rate: MAX_PHYTOPLANKTON_REPRODUCTION_PER_TICK,
            speeds: SPEED_MULTIPLIERS.to_vec(),
            env: EnvSettings::default(),
            init: InitSettings::default(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn set_windows_keep_awake(enabled: bool) {
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
    const ES_DISPLAY_REQUIRED: u32 = 0x0000_0002;
    const ES_CONTINUOUS: u32 = 0x8000_0000;

    extern "system" {
        fn SetThreadExecutionState(execution_state: u32) -> u32;
    }

    let execution_state = if enabled {
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
    } else {
        ES_CONTINUOUS
    };
    unsafe {
        let _ = SetThreadExecutionState(execution_state);
    }
}

#[cfg(not(target_os = "windows"))]
fn set_windows_keep_awake(_: bool) {}

fn saved_genome(key: &str, shared: &Shared) -> Option<Genome> {
    let id = key.strip_prefix("ind:")?;
    let banco = shared.banco.lock().ok()?;
    genoma_de(banco.buscar(id)?)
}

fn make_world(
    init: &InitSettings,
    env: &EnvSettings,
    map: &MapDefinition,
    founder: Option<&Genome>,
) -> World {
    let seed = init.seed();
    let configuration = neural_from(&init.neural);
    let mut world = if let Some(genome) = founder {
        let cells = if init.amadeus { 1 } else { init.cells };
        let mut world = World::new_with_founder_genomes(
            seed,
            cells,
            map.clone(),
            std::slice::from_ref(genome),
        );
        if init.amadeus {
            if let Some(organism) = world.organisms.first_mut() {
                organism.position = Vec2::new(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5);
            }
        }
        world
    } else if init.amadeus {
        World::new_amadeus_with_initial_population(seed, map.clone(), configuration, &[])
    } else {
        World::new_with_initial_population(seed, init.cells, map.clone(), configuration, &[])
    };
    world.set_phytoplankton_reproduction_rate(env.phyto_rate);
    world.set_anti_cramming(env.anti_cramming());
    world.reproduction_minimum = env.min_repro;
    world
}

fn target_publish_hz(speed: usize) -> f32 {
    let speed_multiplier = SPEED_MULTIPLIERS[speed];
    let speed_penalty = 1.0 + 0.35 * speed_multiplier.log10().max(0.0);
    (60.0 * 0.9 / speed_penalty).clamp(5.0, 60.0)
}

struct WorkerState {
    env: EnvSettings,
    init: InitSettings,
    world: World,
    flow: FlowField,
    applied_map: MapDefinition,
    edited_map: MapDefinition,
    map_history: Vec<MapDefinition>,
    map_version: u64,
    map_is_dirty: bool,
    paused: bool,
    keep_awake: bool,
    headless: bool,
    speed: usize,
    selected: Option<u64>,
    chem: Option<PheromoneDisplay>,
    run_id: u64,
    epoch: u64,
    notice: Notice,
    /// Teto de cálculo (só o tempo dos ticks, sem as pausas do limitador).
    /// Serve para dimensionar o lote; não é a velocidade da sessão.
    estimated_tps: f32,
    /// Velocidade real: ticks avançados ÷ tempo de relógio, em janelas de
    /// meio segundo. É o que a interface mostra como ticks/s.
    effective_tps: f32,
    rate_window_started: Instant,
    rate_window_ticks: u64,
    last_telemetry_tick: u64,
    telemetry_interval: u64,
    /// Pool próprio do motor. O resultado é o mesmo com qualquer número de
    /// threads; só muda a velocidade.
    pool: Arc<rayon::ThreadPool>,
    threads_requested: usize,
}

impl WorkerState {
    fn say(&mut self, text: impl Into<String>, ok: bool) {
        self.notice = Notice {
            seq: self.notice.seq + 1,
            text: text.into(),
            ok,
        };
    }

    fn publish_map(&mut self, shared: &Shared) {
        self.map_version += 1;
        let dirty = self.map_dirty();
        self.map_is_dirty = dirty;
        if let Ok(mut map) = shared.map.lock() {
            *map = (self.map_version, Arc::new(self.edited_map.clone()), dirty);
        }
    }

    fn map_dirty(&self) -> bool {
        self.edited_map.wall_cells().ne(self.applied_map.wall_cells())
            || self.edited_map.sunlight_cells().ne(self.applied_map.sunlight_cells())
    }

    fn publish_flow(&self, shared: &Shared) {
        if let Ok(mut flow) = shared.flow.lock() {
            *flow = (self.init.seed(), self.env.clone());
        }
    }

    fn reset_telemetry(&mut self, shared: &Shared) {
        self.epoch += 1;
        if let Ok(mut log) = shared.telemetry.lock() {
            log.0 = self.epoch;
            log.1.clear();
            log.1.push(capture_sample(&self.world));
        }
        self.last_telemetry_tick = self.world.tick;
    }

    fn handle(&mut self, command: UiCommand, shared: &Shared) {
        match command {
            UiCommand::Pause { value } => {
                self.paused = value;
                set_windows_keep_awake(self.keep_awake && !self.paused);
            }
            UiCommand::Speed { value } => self.speed = value.min(SPEED_MULTIPLIERS.len() - 1),
            UiCommand::Headless { value } => self.headless = value,
            UiCommand::KeepAwake { value } => {
                self.keep_awake = value;
                set_windows_keep_awake(self.keep_awake && !self.paused);
            }
            UiCommand::Apply { env } => {
                self.env = env.sanitized();
                self.flow = self.env.flow_field(self.init.seed());
                self.world.set_phytoplankton_reproduction_rate(self.env.phyto_rate);
                self.world.set_anti_cramming(self.env.anti_cramming());
                self.world.reproduction_minimum = self.env.min_repro;
                self.publish_flow(shared);
                let tick = self.world.tick;
                self.say(format!("Parâmetros aplicados no tick {tick}"), true);
            }
            UiCommand::Restart { init, env } => {
                self.init = init;
                self.env = env.sanitized();
                self.applied_map = self.edited_map.clone();
                let founder = if self.init.neural.starts_with("ind:") {
                    let genome = saved_genome(&self.init.neural, shared);
                    if genome.is_none() {
                        self.say("Indivíduo não encontrado no banco; usando a Clássica", false);
                        self.init.neural = "classica".to_owned();
                    }
                    genome
                } else {
                    None
                };
                self.world = make_world(&self.init, &self.env, &self.applied_map, founder.as_ref());
                self.flow = self.env.flow_field(self.init.seed());
                self.run_id = self.init.seed() ^ shared.started.elapsed().as_nanos() as u64;
                self.selected = None;
                self.publish_flow(shared);
                self.publish_map(shared);
                self.reset_telemetry(shared);
                if let Ok(mut genetics) = shared.genetics.lock() {
                    *genetics = None;
                }
                if founder.is_some() || !self.init.neural.starts_with("ind:") {
                    self.say("Nova run iniciada", true);
                }
            }
            UiCommand::Insert { x, y, neural } => {
                let configuration = neural_from(&neural);
                let saved = if neural.starts_with("ind:") {
                    saved_genome(&neural, shared)
                } else {
                    None
                };
                if neural.starts_with("ind:") && saved.is_none() {
                    self.say("Indivíduo não encontrado no banco", false);
                    return;
                }
                let inserted = match saved {
                    Some(genome) => self
                        .world
                        .insert_organism_with_genome_at(Vec2::new(x, y), genome),
                    None => self
                        .world
                        .insert_organism_at(Vec2::new(x, y), configuration, &[]),
                };
                if inserted {
                    self.selected = self.world.organisms.last().map(|organism| organism.id);
                    self.say("Célula inserida", true);
                } else {
                    self.say(
                        "Posição inválida: parede, borda ou outra célula no lugar",
                        false,
                    );
                }
            }
            UiCommand::Select { id } => self.selected = id,
            UiCommand::Chem { display } => {
                self.chem = display.as_deref().and_then(chem_display_from);
            }
            UiCommand::MapStroke {
                erase,
                x,
                y,
                radius,
            } => {
                self.edited_map
                    .paint_wall_brush(Vec2::new(x, y), !erase, radius);
                self.publish_map(shared);
            }
            UiCommand::MapSun { points } => {
                self.map_history.push(self.edited_map.clone());
                let polygon: Vec<Vec2> =
                    points.iter().map(|point| Vec2::new(point[0], point[1])).collect();
                self.edited_map.set_sunlight_polygon(&polygon);
                self.publish_map(shared);
            }
            UiCommand::MapClearSun => {
                self.map_history.push(self.edited_map.clone());
                self.edited_map.clear_sunlight();
                self.publish_map(shared);
            }
            UiCommand::MapApply => {
                self.applied_map = self.edited_map.clone();
                self.world.map = self.applied_map.clone();
                self.map_history.clear();
                self.publish_map(shared);
                self.say("Mapa aplicado ao mundo", true);
            }
            UiCommand::MapRevert => {
                self.edited_map = self.applied_map.clone();
                self.map_history.clear();
                self.publish_map(shared);
                self.say("Edição do mapa descartada", true);
            }
            UiCommand::MapUndo => {
                if let Some(previous) = self.map_history.pop() {
                    self.edited_map = previous;
                    self.publish_map(shared);
                }
            }
            UiCommand::SaveIndividual { id, name } => {
                let found = self
                    .world
                    .organisms
                    .iter()
                    .find(|organism| organism.id == id)
                    .map(|organism| (organism.generation, organism.genome.clone()));
                let Some((generation, genome)) = found else {
                    self.say("A célula não está mais viva; nada foi salvo", false);
                    return;
                };
                let name = if name.trim().is_empty() {
                    format!("Célula #{id}")
                } else {
                    name.trim().chars().take(60).collect()
                };
                let origin = format!(
                    "run #{} · tick {} · célula #{id}",
                    self.run_id % 10_000_000,
                    self.world.tick
                );
                let result = match shared.banco.lock() {
                    Ok(mut banco) => banco.adicionar(name.clone(), origin, generation, &genome),
                    Err(_) => Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "banco indisponível",
                    )),
                };
                match result {
                    Ok(_) => {
                        shared.bank_version.fetch_add(1, Ordering::Relaxed);
                        self.say(
                            format!("“{name}” salvo; já aparece em Sessão › Configuração neural"),
                            true,
                        );
                    }
                    Err(erro) => self.say(format!("Não foi possível salvar: {erro}"), false),
                }
            }
            UiCommand::TelemetryInterval { ticks } => {
                self.telemetry_interval =
                    ticks.clamp(MIN_TELEMETRY_INTERVAL_TICKS, MAX_TELEMETRY_INTERVAL_TICKS);
                let interval = self.telemetry_interval;
                self.say(format!("Demografia coletando a cada {interval} ticks"), true);
            }
            UiCommand::Threads { count } => {
                let requested = (count as usize).min(max_threads());
                match build_pool(requested) {
                    Ok(pool) => {
                        self.pool = Arc::new(pool);
                        self.threads_requested = requested;
                        let used = self.pool.current_num_threads();
                        self.say(
                            if requested == 0 {
                                format!("Cálculo em todos os núcleos ({used} threads)")
                            } else {
                                format!("Cálculo em {used} thread(s)")
                            },
                            true,
                        );
                    }
                    Err(error) => self.say(format!("Não foi possível trocar as threads: {error}"), false),
                }
            }
            UiCommand::DeleteIndividual { .. } | UiCommand::WantGenetics => {}
        }
    }

    fn begin_stroke(&mut self) {
        if self.map_history.len() > 40 {
            self.map_history.remove(0);
        }
        self.map_history.push(self.edited_map.clone());
    }

    fn build_frame(&self, publish_hz: f32, light: bool) -> Frame {
        let world = &self.world;
        let mut frame = Frame {
            tick: world.tick,
            population: world.organisms.len(),
            food_count: world.food.len(),
            tps: if self.paused { 0.0 } else { self.effective_tps },
            capacity_tps: self.estimated_tps,
            publish_hz,
            paused: self.paused,
            speed: self.speed,
            headless: self.headless,
            run_id: self.run_id,
            epoch: self.epoch,
            births_total: world.births_total,
            deaths_total: world.deaths_total,
            energy_capacity: world.energy_capacity,
            reproduction_minimum: world.reproduction_minimum,
            crowding_pairs: world.crowding.violating_pairs,
            crowding_min: world.crowding.minimum_violation_distance,
            map_version: self.map_version,
            map_dirty: self.map_is_dirty,
            telemetry_interval: self.telemetry_interval,
            threads: self.pool.current_num_threads(),
            threads_requested: self.threads_requested,
            max_threads: max_threads(),
            notice: self.notice.clone(),
            ..Frame::default()
        };
        if light {
            return frame;
        }
        let count = world.organisms.len();
        frame.ids.reserve(count);
        frame.xs.reserve(count);
        frame.ys.reserve(count);
        frame.radii.reserve(count);
        frame.colors.reserve(count);
        frame.energies.reserve(count);
        frame.ranges.reserve(count);
        frame.crowded.reserve(count);
        for organism in &world.organisms {
            let [r, g, b] = organism.genome.genetic_rgb();
            frame.ids.push(organism.id);
            frame.xs.push(organism.position.x);
            frame.ys.push(organism.position.y);
            frame.radii.push(organism.physical_radius());
            frame.colors.push(((r as u32) << 16) | ((g as u32) << 8) | b as u32);
            frame.energies.push(organism.energy);
            frame.ranges.push(organism.food_sensor_range());
            frame.crowded.push(organism.crowding_violation as u8);
        }
        frame.food.reserve(world.food.len() * 2);
        for particle in &world.food {
            frame.food.push(particle.position.x);
            frame.food.push(particle.position.y);
        }
        if let Some(display) = self.chem {
            let max = world.pheromones.display_max(display);
            let scale = if max > 0.0 { 255.0 / max } else { 0.0 };
            let mut values = Vec::with_capacity(FIELD_WIDTH * FIELD_HEIGHT);
            for y in 0..FIELD_HEIGHT {
                for x in 0..FIELD_WIDTH {
                    let value = world.pheromones.display_value(display, x, y) * scale;
                    values.push(value.clamp(0.0, 255.0) as u8);
                }
            }
            frame.chem = Some(ChemLayer {
                width: FIELD_WIDTH,
                height: FIELD_HEIGHT,
                max,
                values,
            });
        }
        if let Some(id) = self.selected {
            frame.detail = cell_detail(world, id);
        }
        frame
    }

}

fn worker(
    env: EnvSettings,
    init: InitSettings,
    map: MapDefinition,
    commands: mpsc::Receiver<Command>,
    shared: Arc<Shared>,
) {
    let world = make_world(&init, &env, &map, None);
    let flow = env.flow_field(init.seed());
    let mut state = WorkerState {
        run_id: init.seed(),
        env,
        init,
        world,
        flow,
        applied_map: map.clone(),
        edited_map: map,
        map_history: Vec::new(),
        map_version: 1,
        map_is_dirty: false,
        paused: false,
        keep_awake: true,
        headless: false,
        speed: 0,
        selected: None,
        chem: None,
        epoch: 0,
        notice: Notice::default(),
        estimated_tps: 1_000.0,
        effective_tps: 0.0,
        rate_window_started: Instant::now(),
        rate_window_ticks: 0,
        last_telemetry_tick: 0,
        telemetry_interval: DEFAULT_TELEMETRY_INTERVAL_TICKS,
        // Padrão: 1 thread. Na medição da V3.0.3 o tick (~0,5 ms) é curto
        // demais para compensar acordar várias threads a cada etapa; o
        // seletor da Sessão permite testar mais núcleos em cada máquina.
        pool: Arc::new(build_pool(1).expect("pool de threads do motor")),
        threads_requested: 1,
    };
    set_windows_keep_awake(true);
    state.reset_telemetry(&shared);
    state.publish_map(&shared);
    let mut last_publish = Instant::now() - Duration::from_secs(1);
    let mut last_genetics = Instant::now() - Duration::from_secs(10);
    let mut stroke_open = false;
    let mut last_stroke = Instant::now();

    loop {
        let mut force_publish = false;
        while let Ok(Command::Ui(command)) = commands.try_recv() {
            if let UiCommand::MapStroke { .. } = command {
                // Uma pincelada contínua vira um único passo de desfazer.
                if !stroke_open || last_stroke.elapsed() > Duration::from_millis(400) {
                    state.begin_stroke();
                    stroke_open = true;
                }
                last_stroke = Instant::now();
            } else {
                stroke_open = false;
            }
            state.handle(command, &shared);
            force_publish = true;
        }

        // Relatório genético: só é calculado enquanto a interface pede.
        let wanted = shared.genetics_wanted_at.load(Ordering::Relaxed);
        let now_ms = shared.started.elapsed().as_millis() as u64;
        if wanted > 0
            && now_ms.saturating_sub(wanted) < 3_000
            && last_genetics.elapsed() >= Duration::from_millis(1_500)
        {
            let report = genetics_report(&state.world);
            if let Ok(mut genetics) = shared.genetics.lock() {
                *genetics = Some(Arc::new(report));
            }
            last_genetics = Instant::now();
        }

        let publish_hz = target_publish_hz(state.speed);

        if state.paused {
            if force_publish || last_publish.elapsed() >= Duration::from_millis(100) {
                let mut frame = state.build_frame(publish_hz, state.headless);
                frame.bank_version = shared.bank_version.load(Ordering::Relaxed);
                if let Ok(mut target) = shared.frame.lock() {
                    *target = Arc::new(frame);
                }
                last_publish = Instant::now();
            }
            // Ao retomar, a medição recomeça do zero: a pausa não conta.
            state.effective_tps = 0.0;
            state.rate_window_started = Instant::now();
            state.rate_window_ticks = 0;
            thread::sleep(Duration::from_millis(4));
            continue;
        }

        let requested_tps = 10.0 * SPEED_MULTIPLIERS[state.speed];
        let batch = if state.headless || requested_tps > state.estimated_tps {
            (state.estimated_tps * 0.008).round().clamp(64.0, 8_192.0) as u32
        } else {
            (requested_tps / publish_hz).ceil().max(1.0) as u32
        };
        let started = Instant::now();
        // O lote inteiro roda dentro do pool do motor: as etapas paralelas do
        // tick usam essas threads sem trocar de thread a cada tick.
        let pool = Arc::clone(&state.pool);
        pool.install(|| for _ in 0..batch {
            state.world.tick(
                &state.flow,
                state.env.emission,
                state.env.emission,
                state.env.lifetime,
                state.env.food_energy,
                state.env.mental,
                state.env.min_repro,
                state.env.limit,
            );
            if state.world.tick.saturating_sub(state.last_telemetry_tick) >= state.telemetry_interval {
                if let Ok(mut log) = shared.telemetry.lock() {
                    let mut sample = capture_sample(&state.world);
                    sample.reproduction_minimum = state.env.min_repro;
                    sample.tps = state.effective_tps;
                    if log.1.len() >= MAX_TELEMETRY_SAMPLES {
                        // Mantém a série inteira com metade da resolução.
                        let kept: Vec<Sample> = log.1.iter().copied().step_by(2).collect();
                        log.1 = kept;
                    }
                    log.1.push(sample);
                }
                state.last_telemetry_tick = state.world.tick;
            }
        });
        let elapsed = started.elapsed().as_secs_f32().max(0.000_001);
        let observed_tps = batch as f32 / elapsed;
        state.estimated_tps = state.estimated_tps * 0.85 + observed_tps * 0.15;
        // A velocidade real inclui as pausas do limitador e a publicação.
        state.rate_window_ticks += batch as u64;
        let window = state.rate_window_started.elapsed().as_secs_f32();
        if window >= 0.5 {
            let measured = state.rate_window_ticks as f32 / window;
            state.effective_tps = if state.effective_tps > 0.0 {
                state.effective_tps * 0.5 + measured * 0.5
            } else {
                measured
            };
            state.rate_window_started = Instant::now();
            state.rate_window_ticks = 0;
        }

        let interval = if state.headless {
            0.25
        } else {
            1.0 / publish_hz
        };
        if force_publish || last_publish.elapsed().as_secs_f32() >= interval {
            let mut frame =
                state.build_frame(if state.headless { 0.0 } else { publish_hz }, state.headless);
            frame.bank_version = shared.bank_version.load(Ordering::Relaxed);
            if let Ok(mut target) = shared.frame.lock() {
                *target = Arc::new(frame);
            }
            last_publish = Instant::now();
        }

        if !state.headless && requested_tps < observed_tps {
            let desired_seconds = batch as f32 / requested_tps;
            let remaining = desired_seconds - elapsed;
            if remaining > 0.000_5 {
                thread::sleep(Duration::from_secs_f32(remaining.min(0.25)));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Leituras derivadas do mundo
// ---------------------------------------------------------------------------

fn cell_detail(world: &World, id: u64) -> Option<CellDetail> {
    let organism = world.organisms.iter().find(|organism| organism.id == id)?;
    let similar = world.most_similar_living_cell(id);
    let slots = (0..SLOT_COUNT)
        .map(|slot| organism.brain.slot_value(slot))
        .collect();
    let active = (0..SLOT_COUNT)
        .map(|slot| slot < INTERNAL_START || organism.genome.neural_slot_is_active(slot))
        .collect();
    let wires = organism
        .brain
        .visual_connections()
        .map(|(connection, signal)| {
            [
                connection.source as f32,
                connection.destination as f32,
                connection.weight,
                signal,
            ]
        })
        .collect();
    Some(CellDetail {
        id,
        energy: organism.energy,
        capacity: world.energy_capacity,
        age: organism.age_ticks,
        max_age: organism.maximum_age_ticks(),
        generation: organism.generation,
        children: organism.children_produced,
        radius: organism.physical_radius(),
        sensor_range: organism.food_sensor_range(),
        sensor_cost: organism.food_sensor_energy_cost(),
        connections: organism.brain.connection_count(),
        active_nodes: organism.brain.active_node_count(),
        color: organism.genome.genetic_hex(),
        similar_id: similar.map(|(other, _)| other),
        similar: similar.map(|(_, value)| value),
        slots,
        active,
        wires,
        genome: organism.genome.binary_string(),
        genome_map: genome_map(&organism.genome, Some(organism.maximum_age_ticks())),
    })
}

const SLOT_LABELS: [&str; SLOT_COUNT] = [
    "V<", "V>", "V^", "Vv", "Cx", "Cy", "En", "+1", "Ct", "PG", "PGx", "PGy", "P1", "P1x", "P1y",
    "P2", "P2x", "P2y", "P3", "P3x", "P3y", "Fx", "Fy", "PF", "PFx", "PFy", "I0", "I1", "I2", "I3",
    "I4", "I5", "I6", "I7", "I8", "I9", "I10", "I11", "I12", "I13", "I14", "I15", "T1", "T2", "T3",
    "←", "→", "↑", "↓", "Rep", "Fag", "EG", "E1", "E2", "E3", "Doar",
];

fn slot_label(slot: usize) -> String {
    SLOT_LABELS
        .get(slot)
        .map(|label| (*label).to_owned())
        .unwrap_or_else(|| format!("#{slot}"))
}

fn bits_value(bits: &str) -> u32 {
    bits.chars().fold(0, |value, bit| (value << 1) | (bit == '1') as u32)
}

/// Decodifica o genoma em trechos legíveis. As posições seguem `genome.rs`:
/// 64 registros de 19 bits (0–1215), canais químicos nos 9 bits finais desse
/// bloco (1207–1215, sobrepostos ao registro 63), alelo de alcance (1216–1220),
/// atividade dos 30 nós (1221–1250) e 5 bits de reserva no último byte.
fn genome_map(genome: &Genome, max_age: Option<u64>) -> GenomeMap {
    let bits = genome.binary_string();
    let slice = |start: usize, length: usize| -> String {
        bits.get(start..(start + length).min(bits.len()))
            .unwrap_or("")
            .to_owned()
    };
    let neural_bits = CONNECTION_RECORDS * BITS_PER_RECORD;
    let channel_start = neural_bits - 9;
    let allele_start = neural_bits;
    let activity_start = allele_start + 5;
    let activity_length = GENOME_BIT_COUNT - activity_start;

    let mut records = Vec::with_capacity(CONNECTION_RECORDS);
    for index in 0..CONNECTION_RECORDS {
        let start = index * BITS_PER_RECORD;
        let record_bits = slice(start, BITS_PER_RECORD);
        let source = bits_value(&record_bits[0..6]) as usize;
        let destination = bits_value(&record_bits[6..12]) as usize;
        let raw_weight = bits_value(&record_bits[12..18]) as i32;
        let weight_steps = if raw_weight & 0b10_0000 != 0 { raw_weight - 64 } else { raw_weight };
        let active = &record_bits[18..19] == "1";
        let legal = source < MOTOR_START && (INTERNAL_START..SLOT_COUNT).contains(&destination);
        let nodes_on = legal
            && genome.neural_slot_is_active(destination)
            && (source < INTERNAL_START || genome.neural_slot_is_active(source));
        records.push(GenomeRecord {
            index,
            start,
            bits: record_bits,
            source,
            destination,
            weight: weight_steps as f32 / 8.0,
            active,
            legal,
            effective: active && nodes_on,
        });
    }
    let active_records = records.iter().filter(|record| record.active).count();
    let effective_records = records.iter().filter(|record| record.effective).count();

    let channels = genome.pheromone_channels();
    let channel_bits = slice(channel_start, 9);
    let raw_channels = [
        bits_value(&channel_bits[0..3]),
        bits_value(&channel_bits[3..6]),
        bits_value(&channel_bits[6..9]),
    ];
    let normalized = raw_channels
        .iter()
        .zip(channels.iter())
        .any(|(raw, channel)| *raw != *channel as u32);

    let allele_bits = slice(allele_start, 5);
    let activity_bits = slice(activity_start, activity_length);
    let active_nodes = activity_bits.chars().filter(|bit| *bit == '1').count();
    let inactive_names: Vec<String> = (0..activity_length)
        .filter(|index| activity_bits.as_bytes().get(*index) == Some(&b'0'))
        .map(|index| {
            let slot = if index < MOTOR_START - INTERNAL_START {
                INTERNAL_START + index
            } else {
                MOTOR_START + index - (MOTOR_START - INTERNAL_START)
            };
            slot_label(slot)
        })
        .collect();

    let total_bytes_bits = GENOME_BYTES * 8;
    let reserve: String = (GENOME_BIT_COUNT..total_bytes_bits)
        .map(|index| {
            let byte = genome.bytes()[index / 8];
            if byte & (1 << (7 - index % 8)) != 0 { '1' } else { '0' }
        })
        .collect();

    let mut sections = vec![
        GenomeSection {
            key: "fiacao".to_owned(),
            name: "Fiação neural".to_owned(),
            start: 0,
            length: neural_bits,
            bits: String::new(),
            summary: format!(
                "64 registros de 19 bits · {active_records} com o bit ativo · {effective_records} entram no cérebro"
            ),
            note: "Cada registro: origem (6 bits), destino (6), peso (6, de −4 a +3,875) e ativo (1). Veja a tabela de registros.".to_owned(),
        },
        GenomeSection {
            key: "canais".to_owned(),
            name: "Canais químicos de P1, P2 e P3".to_owned(),
            start: channel_start,
            length: 9,
            bits: channel_bits.clone(),
            summary: format!(
                "P1 = C{} · P2 = C{} · P3 = C{}",
                channels[0], channels[1], channels[2]
            ),
            note: format!(
                "Três grupos de 3 bits ({}, {}, {}). Ficam nos 9 bits finais da fiação, sobrepostos ao registro 63.{}",
                raw_channels[0],
                raw_channels[1],
                raw_channels[2],
                if normalized {
                    " Houve repetição de canal: o motor avançou para o próximo canal livre, para P1, P2 e P3 serem diferentes."
                } else {
                    ""
                }
            ),
        },
        GenomeSection {
            key: "alcance".to_owned(),
            name: "Alelo do alcance visual".to_owned(),
            start: allele_start,
            length: 5,
            bits: allele_bits.clone(),
            summary: format!(
                "alelo {} de 31 · alcance-base {:.0} u",
                bits_value(&allele_bits),
                genome.food_sensor_range()
            ),
            note: "16 é o padrão (240 u). O corpo maior amplia o alcance efetivo.".to_owned(),
        },
        GenomeSection {
            key: "atividade".to_owned(),
            name: "Atividade dos nós neurais".to_owned(),
            start: activity_start,
            length: activity_length,
            bits: activity_bits,
            summary: format!("{active_nodes} de {activity_length} nós ligados"),
            note: if inactive_names.is_empty() {
                "Um bit por nó, na ordem I0–I15, T1–T3 e os 11 motores. Todos ligados.".to_owned()
            } else {
                format!(
                    "Um bit por nó, na ordem I0–I15, T1–T3 e os 11 motores. Desligados: {}.",
                    inactive_names.join(", ")
                )
            },
        },
        GenomeSection {
            key: "reserva".to_owned(),
            name: "Reserva".to_owned(),
            start: GENOME_BIT_COUNT,
            length: reserve.len(),
            bits: reserve,
            summary: "não usada".to_owned(),
            note: "Bits que sobram no último byte; ficam livres para regras futuras.".to_owned(),
        },
    ];
    if let Some(max_age) = max_age {
        sections.push(GenomeSection {
            key: "longevidade".to_owned(),
            name: "Longevidade (fora do genoma neural)".to_owned(),
            start: 0,
            length: 0,
            bits: String::new(),
            summary: format!("idade máxima {max_age} ticks"),
            note: "Alelo separado do genoma: não muda cor, similaridade nem PG.".to_owned(),
        });
    }
    GenomeMap {
        total_bits: GENOME_BIT_COUNT,
        channels,
        sections,
        records,
    }
}

fn ones_per_bit(world: &World) -> Vec<u32> {
    let mut ones = vec![0u32; GENOME_BIT_COUNT];
    for organism in &world.organisms {
        for (byte_index, byte) in organism.genome.bytes().iter().enumerate() {
            for bit_in_byte in 0..8 {
                let bit_index = byte_index * 8 + bit_in_byte;
                if bit_index >= GENOME_BIT_COUNT {
                    continue;
                }
                ones[bit_index] += ((byte >> (7 - bit_in_byte)) & 1) as u32;
            }
        }
    }
    ones
}

fn binary_entropy(probability: f32) -> f32 {
    if probability <= 0.0 || probability >= 1.0 {
        0.0
    } else {
        -probability * probability.log2() - (1.0 - probability) * (1.0 - probability).log2()
    }
}

fn capture_sample(world: &World) -> Sample {
    let population = world.organisms.len();
    let max_generation = world
        .organisms
        .iter()
        .map(|organism| organism.generation)
        .max()
        .unwrap_or(0);
    let genetic_diversity = if population == 0 {
        0.0
    } else {
        let ones = ones_per_bit(world);
        ones.iter()
            .map(|count| binary_entropy(*count as f32 / population as f32))
            .sum::<f32>()
            / ones.len() as f32
    };
    Sample {
        tick: world.tick,
        population,
        births_total: world.births_total,
        deaths_total: world.deaths_total,
        births_neural: world.births_by_neural_decision_total,
        births_limit: world.births_by_energy_limit_total,
        deaths_starvation: world.deaths_by_starvation_total,
        deaths_phagocytosis: world.deaths_by_phagocytosis_total,
        deaths_age: world.deaths_by_age_total,
        average_energy: world.average_energy(),
        average_age: world.average_age_ticks(),
        average_generation: world.average_generation(),
        max_generation,
        phytoplankton: world.food.len(),
        tps: 0.0,
        genetic_diversity,
        reproduction_minimum: world.reproduction_minimum,
    }
}

fn genetics_report(world: &World) -> Genetics {
    let population = world.organisms.len();
    if population == 0 {
        return Genetics {
            tick: world.tick,
            ..Genetics::default()
        };
    }
    let mut order: Vec<usize> = (0..population).collect();
    order.sort_by_key(|&index| world.organisms[index].id);

    // Grupos por similaridade com a representante (heurística de 98,5%).
    struct Group {
        representative: usize,
        members: Vec<usize>,
        similarity_sum: f32,
        generation_sum: f64,
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut group_of = vec![0usize; population];
    for &index in &order {
        let genome = &world.organisms[index].genome;
        let mut best: Option<(usize, f32)> = None;
        for (group_index, group) in groups.iter().enumerate() {
            let similarity = genome.similarity_to(&world.organisms[group.representative].genome);
            if similarity >= GENETIC_GROUP_THRESHOLD
                && best.map_or(true, |(_, current)| similarity > current)
            {
                best = Some((group_index, similarity));
            }
        }
        let generation = world.organisms[index].generation as f64;
        match best {
            Some((group_index, similarity)) => {
                let group = &mut groups[group_index];
                group.members.push(index);
                group.similarity_sum += similarity;
                group.generation_sum += generation;
                group_of[index] = group_index;
            }
            None => {
                group_of[index] = groups.len();
                groups.push(Group {
                    representative: index,
                    members: vec![index],
                    similarity_sum: 1.0,
                    generation_sum: generation,
                });
            }
        }
    }

    let mut group_infos: Vec<GroupInfo> = groups
        .iter()
        .map(|group| GroupInfo {
            color: world.organisms[group.representative].genome.genetic_hex(),
            count: group.members.len(),
            share: group.members.len() as f32 / population as f32,
            cohesion: group.similarity_sum / group.members.len() as f32,
            average_generation: (group.generation_sum / group.members.len() as f64) as f32,
            representative: world.organisms[group.representative].id,
        })
        .collect();
    group_infos.sort_by(|a, b| b.count.cmp(&a.count));
    let dominant = group_infos.first().cloned().unwrap_or_default();

    // Concordância do consenso: média do alelo majoritário por bit.
    let ones = ones_per_bit(world);
    let consensus = ones
        .iter()
        .map(|count| {
            let fraction = *count as f32 / population as f32;
            fraction.max(1.0 - fraction)
        })
        .sum::<f32>()
        / ones.len().max(1) as f32;

    // Circuitos e uso de slots.
    let mut pairs: HashMap<(usize, usize), (usize, f32, HashSet<usize>)> = HashMap::new();
    let mut used = vec![0usize; SLOT_COUNT];
    let mut as_source = vec![0usize; SLOT_COUNT];
    let mut as_destination = vec![0usize; SLOT_COUNT];
    for (index, organism) in world.organisms.iter().enumerate() {
        let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();
        let mut seen_used = vec![false; SLOT_COUNT];
        let mut seen_source = vec![false; SLOT_COUNT];
        let mut seen_destination = vec![false; SLOT_COUNT];
        for (connection, _) in organism.brain.visual_connections() {
            let source = connection.source.min(SLOT_COUNT - 1);
            let destination = connection.destination.min(SLOT_COUNT - 1);
            let entry = pairs
                .entry((source, destination))
                .or_insert_with(|| (0, 0.0, HashSet::new()));
            if seen_pairs.insert((source, destination)) {
                entry.0 += 1;
                entry.2.insert(group_of[index]);
            }
            entry.1 += connection.weight;
            seen_used[source] = true;
            seen_used[destination] = true;
            seen_source[source] = true;
            seen_destination[destination] = true;
        }
        for slot in 0..SLOT_COUNT {
            used[slot] += seen_used[slot] as usize;
            as_source[slot] += seen_source[slot] as usize;
            as_destination[slot] += seen_destination[slot] as usize;
        }
    }
    let mut connections: Vec<ConnectionInfo> = pairs
        .into_iter()
        .map(|((source, destination), (count, weight_sum, group_set))| ConnectionInfo {
            source,
            destination,
            frequency: count as f32 / population as f32,
            mean_weight: weight_sum / count.max(1) as f32,
            groups: group_set.len(),
        })
        .collect();
    connections.sort_by(|a, b| {
        b.frequency
            .partial_cmp(&a.frequency)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    connections.truncate(24);
    let mut slots: Vec<SlotInfo> = (0..SLOT_COUNT)
        .map(|slot| SlotInfo {
            slot,
            used: used[slot] as f32 / population as f32,
            as_source: as_source[slot] as f32 / population as f32,
            as_destination: as_destination[slot] as f32 / population as f32,
        })
        .collect();
    slots.sort_by(|a, b| b.used.partial_cmp(&a.used).unwrap_or(std::cmp::Ordering::Equal));

    Genetics {
        tick: world.tick,
        population,
        group_count: group_infos.len(),
        dominant_share: dominant.share,
        dominant_cohesion: dominant.cohesion,
        consensus,
        groups: group_infos.into_iter().take(30).collect(),
        connections,
        slots,
    }
}
