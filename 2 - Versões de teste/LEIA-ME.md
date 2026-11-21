# Versões de teste

Cada build novo chega aqui primeiro, já com o número definitivo, dentro da
pasta da sua **versão maior**. Cada versão maior tem um título que diz qual é a
grande mudança dela:

| Pasta | Versão maior | O que ela é |
| --- | --- | --- |
| `V1 - Interface original/` | V1.x | o programa com a interface original (Macroquad) |
| `V2 - Revamp da interface/` | V2.x | **revamp da interface de usuário**: interface nova (Tauri) sobre o mesmo motor |

```text
2 - Versões de teste/
├── V1 - Interface original/
│   └── New Gods Unlimited V1.1.0.exe
├── V2 - Revamp da interface/
│   └── New Gods Unlimited V2.0.0.exe
└── _histórico/
```

Dentro de cada pasta, o número sobe a cada build (`V2.0.0`, `V2.0.1`, `V2.1.0`…).
Quando você aprova uma versão, o mesmo arquivo passa para `1 - Versão oficial/`
e a oficial anterior vai para `_histórico/`. Versões reprovadas também vão para
`_histórico/`.

## Em validação

| Arquivo | O que validar | Data |
| --- | --- | --- |
| `V2 - Revamp da interface/New Gods Unlimited V2.0.0.exe` | primeira versão da interface nova; salvar/carregar e Laboratório ainda não portados | 21/11/2026 |
| `V1 - Interface original/New Gods Unlimited V1.1.0.exe` | visão genérica e quimiorrecepção PF (antiga build experimental "PF Vision") | 21/11/2026 |

## `_histórico/`

Entregas antigas, só para consulta. Cada pasta começa com a data:

- `2026-10-16 …` e `2026-10-17 …` — builds da época em que o programa se chamava "Ecossistema Neural";
- `2026-10-24 visao-pf-experimental` — a primeira build experimental de visão + PF, substituída pela `V1.1.0`.
