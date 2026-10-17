//! Benchmark headless e reproduzível do núcleo da simulação.
//!
//! Exemplo:
//! `cargo run --release --bin benchmark_simulacao -- --cenario realista`

use std::{
    env,
    fs::{self, File},
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use new_gods_unlimited::{
    environment::{
        flow::{FlowField, FlowPreset},
        map::MapDefinition,
    },
    simulation::{
        genome::{
            GENOME_BYTES, Genome, LEGACY_V1_GENOME_BYTES, PRE_FOOD_SENSOR_RANGE_GENOME_BYTES,
            PRE_NEURAL_NODE_ACTIVITY_GENOME_BYTES,
        },
        world::{
            AntiCrammingConfig, DEFAULT_MENTAL_ENERGY_COST, FOOD_ENERGY,
            InitialNeuralConfiguration, MAX_ENERGY, MIN_EXCLUSION_DISTANCE, TickProfile, World,
        },
    },
};

const RUN_MAGIC: [u8; 8] = *b"ECSRUN01";
const MAX_RUN_FORMAT_VERSION: u32 = 17;
const MAX_VALIDATED_GENOMES: usize = 20_000;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Base,
    Realistic,
    Stress,
}

impl Scenario {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "base" => Ok(Self::Base),
            "realista" => Ok(Self::Realistic),
            "estresse" => Ok(Self::Stress),
            _ => Err(format!(
                "cenário inválido: {value}. Use base, realista ou estresse."
            )),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Realistic => "realista",
            Self::Stress => "estresse",
        }
    }

    fn defaults(self) -> ScenarioDefaults {
        match self {
            Self::Base => ScenarioDefaults {
                population: 100,
                desert_emission: 0.0,
                fertile_emission: 0.0,
                food_lifetime: 600,
                food_energy: 0.0,
                reproduction_minimum: 1_000_000.0,
                reproduction_limit: MAX_ENERGY,
                anti_cramming: AntiCrammingConfig::default(),
            },
            Self::Realistic => ScenarioDefaults {
                population: 100,
                desert_emission: 0.12,
                fertile_emission: 0.24,
                food_lifetime: 600,
                food_energy: FOOD_ENERGY,
                reproduction_minimum: MAX_ENERGY,
                reproduction_limit: 5_000.0,
                anti_cramming: AntiCrammingConfig::default(),
            },
            Self::Stress => ScenarioDefaults {
                population: 300,
                desert_emission: 0.48,
                fertile_emission: 0.96,
                food_lifetime: 1_200,
                food_energy: FOOD_ENERGY,
                reproduction_minimum: MAX_ENERGY,
                reproduction_limit: 5_000.0,
                anti_cramming: AntiCrammingConfig {
                    enabled: true,
                    force: 1.0,
                    exclusion_distance: MIN_EXCLUSION_DISTANCE,
                },
            },
        }
    }
}

#[derive(Clone, Copy)]
struct ScenarioDefaults {
    population: usize,
    desert_emission: f32,
    fertile_emission: f32,
    food_lifetime: u32,
    food_energy: f32,
    reproduction_minimum: f32,
    reproduction_limit: f32,
    anti_cramming: AntiCrammingConfig,
}

struct Options {
    scenario: Scenario,
    population: Option<usize>,
    ticks: u64,
    warmup_ticks: u64,
    repetitions: usize,
    seed: u64,
    validated_run: Option<PathBuf>,
    write_report: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scenario: Scenario::Realistic,
            population: None,
            ticks: 2_000,
            warmup_ticks: 250,
            repetitions: 3,
            seed: 46_349_581_402_012_806_50,
            validated_run: None,
            write_report: true,
        }
    }
}

struct Measurement {
    elapsed: Duration,
    final_population: usize,
    final_food: usize,
    digest: u64,
}

