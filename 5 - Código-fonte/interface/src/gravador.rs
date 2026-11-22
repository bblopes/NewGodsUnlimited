//! Gravador de sessão — fase 1.
//!
//! Especificação: `PROPOSTA_GRAVADOR_DE_SESSAO.md`. Cada gravação vira uma
//! pasta com arquivos de texto (JSON Lines), legíveis por gente e pelo Claude:
//!
//! ```text
//! gravacao-2026-11-21T23-40-05Z-sistema/
//!   manifesto.json        versão, seed, condições no início e no fim
//!   eventos_eco.jsonl     o que mudou o mundo (parâmetros, mapa, inserções…)
//!   eventos_sis.jsonl     só no modo sistema: comandos da interface, mouse, teclado
//!   marcas.jsonl          momentos marcados pelo usuário (F8)
//!   telemetria.jsonl      uma amostra demográfica por linha
//!   mapas/                o mapa em texto no início e a cada "Aplicar mapa"
//!   RESUMO.md             linha do tempo pronta para ler primeiro
//! ```
//!
//! Regra de separação: o registro ecológico guarda o *resultado* de uma ação
//! que muda o mundo; o registro de sistema guarda *como* o usuário chegou lá
//! (pincel, raio, velocidade, pausa…). Uma pincelada nunca é evento ecológico;
//! o "Aplicar mapa" é.
//!
//! Os eventos são esparsos e vão por `BufWriter`; o custo no laço do motor é
//! pequeno, mas ainda precisa ser medido (§10 da especificação).

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use new_gods_unlimited::environment::map::{MapDefinition, MAP_GRID_HEIGHT, MAP_GRID_WIDTH};

/// Quantos itens a linha do tempo do RESUMO.md guarda (o .jsonl guarda tudo).
const LINHA_DO_TEMPO_MAX: usize = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModoGravacao {
    /// Só o que muda o mundo + demografia.
    Ecossistema,
    /// Tudo do modo ecossistema + comandos de interface, mouse e teclado.
    Sistema,
}

impl ModoGravacao {
    pub fn chave(self) -> &'static str {
        match self {
            Self::Ecossistema => "ecossistema",
            Self::Sistema => "sistema",
        }
    }

    pub fn rotulo(self) -> &'static str {
        match self {
            Self::Ecossistema => "ecossistema",
            Self::Sistema => "ecossistema + sistema",
        }
    }
}

pub struct Gravacao {
    pub modo: ModoGravacao,
    pub pasta: PathBuf,
    inicio: Instant,
    inicio_unix_ms: u128,
    tick_inicial: u64,
    eco: BufWriter<File>,
    sis: Option<BufWriter<File>>,
    marcas: BufWriter<File>,
    telemetria: BufWriter<File>,
    pub eventos_eco: usize,
    pub eventos_sis: usize,
    pub marcas_feitas: usize,
    amostras: usize,
    mapas: usize,
    falhas: usize,
    linha_do_tempo: Vec<String>,
    manifesto: Value,
}

impl Gravacao {
    /// Cria a pasta da gravação e grava o manifesto e o mapa inicial.
    pub fn iniciar(
        base: &Path,
        modo: ModoGravacao,
        tick: u64,
        manifesto: Value,
        mapa: &MapDefinition,
    ) -> io::Result<Self> {
        let inicio_unix_ms = agora_unix_ms();
        let nome = format!("gravacao-{}-{}", carimbo_utc(inicio_unix_ms), modo.chave());
        let pasta = base.join(nome);
        fs::create_dir_all(pasta.join("mapas"))?;
        let abrir = |nome: &str| -> io::Result<BufWriter<File>> {
            Ok(BufWriter::new(File::create(pasta.join(nome))?))
        };
        let mut gravacao = Self {
            modo,
            eco: abrir("eventos_eco.jsonl")?,
            sis: match modo {
                ModoGravacao::Sistema => Some(abrir("eventos_sis.jsonl")?),
                ModoGravacao::Ecossistema => None,
            },
            marcas: abrir("marcas.jsonl")?,
            telemetria: abrir("telemetria.jsonl")?,
            pasta,
            inicio: Instant::now(),
            inicio_unix_ms,
            tick_inicial: tick,
            eventos_eco: 0,
            eventos_sis: 0,
            marcas_feitas: 0,
            amostras: 0,
            mapas: 0,
            falhas: 0,
            linha_do_tempo: Vec::new(),
            manifesto,
        };
        gravacao.escrever_manifesto(None)?;
        gravacao.mapa(tick, mapa, "inicio");
        gravacao.eco(
            tick,
            "gravacao_iniciada",
            json!({ "resumo": format!("modo {}", modo.rotulo()) }),
        );
        Ok(gravacao)
    }

