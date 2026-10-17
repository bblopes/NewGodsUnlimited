/// Câmara lógica: ampliada para 4× a área anterior (2× em cada eixo).
pub const WORLD_WIDTH: f32 = 12_800.0;
pub const WORLD_HEIGHT: f32 = 7_200.0;
pub const MAX_FLOW_SPEED: f32 = 10.0;
pub const MAX_SWIM_SPEED: f32 = 8.0;
/// Alcance-base do sensor de alimento. Cada organismo pode desviá-lo por um
/// alelo do `Genome`; este valor também é o fenótipo de runs antigas migradas.
pub const DEFAULT_FOOD_SENSOR_RANGE: f32 = 240.0;
/// Limites de segurança usados pela grade espacial. O alelo usa uma curva
/// logarítmica e ocupa, na prática, aproximadamente 120–460 unidades.
pub const MIN_FOOD_SENSOR_RANGE: f32 = 120.0;
pub const MAX_FOOD_SENSOR_RANGE: f32 = 480.0;