fn main() -> ExitCode {
    if env::args()
        .skip(1)
        .any(|argument| matches!(argument.as_str(), "--ajuda" | "-h"))
    {
        print_usage();
        return ExitCode::SUCCESS;
    }
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Erro no benchmark: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let options = parse_options(env::args().skip(1))?;
    let defaults = options.scenario.defaults();
    let population = options.population.unwrap_or(defaults.population);
    if population == 0 || options.ticks == 0 || options.repetitions == 0 {
        return Err("população, ticks e repetições precisam ser maiores que zero.".into());
    }

    let validated_run = match options.validated_run.clone() {
        Some(path) => path,
        None => find_most_advanced_run(Path::new("data/runs"))?,
    };
    let founder_genomes = load_founder_genomes(&validated_run)?;
    if founder_genomes.is_empty() {
        return Err(format!(
            "a run validada {} não contém organismos para formar os fundadores.",
            validated_run.display()
        ));
    }

    println!("New Gods Unlimited — medição de desempenho");
    println!("Cenário: {}", options.scenario.name());
    println!(
        "Fundadores: Val 2 + {} genomas extraídos de {}",
        founder_genomes.len(),
        validated_run.display()
    );
    println!(
        "Carga: {population} células iniciais, {} ticks medidos, {} de aquecimento, {} repetição(ões), seed {}",
        options.ticks, options.warmup_ticks, options.repetitions, options.seed
    );

    let flow = FlowField::new(options.seed)
        .with_preset(FlowPreset::Classic)
        .with_intensity(1.0);
    let mut unprofiled = Vec::with_capacity(options.repetitions);
    let mut profiled = TickProfile::default();
    let mut profile_elapsed = Duration::ZERO;
    let mut deterministic = true;

    for repetition in 0..options.repetitions {
        let measured = measure_without_instrumentation(
            options.seed,
            population,
            &founder_genomes,
            &flow,
            defaults,
            options.warmup_ticks,
            options.ticks,
        );
        let (profile, profiled_measurement) = measure_with_profile(
            options.seed,
            population,
            &founder_genomes,
            &flow,
            defaults,
            options.warmup_ticks,
            options.ticks,
        );
        deterministic &= measured.digest == profiled_measurement.digest;
        profile_elapsed += profiled_measurement.elapsed;
        profiled.merge(&profile);
        println!(
            "  repetição {}: {:.1} ticks/s; população final {}; alimento final {}; estado {:016x}",
            repetition + 1,
            ticks_per_second(options.ticks, measured.elapsed),
            measured.final_population,
            measured.final_food,
            measured.digest,
        );
        unprofiled.push(measured);
    }

    let median_tps = median_ticks_per_second(&unprofiled, options.ticks);
    let mean_tps = unprofiled
        .iter()
        .map(|measurement| ticks_per_second(options.ticks, measurement.elapsed))
        .sum::<f64>()
        / unprofiled.len() as f64;
    println!(
        "\nResultado sem instrumentação: mediana {median_tps:.1} ticks/s; média {mean_tps:.1} ticks/s."
    );
    println!(
        "Verificação: tick normal e tick perfilado terminaram no mesmo estado em todas as repetições: {}.",
        if deterministic { "sim" } else { "NÃO" }
    );
    println!("\nDistribuição do tempo (execução perfilada; relógios acrescentam overhead):");
    let profile_total = profiled.total();
    for (name, duration) in profiled.phases() {
        let share = if profile_total.is_zero() {
            0.0
        } else {
            duration.as_secs_f64() / profile_total.as_secs_f64() * 100.0
        };
        let microseconds_per_tick = duration.as_secs_f64() * 1_000_000.0
            / (options.ticks * options.repetitions as u64) as f64;
        println!("  {name:24} {share:5.1}%  {microseconds_per_tick:8.1} µs/tick");
    }

    if options.write_report {
        let path = write_report(
            &options,
            population,
            &validated_run,
            founder_genomes.len(),
            &unprofiled,
            &profiled,
            profile_elapsed,
            deterministic,
        )?;
        println!("\nRelatório bruto salvo em {}", path.display());
    }
    Ok(())
}

fn make_world(
    seed: u64,
    population: usize,
    founders: &[Genome],
    defaults: ScenarioDefaults,
) -> World {
    let mut world = World::new_with_initial_population(
        seed,
        population,
        MapDefinition::default(),
        InitialNeuralConfiguration::ValidatedV2,
        founders,
    );
    world.set_anti_cramming(defaults.anti_cramming);
    world
}