    pub fn segundos(&self) -> f64 {
        self.inicio.elapsed().as_secs_f64()
    }

    fn linha(&self, tick: u64, tipo: &str, dados: Value) -> String {
        let registro = json!({
            "tick": tick,
            "t_s": (self.segundos() * 1000.0).round() / 1000.0,
            "tipo": tipo,
            "dados": dados,
        });
        registro.to_string()
    }

    fn anotar(&mut self, texto: String) {
        if self.linha_do_tempo.len() < LINHA_DO_TEMPO_MAX {
            self.linha_do_tempo.push(texto);
        } else if self.linha_do_tempo.len() == LINHA_DO_TEMPO_MAX {
            self.linha_do_tempo
                .push("… linha do tempo cortada; o restante está em eventos_eco.jsonl".to_owned());
        }
    }

    /// Evento ecológico: algo que muda o mundo. Gravado e descarregado na hora.
    pub fn eco(&mut self, tick: u64, tipo: &str, dados: Value) {
        let resumo = resumo_de(&dados);
        let linha = self.linha(tick, tipo, dados);
        let ok = writeln!(self.eco, "{linha}").and_then(|_| self.eco.flush());
        if ok.is_err() {
            self.falhas += 1;
        }
        self.eventos_eco += 1;
        let t = self.segundos();
        self.anotar(format!("tick {tick} · +{t:.1} s · **{tipo}** — {resumo}"));
    }

    /// Evento de sistema (interface, mouse, teclado). Ignorado no modo ecossistema.
    pub fn sis(&mut self, tick: u64, tipo: &str, dados: Value) {
        if self.sis.is_none() {
            return;
        }
        let linha = self.linha(tick, tipo, dados);
        if let Some(sis) = self.sis.as_mut() {
            if writeln!(sis, "{linha}").is_err() {
                self.falhas += 1;
            }
        }
        self.eventos_sis += 1;
    }

    /// Descarrega o registro de sistema (chamado a cada lote vindo da interface).
    pub fn descarregar_sis(&mut self) {
        if let Some(sis) = self.sis.as_mut() {
            let _ = sis.flush();
        }
    }

    /// Momento marcado pelo usuário ("aqui deu ruim").
    pub fn marca(&mut self, tick: u64, texto: &str) {
        let texto = texto.trim();
        let linha = self.linha(tick, "marca", json!({ "texto": texto }));
        let ok = writeln!(self.marcas, "{linha}").and_then(|_| self.marcas.flush());
        if ok.is_err() {
            self.falhas += 1;
        }
        self.marcas_feitas += 1;
        let t = self.segundos();
        let rotulo = if texto.is_empty() {
            "(sem texto)"
        } else {
            texto
        };
        self.anotar(format!(
            "tick {tick} · +{t:.1} s · 🚩 **marca {}** — {rotulo}",
            self.marcas_feitas
        ));
    }

    pub fn amostra<T: Serialize>(&mut self, amostra: &T) {
        match serde_json::to_string(amostra) {
            Ok(linha) => {
                // Cada amostra vai direto para o disco: se o app fechar ou o
                // computador desligar, tudo até a última amostra fica salvo.
                if writeln!(self.telemetria, "{linha}")
                    .and_then(|_| self.telemetria.flush())
                    .is_err()
                {
                    self.falhas += 1;
                }
                self.amostras += 1;
            }
            Err(_) => self.falhas += 1,
        }
    }

