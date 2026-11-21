# Versões e distribuição

## Nome das versões

Padrão: **`New Gods Unlimited V<maior>.<média>.<correção>.exe`**, só números.

| Parte | Exemplo | Quando sobe |
| --- | --- | --- |
| maior | `V1.4.2` → `V2.0.0` | mudança grande de produto (interface nova, formato de save incompatível) |
| média | `V1.0.0` → `V1.1.0` | entra uma novidade |
| correção | `V1.1.0` → `V1.1.1` | ajuste, correção ou novo build da mesma novidade |

Todo build recebe um número novo e nunca é renomeado. O que diz se ele é
oficial ou está em validação é a pasta (e, no GitHub, a marca *pre-release*).
A tag no Git tem o mesmo número com `v` minúsculo: `v1.1.0`.

Histórico: o que estava publicado em 21/11/2026 virou `V1.0.0`. As entregas de
outubro de 2026 ("Ecossistema Neural") estão em `2 - Versões de teste/_histórico/`.

## Onde cada versão fica

| Onde | O quê |
| --- | --- |
| `2 - Versões de teste/` | todo build novo, até ser aprovado |
| `1 - Versão oficial/` | só a última aprovada, mais presets e runs salvas |
| `2 - Versões de teste/_histórico/` | versões substituídas ou reprovadas |
| GitHub → Releases | todas as versões; as ainda não aprovadas marcadas como *pre-release* |

## Aprovar uma versão

1. Você testa o `.exe` em `2 - Versões de teste/` e diz que aprovou.
2. O arquivo passa para `1 - Versão oficial/`; a oficial anterior vai para `_histórico/`.
3. No GitHub, a Release perde a marca *pre-release* e vira a *latest*.

## Como uma versão é compilada

O padrão é o **GitHub Actions** (`.github/workflows/compilar-windows.yml`):

1. A versão é escrita no campo `version` de `5 - Código-fonte/Cargo.toml`
   (por exemplo `1.1.1`).
2. Quando essa mudança entra na `main`, o GitHub compila no Windows (MSVC,
   `crt-static`), cria a tag `v1.1.1` e publica a Release (*pre-release*) com o
   `.exe` já no nome final. Se a versão já tiver Release, nada é recompilado.
3. O `.exe` é baixado e colocado na pasta certa do computador.

Também dá para rodar o workflow manualmente pela aba **Actions**; nesse caso ele
recompila a versão atual e substitui o `.exe` da Release.

**Reserva local** (sem internet): `5 - Código-fonte/scripts/COMPILAR_VERSAO.bat <versão>`.
Precisa de Rust e do Visual Studio Build Tools 2022 instalados. O script coloca
o `.exe` em `2 - Versões de teste/`.

## Entrega portátil

O `.exe` é autônomo: basta copiá-lo para uma pasta gravável e executá-lo. Não
exige Rust, Cargo nem instalação, e só usa bibliotecas do próprio Windows.

Na primeira vez que algo é salvo, o programa cria ao lado dele:

- `runs_salvas/` — sessões e telemetria exportadas;
- `presets_sessao.ecopreset` e `presets_mapa.ecomap` — presets do usuário.

Mover a pasta inteira preserva os dados. Para compartilhar um ambiente limpo,
entregue só o `.exe`.