fn advance(
    world: &mut World,
    flow: &FlowField,
    defaults: ScenarioDefaults,
    profiled: bool,
) -> TickProfile {
    if profiled {
        world.tick_profiled(
            flow,
            defaults.desert_emission,
            defaults.fertile_emission,
            defaults.food_lifetime,
            defaults.food_energy,
            DEFAULT_MENTAL_ENERGY_COST,
            defaults.reproduction_minimum,
            defaults.reproduction_limit,
        )
    } else {
        world.tick(
            flow,
            defaults.desert_emission,
            defaults.fertile_emission,
            defaults.food_lifetime,
            defaults.food_energy,
            DEFAULT_MENTAL_ENERGY_COST,
            defaults.reproduction_minimum,
            defaults.reproduction_limit,
        );
        TickProfile::default()
    }
}

fn measure_without_instrumentation(
    seed: u64,
    population: usize,
    founders: &[Genome],
    flow: &FlowField,
    defaults: ScenarioDefaults,
    warmup_ticks: u64,
    ticks: u64,
) -> Measurement {
    let mut world = make_world(seed, population, founders, defaults);
    for _ in 0..warmup_ticks {
        advance(&mut world, flow, defaults, false);
    }
    let started = Instant::now();
    for _ in 0..ticks {
        advance(&mut world, flow, defaults, false);
    }
    measurement(world, started.elapsed())
}

fn measure_with_profile(
    seed: u64,
    population: usize,
    founders: &[Genome],
    flow: &FlowField,
    defaults: ScenarioDefaults,
    warmup_ticks: u64,
    ticks: u64,
) -> (TickProfile, Measurement) {
    let mut world = make_world(seed, population, founders, defaults);
    for _ in 0..warmup_ticks {
        advance(&mut world, flow, defaults, false);
    }
    let started = Instant::now();
    let mut profile = TickProfile::default();
    for _ in 0..ticks {
        profile.merge(&advance(&mut world, flow, defaults, true));
    }
    let result = measurement(world, started.elapsed());
    (profile, result)
}

fn measurement(world: World, elapsed: Duration) -> Measurement {
    let mut checkpoint = Vec::new();
    world
        .write_checkpoint(&mut checkpoint)
        .expect("Vec não falha ao receber checkpoint");
    Measurement {
        elapsed,
        final_population: world.organisms.len(),
        final_food: world.food.len(),
        digest: fnv1a(&checkpoint),
    }
}

fn ticks_per_second(ticks: u64, elapsed: Duration) -> f64 {
    ticks as f64 / elapsed.as_secs_f64()
}

fn median_ticks_per_second(measurements: &[Measurement], ticks: u64) -> f64 {
    let mut values: Vec<f64> = measurements
        .iter()
        .map(|measurement| ticks_per_second(ticks, measurement.elapsed))
        .collect();
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn find_most_advanced_run(directory: &Path) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("não foi possível ler {}: {error}", directory.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("ecosrun") {
            continue;
        }
        if let Ok(tick) = read_run_tick(&path) {
            candidates.push((tick, path));
        }
    }
    candidates
        .into_iter()
        .max_by_key(|(tick, _)| *tick)
        .map(|(_, path)| path)
        .ok_or_else(|| {
            format!(
                "nenhuma run .ecosrun compatível em {}. Use --run-validado CAMINHO.",
                directory.display()
            )
        })
}

fn read_run_tick(path: &Path) -> io::Result<u64> {
    let mut reader = BufReader::new(File::open(path)?);
    let version = read_run_header(&mut reader)?;
    if !(1..=MAX_RUN_FORMAT_VERSION).contains(&version) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "versão de run não suportada pelo benchmark",
        ));
    }
    skip_exact(&mut reader, 8)?; // id da run
    read_u64(&mut reader)
}

