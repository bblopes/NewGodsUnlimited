# Versões e distribuição

## Nome das versões

Padrão: **`New Gods Unlimited V<maior>.<média>.<correção>[-teste.<n>].exe`**

| Mudança | Exemplo | Quando usar |
| --- | --- | --- |
| maior | `V1.4.2` → `V2.0.0` | mudança grande de produto (interface nova, formato de save incompatível) |
| média | `V1.0.0` → `V1.1.0` | entrega com novidades |
| correção | `V1.1.0` → `V1.1.1` | só correções, sem novidade |
| teste | `V1.1.0-teste.1`, `-teste.2`… | builds para validar a próxima versão; o número sobe a cada build |

Quando um teste é aprovado, ele é publicado sem o sufixo (`V1.1.0`). A tag no
Git tem o mesmo número com `v` minúsculo: `v1.1.0`, `v1.1.0-teste.2`.

Histórico: o que estava publicado em 21/11/2026 virou `V1.0.0`. As entregas de
outubro de 2026 ("Ecossistema Neural") estão em `2 - Versões de teste/_histórico/`.

## Onde cada versão fica

| Onde | O quê |
| --- | --- |
| `1 - Versão oficial/` | só a última oficial, mais presets e runs salvas |
| `2 - Versões de teste/` | testes em validação |
| `2 - Versões de teste/_histórico/` | versões substituídas, para consulta |
| GitHub → Releases | todas as versões; testes marcados como *pre-release* |

## Como uma versão é compilada

O padrão é o **GitHub Actions** (`.github/workflows/compilar-windows.yml`):

1. Criar e enviar a tag da versão, por exemplo `v1.1.0-teste.1`.
2. O GitHub compila no Windows (MSVC, `crt-static`) a partir de `5 - Código-fonte/`.
3. O `.exe` sai já com o nome final e é publicado na Release da tag.
4. O `.exe` é baixado e colocado na pasta certa do computador.

Também dá para rodar o workflow manualmente pela aba **Actions**, informando a
versão.

**Reserva local** (sem internet): `5 - Código-fonte/scripts/COMPILAR_VERSAO.bat <versão>`.
Precisa de Rust e do Visual Studio Build Tools 2022 instalados. O script coloca
o `.exe` na pasta oficial ou na de teste, conforme a versão.

## Entrega portátil

O `.exe` é autônomo: basta copiá-lo para uma pasta gravável e executá-lo. Não
exige Rust, Cargo nem instalação, e só usa bibliotecas do próprio Windows.

Na primeira vez que algo é salvo, o programa cria ao lado dele:

- `runs_salvas/` — sessões e telemetria exportadas;
- `presets_sessao.ecopreset` e `presets_mapa.ecomap` — presets do usuário.

Mover a pasta inteira preserva os dados. Para compartilhar um ambiente limpo,
entregue só o `.exe`.
