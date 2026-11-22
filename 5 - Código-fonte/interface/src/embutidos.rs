//! Presets de Sol e de Construção (V5.1.1) e os mundos embutidos.
//!
//! ```text
//! 4 - Dados/Sol/<nome>.ngusol          malha de luz + intensidade, basal e limiar
//! 4 - Dados/Construção/<nome>.nguconst paredes
//! 4 - Dados/Mapas/<nome>.ngumapa       mapa completo (construção + sol + corrente)
//! ```
//!
//! Os embutidos são gerados por código e regravados se faltarem, então
//! apagar um deles só vale até a próxima abertura da lista.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use new_gods_unlimited::environment::map::{MapDefinition, MAP_GRID_HEIGHT, MAP_GRID_WIDTH};
use new_gods_unlimited::config::{WORLD_HEIGHT, WORLD_WIDTH};
use macroquad::prelude::vec2;

use crate::arquivos::{self, caminho_seguro, nome_de_arquivo, PresetDeMapa};
use crate::engine::EnvSettings;

pub const EXTENSAO_SOL: &str = "ngusol";
pub const EXTENSAO_CONSTRUCAO: &str = "nguconst";

#[derive(Clone, Serialize, Deserialize)]
pub struct PresetDeSol {
    pub formato: u32,
    pub nome: String,
    pub sun_intensity: f32,
    pub sun_basal: f32,
    pub sun_threshold: f32,
    /// Malha de luz (0–255 por casa), compactada.
    pub sol: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PresetDeConstrucao {
    pub formato: u32,
    pub nome: String,
    /// Paredes (0/1 por casa), compactadas.
    pub paredes: String,
}

pub fn pasta_sol(base: &Path) -> PathBuf {
    base.join("Sol")
}

pub fn pasta_construcao(base: &Path) -> PathBuf {
    base.join("Construção")
}

impl PresetDeSol {
    pub fn novo(nome: &str, mapa: &MapDefinition, env: &EnvSettings) -> Self {
        Self {
            formato: 1,
            nome: nome.to_owned(),
            sun_intensity: env.sun_intensity,
            sun_basal: env.sun_basal,
            sun_threshold: env.sun_threshold,
            sol: arquivos::compactar(mapa.sunlight_values()),
        }
    }

    pub fn malha(&self) -> io::Result<Vec<u8>> {
        arquivos::descompactar(&self.sol)
    }
}

impl PresetDeConstrucao {
    pub fn novo(nome: &str, mapa: &MapDefinition) -> Self {
        Self {
            formato: 1,
            nome: nome.to_owned(),
            paredes: arquivos::compactar(mapa.wall_values()),
        }
    }

