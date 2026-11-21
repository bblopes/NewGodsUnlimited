# Versões de teste

Cada build novo chega aqui primeiro, já com o número definitivo, dentro da
pasta da sua **versão maior**. Cada versão maior tem um título que diz qual é a
grande mudança dela:

| Pasta | Versão maior | O que ela é |
| --- | --- | --- |
| `V1 - Interface original/` | V1.x | o programa com a interface original (Macroquad) |
| `V2 - Revamp da interface/` | V2.x | **revamp da interface de usuário**: interface nova (Tauri) sobre o mesmo motor |
| `V3 - Otimização/` | V3.x | **otimização**: motor mais rápido, com o mesmo resultado de simulação |

```text
2 - Versões de teste/
├── V1 - Interface original/
│   └── V1.1.0/
│       ├── New Gods Unlimited V1.1.0.exe
│       └── ENTREGA.md
├── V2 - Revamp da interface/
│   ├── V2.0.0/
│   │   ├── New Gods Unlimited V2.0.0.exe
│   │   └── ENTREGA.md
│   └── V2.0.1/
│       ├── New Gods Unlimited V2.0.1.exe
│       └── ENTREGA.md
└── _histórico/
```

Cada entrega tem a sua pasta, com o `.exe` e um `ENTREGA.md` curto: o que foi
pedido, o que foi entregue, como testar e o que ficou de fora. O número sobe a
cada entrega (`V2.0.0`, `V2.0.1`, `V2.1.0`…).
Quando você aprova uma versão, o mesmo arquivo passa para `1 - Versão oficial/`
e a oficial anterior vai para `_histórico/`. Versões reprovadas também vão para
`_histórico/`.

## Em validação

| Arquivo | O que validar | Data |
| --- | --- | --- |
| `V3 - Otimização/V3.0.3/` | Medidor de ticks/s corrigido (mostra a velocidade real) e paralelismo opcional (padrão 1 thread) | 21/11/2026 |
| `V3 - Otimização/V3.0.2/` | Índices de vizinhos leves e fitoplâncton mais barato (sessão 2× mais rápida) | 21/11/2026 |
| `V3 - Otimização/V3.0.1/` | Assinatura PG em cache e distância às paredes (células 4× mais rápidas); fitoplâncton e ticks/s na Demografia | 21/11/2026 |
| `V3 - Otimização/V3.0.0/` | PF pré-calculado (sessão 7× mais rápida) e clique da arena alinhado | 21/11/2026 |
| `V2 - Revamp da interface/V2.0.3/` | Demografia corrigida (sem dados na V2.0.0–V2.0.2) e coleta configurável | 21/11/2026 |
| `V2 - Revamp da interface/V2.0.2/` | aba Biologia e código genético explicado (canais de P1/P2/P3) | 21/11/2026 |
| `V2 - Revamp da interface/V2.0.1/` | salvar indivíduo no banco e usá-lo como configuração neural | 21/11/2026 |
| `V2 - Revamp da interface/V2.0.0/` | primeira versão da interface nova | 21/11/2026 |
| `V1 - Interface original/V1.1.0/` | visão genérica e quimiorrecepção PF | 21/11/2026 |

## `_histórico/`

Entregas antigas, só para consulta. Cada pasta começa com a data:

- `2026-10-16 …` e `2026-10-17 …` — builds da época em que o programa se chamava "Ecossistema Neural";
- `2026-10-24 visao-pf-experimental` — a primeira build experimental de visão + PF, substituída pela `V1.1.0`.
