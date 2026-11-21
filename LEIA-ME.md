# New Gods Unlimited

Simulação 2D de um ecossistema celular evolutivo. Células com uma pequena rede
neural vivem numa câmara com correntes, luz e fitoplâncton. Nenhum
comportamento é programado: a seleção natural decide o que funciona.

## Por onde começar

| Quero… | Abra |
| --- | --- |
| **Rodar a versão oficial** | [`1 - Versão oficial/`](1%20-%20Versão%20oficial/) → `New Gods Unlimited V1.0.0.exe` |
| Experimentar a interface nova | [`2 - Versões de teste/`](2%20-%20Versões%20de%20teste/) → `New Gods Unlimited V2.0.0.exe` |
| Testar uma versão em validação | [`2 - Versões de teste/`](2%20-%20Versões%20de%20teste/) |
| Mostrar o projeto para alguém | [`3 - Documentos/Apresentação/`](3%20-%20Documentos/Apresentação/) |
| Ver o que vem a seguir | [`3 - Documentos/Backlog/`](3%20-%20Documentos/Backlog/) |
| Entender as regras da simulação | [`3 - Documentos/Documentação/`](3%20-%20Documentos/Documentação/) |
| Abrir runs, campanhas e benchmarks salvos | [`4 - Dados/`](4%20-%20Dados/) |
| Mexer no código | [`5 - Código-fonte/`](5%20-%20Código-fonte/) |

## As pastas

```text
ECOSSYSTEM/
├── LEIA-ME.md               este arquivo
├── 1 - Versão oficial/      a última versão aprovada, pronta para abrir
├── 2 - Versões de teste/    builds para validar antes de virarem oficiais
│   └── _histórico/          entregas antigas, só para consulta
├── 3 - Documentos/
│   ├── Apresentação/        o que mostrar para outras pessoas
│   ├── Documentação/        regras que já estão no programa
│   ├── Backlog/             o que vem a seguir e o roadmap
│   ├── Propostas/           ideias especificadas, ainda não feitas
│   └── Relatórios/          medições de desempenho e energia
├── 4 - Dados/               runs, campanhas, presets e benchmarks
└── 5 - Código-fonte/        todo o projeto Rust (Cargo, src, scripts…)
```

## Versões

Toda versão tem só número: **`New Gods Unlimited V<maior>.<média>.<correção>`**.

- **média** sobe quando entra novidade: `V1.0.0` → `V1.1.0`;
- **correção** sobe a cada ajuste ou novo build da mesma novidade: `V1.1.0` → `V1.1.1`;
- **maior** sobe numa mudança grande de produto, como a interface nova: `V2.0.0`.

Cada build novo vai primeiro para `2 - Versões de teste/`. Quando você aprova,
o mesmo arquivo, com o mesmo nome, passa para `1 - Versão oficial/`.

A versão fica no `Cargo.toml`; cada versão nova que entra na `main` é compilada
no GitHub (Actions, Windows) e publicada em
[Releases](https://github.com/bblopes/NewGodsUnlimited/releases). A regra completa
está em [`3 - Documentos/Documentação/VERSOES_E_DISTRIBUICAO.md`](3%20-%20Documentos/Documentação/VERSOES_E_DISTRIBUICAO.md).

## O que vai para o GitHub

Código, documentos, scripts e presets de referência. Os `.exe`, as runs salvas,
as campanhas e os caches de compilação ficam só no computador (e os `.exe`
também nas Releases do GitHub).