    pub fn paredes(&self) -> io::Result<Vec<u8>> {
        arquivos::descompactar(&self.paredes)
    }
}

fn salvar_json<T: Serialize>(pasta: &Path, nome: &str, extensao: &str, valor: &T) -> io::Result<String> {
    fs::create_dir_all(pasta)?;
    let arquivo = format!("{}.{extensao}", nome_de_arquivo(nome));
    let texto = serde_json::to_string_pretty(valor).map_err(io::Error::other)?;
    fs::write(pasta.join(&arquivo), texto)?;
    Ok(arquivo)
}

fn ler_json<T: for<'de> Deserialize<'de>>(pasta: &Path, arquivo: &str) -> io::Result<T> {
    let texto = fs::read_to_string(caminho_seguro(pasta, arquivo)?)?;
    serde_json::from_str(&texto).map_err(io::Error::other)
}

pub fn salvar_sol(base: &Path, preset: &PresetDeSol) -> io::Result<String> {
    salvar_json(&pasta_sol(base), &preset.nome, EXTENSAO_SOL, preset)
}

pub fn ler_sol(base: &Path, arquivo: &str) -> io::Result<PresetDeSol> {
    ler_json(&pasta_sol(base), arquivo)
}

pub fn salvar_construcao(base: &Path, preset: &PresetDeConstrucao) -> io::Result<String> {
    salvar_json(&pasta_construcao(base), &preset.nome, EXTENSAO_CONSTRUCAO, preset)
}

pub fn ler_construcao(base: &Path, arquivo: &str) -> io::Result<PresetDeConstrucao> {
    ler_json(&pasta_construcao(base), arquivo)
}

pub fn apagar(pasta: &Path, arquivo: &str) -> io::Result<()> {
    fs::remove_file(caminho_seguro(pasta, arquivo)?)
}

// ---------------------------------------------------------------------------
// Mundos embutidos
// ---------------------------------------------------------------------------

const W: usize = MAP_GRID_WIDTH;
const H: usize = MAP_GRID_HEIGHT;

/// Gerador determinístico simples, para os embutidos saírem sempre iguais.
struct Sorteio(u64);
impl Sorteio {
    fn proximo(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn pos(fx: f32, fy: f32) -> macroquad::prelude::Vec2 {
    vec2(fx * WORLD_WIDTH, fy * WORLD_HEIGHT)
}

fn sol_com_focos(focos: &[(f32, f32, f32, f32, f32)]) -> MapDefinition {
    let mut mapa = MapDefinition::empty();
    for &(fx, fy, raio, pico, forca) in focos {
        mapa.paint_sunlight(pos(fx, fy), raio, pico, forca, false);
    }
    mapa
}

/// (nome, mapa com a malha de luz, basal, intensidade, limiar)
fn sois_embutidos() -> Vec<(&'static str, MapDefinition, f32)> {
    let mut gradiente = vec![0u8; W * H];
    for y in 0..H {
        for x in 0..W {
            gradiente[y * W + x] = ((x as f32 / (W - 1) as f32) * 230.0) as u8;
        }
    }
    let mut mapa_gradiente = MapDefinition::empty();
    mapa_gradiente.set_sunlight_values(&gradiente);

    let mut metade = vec![0u8; W * H];
    for y in 0..H {
        for x in 0..W / 2 - 4 {
            metade[y * W + x] = 200;
        }
    }
    let mut mapa_metade = MapDefinition::empty();
    mapa_metade.set_sunlight_values(&metade);

    let mut sorteio = Sorteio(0x5011_A11C_E5u64);
    let penumbra: Vec<(f32, f32, f32, f32, f32)> = (0..40)
        .map(|_| (sorteio.proximo(), sorteio.proximo(), 350.0, 0.15, 0.2))
        .collect();

    vec![
        ("Foco central", sol_com_focos(&[(0.5, 0.5, 2_600.0, 0.9, 0.35)]), 0.01),
        (
            "Arquipélago",
            sol_com_focos(&[
                (0.15, 0.25, 900.0, 0.8, 0.4),
                (0.38, 0.70, 1_300.0, 0.9, 0.3),
                (0.55, 0.22, 700.0, 0.6, 0.5),
                (0.72, 0.62, 1_100.0, 0.85, 0.35),
                (0.88, 0.30, 600.0, 0.5, 0.6),
                (0.25, 0.82, 500.0, 0.4, 0.5),
            ]),
            0.01,
        ),
        ("Gradiente oeste-leste", mapa_gradiente, 0.01),
        (
            "Dois polos",
            sol_com_focos(&[(0.12, 0.5, 1_800.0, 0.9, 0.4), (0.88, 0.5, 1_800.0, 0.9, 0.4)]),
            0.01,
        ),
        ("Metade clara", mapa_metade, 0.005),
        ("Penumbra", sol_com_focos(&penumbra), 0.02),
    ]
}

fn parede(paredes: &mut [u8], x0: usize, y0: usize, x1: usize, y1: usize) {
    for y in y0.min(H)..y1.min(H) {
        for x in x0.min(W)..x1.min(W) {
            paredes[y * W + x] = 1;
        }
    }
}

fn construcoes_embutidas() -> Vec<(&'static str, Vec<u8>)> {
    let vazio = vec![0u8; W * H];
    let oasis = MapDefinition::default().wall_values().to_vec();

    // Labirinto: grade de corredores com portas em posições sorteadas.
    let mut labirinto = vec![0u8; W * H];
    let mut sorteio = Sorteio(0x1AB1_2147_0u64 | 1);
    for linha in 1..4 {
        let y = linha * H / 4;
        let mut x = 0;
        while x < W {
            let fim = (x + 40).min(W);
            let porta = x + 8 + (sorteio.proximo() * 20.0) as usize;
            parede(&mut labirinto, x, y, porta, y + 2);
            parede(&mut labirinto, porta + 10, y, fim, y + 2);
            x = fim;
        }
    }
    for coluna in 1..6 {
        let x = coluna * W / 6;
        let mut y = 0;
        while y < H {
            let fim = (y + 45).min(H);
            let porta = y + 8 + (sorteio.proximo() * 20.0) as usize;
            parede(&mut labirinto, x, y, x + 2, porta);
            parede(&mut labirinto, x, porta + 10, x + 2, fim);
            y = fim;
        }
    }

    // Duas câmaras ligadas por um corredor estreito.
    let mut camaras = vec![0u8; W * H];
    parede(&mut camaras, W / 2 - 1, 0, W / 2 + 1, H / 2 - 6);
    parede(&mut camaras, W / 2 - 1, H / 2 + 6, W / 2 + 1, H);

    // Espiral retangular até o centro.
    let mut espiral = vec![0u8; W * H];
    let (mut x0, mut y0, mut x1, mut y1) = (12usize, 12usize, W - 12, H - 12);
    let mut lado = 0;
    while x1 > x0 + 24 && y1 > y0 + 24 {
        match lado % 4 {
            0 => parede(&mut espiral, x0, y0, x1, y0 + 2),
            1 => parede(&mut espiral, x1 - 2, y0, x1, y1),
            2 => parede(&mut espiral, x0 + 16, y1 - 2, x1, y1),
            _ => {
                parede(&mut espiral, x0 + 16, y0 + 16, x0 + 18, y1);
                x0 += 16;
                y0 += 16;
                x1 -= 16;
                y1 -= 16;
            }
        }
        lado += 1;
    }

    // Recifes: blocos espalhados.
    let mut recifes = vec![0u8; W * H];
    let mut sorteio = Sorteio(0x2EC1_FE5u64);
    for _ in 0..45 {
        let x = 8 + (sorteio.proximo() * (W - 24) as f32) as usize;
        let y = 8 + (sorteio.proximo() * (H - 24) as f32) as usize;
        let largura = 3 + (sorteio.proximo() * 6.0) as usize;
        let altura = 3 + (sorteio.proximo() * 6.0) as usize;
        parede(&mut recifes, x, y, x + largura, y + altura);
    }

    vec![
        ("Vazio", vazio),
        ("Oásis original", oasis),
        ("Labirinto", labirinto),
        ("Duas câmaras", camaras),
        ("Espiral", espiral),
        ("Recifes", recifes),
    ]
}

fn mapa_de(construcao: &[u8], sol: &MapDefinition) -> MapDefinition {
    let mut mapa = MapDefinition::empty();
    mapa.set_wall_values(construcao);
    mapa.set_sunlight_values(sol.sunlight_values());
    mapa
}

/// Grava os embutidos que faltarem nas três pastas.
pub fn garantir_embutidos(base: &Path) {
    let padrao = EnvSettings::default();
    let sois = sois_embutidos();
    for (nome, mapa, basal) in &sois {
        let caminho = pasta_sol(base).join(format!("{nome}.{EXTENSAO_SOL}"));
        if !caminho.exists() {
            let env = EnvSettings { sun_basal: *basal, ..padrao.clone() };
            let _ = salvar_sol(base, &PresetDeSol::novo(nome, mapa, &env));
        }
    }
    let construcoes = construcoes_embutidas();
    for (nome, paredes) in &construcoes {
        let caminho = pasta_construcao(base).join(format!("{nome}.{EXTENSAO_CONSTRUCAO}"));
        if !caminho.exists() {
            let mut mapa = MapDefinition::empty();
            mapa.set_wall_values(paredes);
            let _ = salvar_construcao(base, &PresetDeConstrucao::novo(nome, &mapa));
        }
    }
    let sol = |nome: &str| sois.iter().find(|(n, _, _)| *n == nome).map(|(_, m, b)| (m.clone(), *b));
    let construcao = |nome: &str| construcoes.iter().find(|(n, _)| *n == nome).map(|(_, p)| p.clone());
    let combinacoes = [
        ("Oásis original", "Oásis original", "Foco central", "Oásis"),
        ("Labirinto com focos", "Labirinto", "Arquipélago", "Clássico"),
        ("Duas câmaras: clara e escura", "Duas câmaras", "Metade clara", "Clássico"),
        ("Espiral com foco central", "Espiral", "Foco central", "Clássico"),
        ("Recifes no arquipélago", "Recifes", "Arquipélago", "Esquerda"),
        ("Gradiente aberto", "Vazio", "Gradiente oeste-leste", "Clássico"),
    ];
    for (nome, nome_construcao, nome_sol, corrente) in combinacoes {
        let caminho = arquivos::pasta_mapas(base).join(format!("{nome}.{}", arquivos::EXTENSAO_MAPA));
        if caminho.exists() {
            continue;
        }
        let (Some(paredes), Some((malha, basal))) = (construcao(nome_construcao), sol(nome_sol)) else {
            continue;
        };
        let mapa = if nome == "Oásis original" { MapDefinition::default() } else { mapa_de(&paredes, &malha) };
        let env = EnvSettings { sun_basal: basal, flow_preset: corrente.to_owned(), ..padrao.clone() };
        if let Ok(preset) = PresetDeMapa::novo(nome, &mapa, &env) {
            let _ = arquivos::salvar_preset(base, &preset);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embutidos_tem_tamanho_certo() {
        for (_, paredes) in construcoes_embutidas() {
            assert_eq!(paredes.len(), W * H);
        }
        for (_, mapa, _) in sois_embutidos() {
            assert!(mapa.sunlight_values().iter().any(|v| *v > 0));
        }
    }
}
