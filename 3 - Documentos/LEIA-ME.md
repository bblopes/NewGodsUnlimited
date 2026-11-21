# Documentos do New Gods Unlimited

Os documentos estão separados pelo uso: o que mostrar, o que já existe no
programa, o que vem a seguir, o que ainda é ideia e o que foi medido.

## Apresentação — para mostrar a outras pessoas

| Documento | Assunto |
| --- | --- |
| [O que é o New Gods Unlimited](Apresentação/O_QUE_E_NEW_GODS_UNLIMITED.md) | visão geral em linguagem simples |
| [Protótipo da nova interface](Apresentação/prototipo-nova-interface.html) | abra no navegador; dados fictícios |

## Documentação — regras que já estão no programa

| Documento | Assunto |
| --- | --- |
| [Visão original do projeto](Documentação/VISAO_ORIGINAL_DO_PROJETO.md) | ideia inicial: ecossistema e cérebro evolutivo |
| [Módulos do código](Documentação/MODULOS_DO_CODIGO.md) | responsabilidade de cada arquivo Rust |
| [Estrutura do repositório](Documentação/ESTRUTURA_DO_REPOSITORIO.md) | o que vai em cada pasta |
| [Guia de leitura do tick](Documentação/GUIA_DE_LEITURA_DO_TICK.md) | ordem de cálculo de `World::tick()` |
| [Fisiologia neural e tamanho](Documentação/FISIOLOGIA_NEURAL_E_TAMANHO.md) | nós neurais hereditários e corpos grandes |
| [Luz solar e fitoplâncton](Documentação/LUZ_SOLAR_E_FITOPLANCTON.md) | máscara solar, reprodução e envelhecimento do alimento |
| [PG, visão e canal de alimento](Documentação/PG_E_CANAL_DE_ALIMENTO.md) | assinatura genética, visão por cone e gradiente PF |
| [Feromônios e comunicação](Documentação/FEROMONIOS_E_COMUNICACAO.md) | PG, P1–P3 e comunicação emergente |
| [Doação de energia e Val 2.0](Documentação/DOACAO_ENERGIA_E_VAL_2.md) | transferência de energia entre células em contato |
| [Longevidade hereditária](Documentação/LONGEVIDADE_HEREDITARIA.md) | alelo de idade máxima |
| [Sessão reproduzível](Documentação/ESPECIFICACAO_SESSAO_REPRODUZIVEL.md) | telemetria, salvar e restaurar uma run |
| [Laboratório](Documentação/DOCUMENTACAO_LABORATORIO.md) | campanhas e execuções em lote |
| [Análise genética](Documentação/DOCUMENTACAO_ANALISE_GENETICA.md) | cor, similaridade, grupos e KPIs |
| [Versões e distribuição](Documentação/VERSOES_E_DISTRIBUICAO.md) | nome das versões, compilação e entrega do `.exe` |

## Backlog — o que vem a seguir

| Documento | Assunto |
| --- | --- |
| [Pendências e roadmap](Backlog/PENDENCIAS_E_ROADMAP.md) | tudo que está pendente, agrupado por tema |
| [Nova interface](Backlog/NOVA_INTERFACE.md) | troca da UI Macroquad por Tauri + React |
| [Backlog de entregas](Backlog/BACKLOG.md) | escopo aprovado por entrega e histórico |

## Propostas — especificadas, ainda não feitas

| Documento | Assunto |
| --- | --- |
| [Mutação adaptativa](Propostas/PROPOSTA_MUTACAO_ADAPTATIVA.md) | genes que regulam a taxa de mutação |
| [Telemetria demográfica](Propostas/PROPOSTA_TELEMETRIA_DEMOGRAFICA.md) | marcos de mudança nos gráficos |
| [Osmose e engolfamento](Propostas/PROPOSTA_INTERACAO_CELULAR_E_COMBATE.md) | interação celular; parte já implementada (fagocitose) |

## Relatórios — medições

| Documento | Assunto |
| --- | --- |
| [Medição de desempenho](Relatórios/MEDICAO_DE_DESEMPENHO.md) | benchmark, cenários e linha de base |
| [Energia e sensores hereditários](Relatórios/RELATORIO_ENERGIA_E_SENSORES_HEREDITARIOS.md) | inventário dos custos de energia |

## Convenção de estado

- **Publicado**: está na versão oficial.
- **Implementado no código**: está no fonte, ainda não numa versão oficial.
- **Proposta**: documentado para validação; não altera a simulação.