fn load_founder_genomes(path: &Path) -> Result<Vec<Genome>, String> {
    let mut reader = BufReader::new(
        File::open(path)
            .map_err(|error| format!("não foi possível abrir {}: {error}", path.display()))?,
    );
    let version = read_run_header(&mut reader).map_err(|error| error.to_string())?;
    if !(1..=MAX_RUN_FORMAT_VERSION).contains(&version) {
        return Err(format!(
            "{} usa versão {version}; o benchmark lê runs entre V1 e V{MAX_RUN_FORMAT_VERSION}.",
            path.display()
        ));
    }
    skip_exact(&mut reader, 8 + 8 + 4) // id, tick e população do cabeçalho
        .map_err(|error| error.to_string())?;
    skip_engine_settings(&mut reader, version).map_err(|error| error.to_string())?;
    let world = read_world_for_run_version(&mut reader, version)
        .map_err(|error| format!("checkpoint inválido em {}: {error}", path.display()))?;
    Ok(world
        .organisms
        .into_iter()
        .map(|organism| organism.genome)
        .collect())
}

/// Avança pelo mesmo cabeçalho de configurações que `app.rs` grava. Mantê-lo
/// aqui evita inicializar a interface gráfica apenas para obter os genomas de
/// uma população validada; a versão do formato torna a migração explícita.
fn skip_engine_settings(reader: &mut impl Read, version: u32) -> io::Result<()> {
    skip_exact(reader, 8)?; // seed
    if version >= 5 {
        skip_exact(reader, 1)?; // modo Amadeus
    }
    if version >= 8 {
        skip_exact(reader, 4)?; // configuração neural inicial
    }
    if version >= 7 {
        skip_exact(reader, 4)?; // preset de corrente
    }
    // intensidade, anti-cramming, metabolismo, emissões, alimento,
    // reprodução e população inicial.
    skip_exact(reader, 4 + 1 + 4 + 4 + 4 + 4)?;
    if version >= 4 {
        skip_exact(reader, 4)?; // emissão da área fértil
    }
    skip_exact(reader, 4 + 4 + 4 + 4 + 8)?;
    if version >= 9 {
        let count = read_u32(reader)? as usize;
        if count > MAX_VALIDATED_GENOMES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "lista de genomas excede o limite seguro",
            ));
        }
        let bytes_per_genome = if version >= 17 {
            GENOME_BYTES
        } else if version >= 16 {
            PRE_NEURAL_NODE_ACTIVITY_GENOME_BYTES
        } else if version >= 10 {
            PRE_FOOD_SENSOR_RANGE_GENOME_BYTES
        } else {
            LEGACY_V1_GENOME_BYTES
        };
        skip_exact(reader, count * bytes_per_genome)?;
    }
    Ok(())
}

fn read_world_for_run_version(reader: &mut impl Read, version: u32) -> io::Result<World> {
    match version {
        1 => World::read_checkpoint_v1(reader),
        2 => World::read_checkpoint_v2(reader),
        3 => World::read_checkpoint_v3(reader),
        4 | 5 => World::read_checkpoint_v4(reader),
        6..=9 => World::read_checkpoint_pre_pheromone(reader),
        10 => World::read_checkpoint_pre_donation(reader),
        11 => World::read_checkpoint_pre_world_expansion(reader),
        12 => World::read_checkpoint_pre_longevity(reader),
        13 => World::read_checkpoint_pre_event_counters(reader),
        14 => World::read_checkpoint_pre_age_counters(reader),
        15 => World::read_checkpoint_pre_food_sensor_range(reader),
        16 => World::read_checkpoint_pre_neural_node_activity(reader),
        17 => World::read_checkpoint(reader),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "versão de checkpoint desconhecida",
        )),
    }
}

fn read_run_header(reader: &mut impl Read) -> io::Result<u32> {
    let mut magic = [0; RUN_MAGIC.len()];
    reader.read_exact(&mut magic)?;
    let version = read_u32(reader)?;
    if magic != RUN_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "não é uma run ECSRUN01",
        ));
    }
    Ok(version)
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

fn skip_exact(reader: &mut impl Read, bytes: usize) -> io::Result<()> {
    let mut discarded = vec![0; bytes];
    reader.read_exact(&mut discarded)
}

