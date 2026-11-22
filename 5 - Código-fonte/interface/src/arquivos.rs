//! Presets de mapa e runs salvas (V5.0.3).
//!
//! Tudo fica em pastas comuns, para dar para copiar entre computadores:
//!
//! ```text
//! 4 - Dados/Mapas/<nome>.ngumapa   preset de mapa: paredes, sol e corrente (JSON)
//! 4 - Dados/Runs/<nome>.ngurun     run salva: configurações, telemetria e o mundo
//! ```
//!
//! Se o executável estiver fora do projeto, as pastas ficam dentro da pasta de
//! dados do aplicativo. Importar um mapa é copiar o arquivo para a pasta; exportar
//! é copiar de lá.

use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use new_gods_unlimited::environment::map::MapDefinition;
use new_gods_unlimited::simulation::world::World;

use crate::engine::{EnvSettings, InitSettings, Sample};

const MAGIA_RUN: &[u8; 8] = b"NGURUN01";
pub const EXTENSAO_MAPA: &str = "ngumapa";
pub const EXTENSAO_RUN: &str = "ngurun";

#[derive(Clone, Serialize, Deserialize)]
pub struct PresetDeMapa {
    pub formato: u32,
    pub nome: String,
    pub flow_on: bool,
    pub flow_intensity: f32,
    pub flow_preset: String,
    /// Sol (V5.1.1); mapas antigos abrem com os valores padrão.
    #[serde(default = "um")]
    pub sun_intensity: f32,
    #[serde(default = "basal_padrao")]
    pub sun_basal: f32,
    #[serde(default)]
    pub sun_threshold: f32,
    /// Mapa binário (`MapDefinition::write_to`) compactado por repetição.
    pub mapa: String,
}

impl PresetDeMapa {
    pub fn novo(nome: &str, mapa: &MapDefinition, env: &EnvSettings) -> io::Result<Self> {
        let mut bytes = Vec::new();
        mapa.write_to(&mut bytes)?;
        Ok(Self {
            formato: 1,
            nome: nome.to_owned(),
            flow_on: env.flow_on,
            flow_intensity: env.flow_intensity,
            flow_preset: env.flow_preset.clone(),
            sun_intensity: env.sun_intensity,
            sun_basal: env.sun_basal,
            sun_threshold: env.sun_threshold,
            mapa: compactar(&bytes),
        })
    }

    pub fn mapa(&self) -> io::Result<MapDefinition> {
        let bytes = descompactar(&self.mapa)?;
        MapDefinition::read_from(&mut Cursor::new(bytes))
    }
}

fn um() -> f32 {
    1.0
}

fn basal_padrao() -> f32 {
    0.01
}

#[derive(Clone, Serialize)]
pub struct ArquivoListado {
    pub file: String,
    pub name: String,
    pub bytes: u64,
    /// Segundos desde 1970.
    pub modified: u64,
}

#[derive(Clone, Default, Serialize)]
pub struct Armazenamento {
    pub maps_folder: String,
    pub runs_folder: String,
    pub map_presets: Vec<ArquivoListado>,
    pub runs: Vec<ArquivoListado>,
    pub sun_presets: Vec<ArquivoListado>,
    pub build_presets: Vec<ArquivoListado>,
}

#[derive(Serialize, Deserialize)]
pub struct CabecalhoDaRun {
    pub formato: u32,
    pub nome: String,
    pub env: EnvSettings,
    pub init: InitSettings,
    pub run_id: u64,
    pub telemetria: Vec<Sample>,
}

pub fn pasta_mapas(base: &Path) -> PathBuf {
    base.join("Mapas")
}

pub fn pasta_runs(base: &Path) -> PathBuf {
    base.join("Runs")
}

/// Nome de arquivo seguro no Windows, a partir do nome digitado.
pub fn nome_de_arquivo(nome: &str) -> String {
    let limpo: String = nome
        .trim()
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c })
        .take(60)
        .collect();
    if limpo.trim().is_empty() {
        "sem nome".to_owned()
    } else {
        limpo.trim().to_owned()
    }
}

pub fn listar(base: &Path) -> Armazenamento {
    Armazenamento {
        maps_folder: pasta_mapas(base).display().to_string(),
        runs_folder: pasta_runs(base).display().to_string(),
        map_presets: listar_pasta(&pasta_mapas(base), EXTENSAO_MAPA),
        runs: listar_pasta(&pasta_runs(base), EXTENSAO_RUN),
        sun_presets: listar_pasta(&crate::embutidos::pasta_sol(base), crate::embutidos::EXTENSAO_SOL),
        build_presets: listar_pasta(
            &crate::embutidos::pasta_construcao(base),
            crate::embutidos::EXTENSAO_CONSTRUCAO,
        ),
    }
}

pub fn listar_pasta(pasta: &Path, extensao: &str) -> Vec<ArquivoListado> {
    let Ok(entradas) = fs::read_dir(pasta) else {
        return Vec::new();
    };
    let mut lista: Vec<ArquivoListado> = entradas
        .flatten()
        .filter_map(|entrada| {
            let caminho = entrada.path();
            if caminho.extension()?.to_str()? != extensao {
                return None;
            }
            let meta = entrada.metadata().ok()?;
            Some(ArquivoListado {
                file: caminho.file_name()?.to_str()?.to_owned(),
                name: caminho.file_stem()?.to_str()?.to_owned(),
                bytes: meta.len(),
                modified: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            })
        })
        .collect();
    lista.sort_by(|a, b| b.modified.cmp(&a.modified));
    lista
}