    /// Grava o mapa como texto: `#` parede, `*` sol, `%` parede ao sol, `.` livre.
    pub fn mapa(&mut self, tick: u64, mapa: &MapDefinition, rotulo: &str) {
        let mut grade = vec![b'.'; MAP_GRID_WIDTH * MAP_GRID_HEIGHT];
        for (x, y) in mapa.sunlight_cells() {
            grade[y * MAP_GRID_WIDTH + x] = b'*';
        }
        for (x, y) in mapa.wall_cells() {
            let celula = &mut grade[y * MAP_GRID_WIDTH + x];
            *celula = if *celula == b'*' { b'%' } else { b'#' };
        }
        let mut texto = String::with_capacity((MAP_GRID_WIDTH + 1) * MAP_GRID_HEIGHT + 128);
        texto.push_str(&format!(
            "# mapa {MAP_GRID_WIDTH}x{MAP_GRID_HEIGHT} · tick {tick} · {rotulo}\n# '#' parede · '*' sol · '%' parede ao sol · '.' livre\n"
        ));
        for linha in grade.chunks(MAP_GRID_WIDTH) {
            texto.push_str(std::str::from_utf8(linha).unwrap_or(""));
            texto.push('\n');
        }
        self.mapas += 1;
        let nome = format!("{:03}-tick-{tick}-{rotulo}.txt", self.mapas);
        if fs::write(self.pasta.join("mapas").join(nome), texto).is_err() {
            self.falhas += 1;
        }
    }

    fn escrever_manifesto(&self, fim: Option<Value>) -> io::Result<()> {
        let mut manifesto = self.manifesto.clone();
        if let Value::Object(campos) = &mut manifesto {
            campos.insert("formato".into(), json!("ngu-gravacao/1"));
            campos.insert("modo".into(), json!(self.modo.chave()));
            campos.insert("inicio_utc".into(), json!(carimbo_utc(self.inicio_unix_ms)));
            campos.insert("inicio_unix_ms".into(), json!(self.inicio_unix_ms as u64));
            campos.insert("tick_inicial".into(), json!(self.tick_inicial));
            if let Some(fim) = fim {
                campos.insert("fim".into(), fim);
            }
        }
        let texto = serde_json::to_string_pretty(&manifesto).unwrap_or_else(|_| "{}".into());
        fs::write(self.pasta.join("manifesto.json"), texto)
    }

    /// Fecha a gravação: evento final, manifesto completo e RESUMO.md.
    pub fn encerrar(mut self, tick: u64, estado_final: Value) -> io::Result<PathBuf> {
        self.eco(
            tick,
            "gravacao_encerrada",
            json!({ "resumo": format!("tick final {tick}") }),
        );
        let _ = self.telemetria.flush();
        self.descarregar_sis();
        let duracao = self.segundos();
        let fim = json!({
            "tick_final": tick,
            "duracao_s": (duracao * 10.0).round() / 10.0,
            "eventos_eco": self.eventos_eco,
            "eventos_sis": self.eventos_sis,
            "marcas": self.marcas_feitas,
            "amostras": self.amostras,
            "mapas": self.mapas,
            "falhas_de_escrita": self.falhas,
            "estado_final": estado_final,
        });
        self.escrever_manifesto(Some(fim))?;
        fs::write(self.pasta.join("RESUMO.md"), self.resumo_md(tick, duracao))?;
        Ok(self.pasta.clone())
    }

