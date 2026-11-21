//! Banco de indivíduos salvos.
//!
//! Fica num JSON na pasta de dados do aplicativo no Windows
//! (`%APPDATA%\com.bblopes.newgodsunlimited\banco_de_individuos.json`), fora
//! das pastas de versão. Assim qualquer versão nova da interface encontra os
//! indivíduos salvos pelas anteriores. O genoma é guardado em hexadecimal junto
//! com o tamanho do formato, para que versões futuras possam migrá-lo.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use new_gods_unlimited::simulation::genome::{Genome, GENOME_BYTES};

pub const FORMATO_DO_BANCO: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndividuoSalvo {
    pub id: String,
    pub nome: String,
    /// Segundos desde 1970 (UTC).
    pub criado_em: u64,
    pub origem: String,
    pub geracao: u32,
    pub cor: String,
    /// Tamanho em bytes do formato de genoma em que foi salvo.
    pub bytes_do_genoma: usize,
    pub genoma: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DadosDoBanco {
    pub formato: u32,
    pub individuos: Vec<IndividuoSalvo>,
}

pub struct Banco {
    caminho: Option<PathBuf>,
    pub dados: DadosDoBanco,
    pub erro: Option<String>,
}

impl Banco {
    pub fn abrir(caminho: Option<PathBuf>) -> Self {
        let mut banco = Self {
            caminho,
            dados: DadosDoBanco {
                formato: FORMATO_DO_BANCO,
                individuos: Vec::new(),
            },
            erro: None,
        };
        if let Some(caminho) = &banco.caminho {
            match fs::read_to_string(caminho) {
                Ok(texto) => match serde_json::from_str::<DadosDoBanco>(&texto) {
                    Ok(dados) => banco.dados = dados,
                    Err(erro) => {
                        // Nunca sobrescreve um banco que não conseguiu ler.
                        let copia = caminho.with_extension("json.ilegivel");
                        let _ = fs::copy(caminho, &copia);
                        banco.erro = Some(format!(
                            "não foi possível ler o banco ({erro}); uma cópia foi guardada em {}",
                            copia.display()
                        ));
                    }
                },
                Err(erro) if erro.kind() == io::ErrorKind::NotFound => {}
                Err(erro) => banco.erro = Some(format!("não foi possível abrir o banco: {erro}")),
            }
        }
        banco
    }

    pub fn caminho_texto(&self) -> String {
        self.caminho
            .as_ref()
            .map(|caminho| caminho.display().to_string())
            .unwrap_or_else(|| "(sem pasta de dados)".to_owned())
    }

    fn gravar(&self) -> io::Result<()> {
        let Some(caminho) = &self.caminho else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "sem pasta de dados"));
        };
        if let Some(pasta) = caminho.parent() {
            fs::create_dir_all(pasta)?;
        }
        let texto = serde_json::to_string_pretty(&self.dados)
            .map_err(|erro| io::Error::new(io::ErrorKind::InvalidData, erro))?;
        // Grava num temporário e troca, para nunca deixar o banco pela metade.
        let temporario = caminho.with_extension("json.tmp");
        fs::write(&temporario, texto)?;
        fs::rename(&temporario, caminho)
    }

    pub fn adicionar(
        &mut self,
        nome: String,
        origem: String,
        geracao: u32,
        genome: &Genome,
    ) -> io::Result<IndividuoSalvo> {
        let agora = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duracao| duracao.as_millis() as u64)
            .unwrap_or(0);
        let mut id = format!("i{agora}");
        while self.dados.individuos.iter().any(|individuo| individuo.id == id) {
            id.push('x');
        }
        let individuo = IndividuoSalvo {
            id,
            nome,
            criado_em: agora / 1000,
            origem,
            geracao,
            cor: genome.genetic_hex(),
            bytes_do_genoma: GENOME_BYTES,
            genoma: genome.bytes().iter().map(|byte| format!("{byte:02x}")).collect(),
        };
        self.dados.formato = FORMATO_DO_BANCO;
        self.dados.individuos.push(individuo.clone());
        if let Err(erro) = self.gravar() {
            self.dados.individuos.pop();
            return Err(erro);
        }
        Ok(individuo)
    }

    pub fn remover(&mut self, id: &str) -> io::Result<bool> {
        let antes = self.dados.individuos.len();
        let removidos: Vec<IndividuoSalvo> = self
            .dados
            .individuos
            .iter()
            .filter(|individuo| individuo.id == id)
            .cloned()
            .collect();
        self.dados.individuos.retain(|individuo| individuo.id != id);
        if self.dados.individuos.len() == antes {
            return Ok(false);
        }
        if let Err(erro) = self.gravar() {
            self.dados.individuos.extend(removidos);
            return Err(erro);
        }
        Ok(true)
    }

    pub fn buscar(&self, id: &str) -> Option<&IndividuoSalvo> {
        self.dados.individuos.iter().find(|individuo| individuo.id == id)
    }
}

/// Reconstrói o genoma salvo. Devolve `None` se o formato não for o atual
/// (uma versão futura deve trazer a migração correspondente).
pub fn genoma_de(individuo: &IndividuoSalvo) -> Option<Genome> {
    if individuo.bytes_do_genoma != GENOME_BYTES || individuo.genoma.len() != GENOME_BYTES * 2 {
        return None;
    }
    let mut bytes = [0u8; GENOME_BYTES];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&individuo.genoma[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(Genome::from_bytes(bytes))
}