/// Só aceita um nome simples de arquivo dentro da pasta (sem subpastas).
pub fn caminho_seguro(pasta: &Path, arquivo: &str) -> io::Result<PathBuf> {
    if arquivo.is_empty() || arquivo.contains(['/', '\\']) || arquivo.contains("..") {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "nome de arquivo inválido"));
    }
    Ok(pasta.join(arquivo))
}

pub fn salvar_preset(base: &Path, preset: &PresetDeMapa) -> io::Result<String> {
    let pasta = pasta_mapas(base);
    fs::create_dir_all(&pasta)?;
    let arquivo = format!("{}.{EXTENSAO_MAPA}", nome_de_arquivo(&preset.nome));
    let texto = serde_json::to_string_pretty(preset).map_err(io::Error::other)?;
    fs::write(pasta.join(&arquivo), texto)?;
    Ok(arquivo)
}

pub fn ler_preset(base: &Path, arquivo: &str) -> io::Result<PresetDeMapa> {
    let texto = fs::read_to_string(caminho_seguro(&pasta_mapas(base), arquivo)?)?;
    serde_json::from_str(&texto).map_err(io::Error::other)
}

pub fn apagar_preset(base: &Path, arquivo: &str) -> io::Result<()> {
    fs::remove_file(caminho_seguro(&pasta_mapas(base), arquivo)?)
}

pub fn salvar_run(base: &Path, cabecalho: &CabecalhoDaRun, mundo: &World) -> io::Result<String> {
    let pasta = pasta_runs(base);
    fs::create_dir_all(&pasta)?;
    let arquivo = format!("{}.{EXTENSAO_RUN}", nome_de_arquivo(&cabecalho.nome));
    let json = serde_json::to_vec(cabecalho).map_err(io::Error::other)?;
    // Grava num temporário e troca no fim: um desligamento no meio não
    // destrói a run anterior com o mesmo nome.
    let temporario = pasta.join(format!("{arquivo}.tmp"));
    {
        let mut saida = io::BufWriter::new(fs::File::create(&temporario)?);
        saida.write_all(MAGIA_RUN)?;
        saida.write_all(&(json.len() as u32).to_le_bytes())?;
        saida.write_all(&json)?;
        mundo.write_checkpoint(&mut saida)?;
        saida.flush()?;
    }
    fs::rename(&temporario, pasta.join(&arquivo))?;
    Ok(arquivo)
}

pub fn ler_run(base: &Path, arquivo: &str) -> io::Result<(CabecalhoDaRun, World)> {
    let bytes = fs::read(caminho_seguro(&pasta_runs(base), arquivo)?)?;
    let mut leitor = Cursor::new(bytes);
    let mut magia = [0u8; 8];
    leitor.read_exact(&mut magia)?;
    if &magia != MAGIA_RUN {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "não é uma run da V5"));
    }
    let mut tamanho = [0u8; 4];
    leitor.read_exact(&mut tamanho)?;
    let mut json = vec![0u8; u32::from_le_bytes(tamanho) as usize];
    leitor.read_exact(&mut json)?;
    let cabecalho: CabecalhoDaRun = serde_json::from_slice(&json).map_err(io::Error::other)?;
    let mundo = World::read_checkpoint(&mut leitor)?;
    Ok((cabecalho, mundo))
}

/// Compactação por repetição: "quantidade*valor," (o mapa é quase todo zeros).
pub fn compactar(bytes: &[u8]) -> String {
    let mut texto = String::new();
    let mut indice = 0;
    while indice < bytes.len() {
        let valor = bytes[indice];
        let mut fim = indice + 1;
        while fim < bytes.len() && bytes[fim] == valor {
            fim += 1;
        }
        texto.push_str(&format!("{}*{},", fim - indice, valor));
        indice = fim;
    }
    texto
}

pub fn descompactar(texto: &str) -> io::Result<Vec<u8>> {
    let invalido = || io::Error::new(io::ErrorKind::InvalidData, "mapa corrompido");
    let mut bytes = Vec::new();
    for pedaco in texto.split(',').filter(|p| !p.is_empty()) {
        let (quantidade, valor) = pedaco.split_once('*').ok_or_else(invalido)?;
        let quantidade: usize = quantidade.parse().map_err(|_| invalido())?;
        let valor: u8 = valor.parse().map_err(|_| invalido())?;
        if bytes.len() + quantidade > 4_000_000 {
            return Err(invalido());
        }
        bytes.extend(std::iter::repeat_n(valor, quantidade));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compactacao_volta_igual() {
        let bytes = vec![0, 0, 0, 1, 1, 0, 5, 5, 5, 5];
        assert_eq!(descompactar(&compactar(&bytes)).unwrap(), bytes);
    }

    #[test]
    fn preset_preserva_o_mapa() {
        let mapa = MapDefinition::default();
        let preset = PresetDeMapa::novo("teste", &mapa, &EnvSettings::default()).unwrap();
        let volta = preset.mapa().unwrap();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        mapa.write_to(&mut a).unwrap();
        volta.write_to(&mut b).unwrap();
        assert_eq!(a, b);
    }
}
