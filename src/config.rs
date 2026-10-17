/// Câmara lógica: ampliada para 4× a área anterior (2× em cada eixo).
pub const WORLD_WIDTH: f32 = 12_800.0;
pub const WORLD_HEIGHT: f32 = 7_200.0;
pub const MAX_FLOW_SPEED: f32 = 10.0;
pub const MAX_SWIM_SPEED: f32 = 8.0;
/// Alcance-base do sensor de alimento. O alelo do `Genome` o multiplica e o
/// corpo maior o amplia com eficiência volumétrica; este é o fenótipo das runs
/// antigas migradas quando a energia é zero.
pub const DEFAULT_FOOD_SENSOR_RANGE: f32 = 240.0;
/// Limites do alelo hereditário, antes do multiplicador corporal volumétrico.
/// O alcance efetivo pode ultrapassá-los.
pub const MIN_FOOD_SENSOR_RANGE: f32 = 120.0;
pub const MAX_FOOD_SENSOR_RANGE: f32 = 480.0;