    fn resumo_md(&self, tick_final: u64, duracao: f64) -> String {
        let campo = |chave: &str| -> String {
            self.manifesto
                .get(chave)
                .map(|valor| match valor {
                    Value::String(texto) => texto.clone(),
                    outro => outro.to_string(),
                })
                .unwrap_or_else(|| "—".into())
        };
        let nome = self
            .pasta
            .file_name()
            .map(|nome| nome.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut md = String::new();
        md.push_str(&format!("# {nome}\n\n"));
        md.push_str("Leia este arquivo primeiro. Os brutos ficam ao lado.\n\n");
        md.push_str(&format!("- **Modo:** {}\n", self.modo.rotulo()));
        md.push_str(&format!(
            "- **Início:** {} (UTC)\n",
            carimbo_utc(self.inicio_unix_ms)
        ));
        md.push_str(&format!("- **Duração:** {duracao:.1} s\n"));
        md.push_str(&format!(
            "- **Ticks:** {} → {} ({} ticks)\n",
            self.tick_inicial,
            tick_final,
            tick_final.saturating_sub(self.tick_inicial)
        ));
        md.push_str(&format!(
            "- **Versão da interface:** {}\n",
            campo("versao_interface")
        ));
        md.push_str(&format!(
            "- **Seed:** {} · **run:** {}\n",
            campo("seed"),
            campo("run_id")
        ));
        md.push_str(&format!(
            "- **Contagens:** {} eventos ecológicos · {} eventos de sistema · {} marcas · {} amostras · {} mapas",
            self.eventos_eco, self.eventos_sis, self.marcas_feitas, self.amostras, self.mapas
        ));
        if self.falhas > 0 {
            md.push_str(&format!(" · ⚠️ {} falhas de escrita", self.falhas));
        }
        md.push_str("\n\n## Linha do tempo (ecossistema e marcas)\n\n");
        for item in &self.linha_do_tempo {
            md.push_str(&format!("- {item}\n"));
        }
        md.push_str(
            "\n## Arquivos\n\n\
             | Arquivo | Conteúdo |\n|---|---|\n\
             | `manifesto.json` | versão, seed, condições no início e no fim |\n\
             | `eventos_eco.jsonl` | mudanças no mundo, com antes/depois |\n\
             | `eventos_sis.jsonl` | comandos da interface, mouse e teclado (modo sistema) |\n\
             | `marcas.jsonl` | momentos marcados com F8 |\n\
             | `telemetria.jsonl` | amostras demográficas, uma por linha |\n\
             | `mapas/` | o mapa em texto no início e a cada aplicação |\n",
        );
        md
    }
}

// ---------------------------------------------------------------------------
// Diferenças (antes → depois)
// ---------------------------------------------------------------------------

/// Compara dois objetos JSON campo a campo e devolve só o que mudou:
/// `{ campo: { antes, depois, variacao_pct? } }`.
pub fn diferenca_de_parametros(antes: &Value, depois: &Value) -> Map<String, Value> {
    let mut mudancas = Map::new();
    let (Value::Object(a), Value::Object(d)) = (antes, depois) else {
        return mudancas;
    };
    for (chave, valor_depois) in d {
        let valor_antes = a.get(chave).cloned().unwrap_or(Value::Null);
        if &valor_antes == valor_depois {
            continue;
        }
        let mut item = Map::new();
        item.insert("antes".into(), valor_antes.clone());
        item.insert("depois".into(), valor_depois.clone());
        if let (Some(x), Some(y)) = (valor_antes.as_f64(), valor_depois.as_f64()) {
            if x != 0.0 {
                let pct = (y - x) / x.abs() * 100.0;
                item.insert("variacao_pct".into(), json!((pct * 10.0).round() / 10.0));
            }
        }
        mudancas.insert(chave.clone(), Value::Object(item));
    }
    mudancas
}

/// "emission 0.12 → 0.08 (-33.3%); flow_preset Clássico → Oásis"
pub fn resumo_de_parametros(mudancas: &Map<String, Value>) -> String {
    let curto = |valor: Option<&Value>| match valor {
        Some(Value::String(texto)) => texto.clone(),
        Some(Value::Number(numero)) => numero
            .as_f64()
            .map(|x| format!("{}", (x * 10_000.0).round() / 10_000.0))
            .unwrap_or_else(|| numero.to_string()),
        Some(outro) => outro.to_string(),
        None => "—".into(),
    };
    mudancas
        .iter()
        .map(|(campo, item)| {
            let pct = item
                .get("variacao_pct")
                .and_then(Value::as_f64)
                .map(|p| format!(" ({p:+.1}%)"))
                .unwrap_or_default();
            format!(
                "{campo} {} → {}{pct}",
                curto(item.get("antes")),
                curto(item.get("depois"))
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// O que um "Aplicar mapa" mudou: paredes e sol, em células e em % do mapa.
pub fn diferenca_de_mapa(antes: &MapDefinition, depois: &MapDefinition) -> Value {
    let total = (MAP_GRID_WIDTH * MAP_GRID_HEIGHT) as f64;
    let paredes_a: HashSet<(usize, usize)> = antes.wall_cells().collect();
    let paredes_d: HashSet<(usize, usize)> = depois.wall_cells().collect();
    let sol_a: HashSet<(usize, usize)> = antes.sunlight_cells().collect();
    let sol_d: HashSet<(usize, usize)> = depois.sunlight_cells().collect();
    let pct = |n: usize| (n as f64 / total * 1000.0).round() / 10.0;
    let paredes_mais = paredes_d.difference(&paredes_a).count();
    let paredes_menos = paredes_a.difference(&paredes_d).count();
    let sol_mais = sol_d.difference(&sol_a).count();
    let sol_menos = sol_a.difference(&sol_d).count();
    let resumo = format!(
        "paredes +{paredes_mais}/−{paredes_menos} células ({}% → {}% do mapa); sol +{sol_mais}/−{sol_menos} ({}% → {}%)",
        pct(paredes_a.len()),
        pct(paredes_d.len()),
        pct(sol_a.len()),
        pct(sol_d.len()),
    );
    json!({
        "resumo": resumo,
        "paredes": {
            "adicionadas": paredes_mais,
            "removidas": paredes_menos,
            "antes": paredes_a.len(),
            "depois": paredes_d.len(),
            "pct_antes": pct(paredes_a.len()),
            "pct_depois": pct(paredes_d.len()),
        },
        "sol": {
            "adicionado": sol_mais,
            "removido": sol_menos,
            "antes": sol_a.len(),
            "depois": sol_d.len(),
            "pct_antes": pct(sol_a.len()),
            "pct_depois": pct(sol_d.len()),
        },
        "grade": [MAP_GRID_WIDTH, MAP_GRID_HEIGHT],
    })
}

// ---------------------------------------------------------------------------
// Utilidades
// ---------------------------------------------------------------------------

fn resumo_de(dados: &Value) -> String {
    if let Some(texto) = dados.get("resumo").and_then(Value::as_str) {
        return texto.to_owned();
    }
    let mut texto = dados.to_string();
    if texto.len() > 200 {
        let mut corte = 200;
        while !texto.is_char_boundary(corte) {
            corte -= 1;
        }
        texto.truncate(corte);
        texto.push('…');
    }
    texto
}

pub fn agora_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duracao| duracao.as_millis())
        .unwrap_or(0)
}

/// `2026-11-21T23-40-05Z` (UTC; sem `:` para servir de nome de pasta no Windows).
pub fn carimbo_utc(unix_ms: u128) -> String {
    let segundos = (unix_ms / 1000) as i64;
    let dias = segundos.div_euclid(86_400);
    let resto = segundos.rem_euclid(86_400);
    let (ano, mes, dia) = data_civil(dias);
    format!(
        "{ano:04}-{mes:02}-{dia:02}T{:02}-{:02}-{:02}Z",
        resto / 3600,
        (resto % 3600) / 60,
        resto % 60
    )
}

/// Dias desde 1970-01-01 → (ano, mês, dia). Algoritmo de Howard Hinnant.
fn data_civil(dias: i64) -> (i64, u32, u32) {
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mes = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let ano = yoe + era * 400 + i64::from(mes <= 2);
    (ano, mes, dia)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn carimbo_conhecido() {
        // 2026-11-21 23:40:05 UTC
        assert_eq!(carimbo_utc(1_795_304_405_000), "2026-11-21T23-40-05Z");
        assert_eq!(carimbo_utc(0), "1970-01-01T00-00-00Z");
    }

    #[test]
    fn diferenca_so_do_que_mudou() {
        let antes = json!({ "emission": 0.12, "flow_preset": "Clássico", "lifetime": 600 });
        let depois = json!({ "emission": 0.06, "flow_preset": "Oásis", "lifetime": 600 });
        let mudancas = diferenca_de_parametros(&antes, &depois);
        assert_eq!(mudancas.len(), 2);
        assert_eq!(mudancas["emission"]["variacao_pct"], json!(-50.0));
        let resumo = resumo_de_parametros(&mudancas);
        assert!(resumo.contains("flow_preset Clássico → Oásis"));
    }
}