fn parse_options(arguments: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--cenario" => {
                options.scenario = Scenario::parse(&option_value("--cenario", &mut arguments)?)?
            }
            "--populacao" => {
                options.population = Some(parse_number(
                    "--populacao",
                    option_value("--populacao", &mut arguments)?,
                )?)
            }
            "--ticks" => {
                options.ticks = parse_number("--ticks", option_value("--ticks", &mut arguments)?)?
            }
            "--aquecimento" => {
                options.warmup_ticks = parse_number(
                    "--aquecimento",
                    option_value("--aquecimento", &mut arguments)?,
                )?
            }
            "--repeticoes" => {
                options.repetitions = parse_number(
                    "--repeticoes",
                    option_value("--repeticoes", &mut arguments)?,
                )?
            }
            "--seed" => {
                options.seed = parse_number("--seed", option_value("--seed", &mut arguments)?)?
            }
            "--run-validado" => {
                options.validated_run = Some(PathBuf::from(option_value(
                    "--run-validado",
                    &mut arguments,
                )?))
            }
            "--sem-arquivo" => options.write_report = false,
            "--ajuda" | "-h" => {
                return Err("a ajuda é tratada antes do benchmark iniciar.".into());
            }
            _ => return Err(format!("opção desconhecida: {argument}")),
        }
    }
    Ok(options)
}

fn print_usage() {
    println!("Uso: cargo run --release --bin benchmark_simulacao -- [opções]");
    println!("  --cenario base|realista|estresse");
    println!("  --populacao N --ticks N --aquecimento N --repeticoes N --seed N");
    println!("  --run-validado CAMINHO   usa esta população salva como fundadora Val 2");
    println!("  --sem-arquivo             não grava JSON em data/benchmarks");
}

fn option_value(
    flag: &str,
    arguments: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("{flag} exige um valor."))
}

fn parse_number<T: std::str::FromStr>(flag: &str, value: String) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("valor inválido para {flag}: {value}"))
}

fn write_report(
    options: &Options,
    population: usize,
    validated_run: &Path,
    founder_count: usize,
    measurements: &[Measurement],
    profile: &TickProfile,
    profile_elapsed: Duration,
    deterministic: bool,
) -> Result<PathBuf, String> {
    let directory = Path::new("data/benchmarks");
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let path = directory.join(format!("{}-{timestamp}.json", options.scenario.name()));
    let results = measurements
        .iter()
        .map(|measurement| {
            format!(
                "{{\"elapsed_ms\":{:.3},\"ticks_per_second\":{:.3},\"final_population\":{},\"final_food\":{},\"digest\":\"{:016x}\"}}",
                measurement.elapsed.as_secs_f64() * 1_000.0,
                ticks_per_second(options.ticks, measurement.elapsed),
                measurement.final_population,
                measurement.final_food,
                measurement.digest,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let phases = profile
        .phases()
        .into_iter()
        .map(|(name, duration)| format!("\"{name}\":{:.3}", duration.as_secs_f64() * 1_000.0))
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        concat!(
            "{{\n",
            "  \"scenario\": \"{}\",\n",
            "  \"seed\": {},\n",
            "  \"initial_population\": {},\n",
            "  \"measured_ticks\": {},\n",
            "  \"warmup_ticks\": {},\n",
            "  \"validated_founder_count\": {},\n",
            "  \"validated_run\": \"{}\",\n",
            "  \"configuration\": \"ValidatedV2\",\n",
            "  \"normal_and_profiled_states_match\": {},\n",
            "  \"profiled_wall_time_ms\": {:.3},\n",
            "  \"results\": [{}],\n",
            "  \"phase_time_ms\": {{{}}}\n",
            "}}\n"
        ),
        options.scenario.name(),
        options.seed,
        population,
        options.ticks,
        options.warmup_ticks,
        founder_count,
        validated_run.display().to_string().replace('\\', "\\\\"),
        deterministic,
        profile_elapsed.as_secs_f64() * 1_000.0,
        results,
        phases,
    );
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(path)
}
