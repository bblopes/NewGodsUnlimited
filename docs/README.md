# Documentação do New Gods Unlimited

Esta pasta reúne as decisões de produto e de arquitetura. O código executável
permanece em `src/`; estes documentos explicam as regras que ele implementa ou
as propostas que ainda aguardam aprovação.

| Documento | Assunto |
|---|---|
| [Documentação preliminar](DOCUMENTACAO_PRELIMINAR.md) | visão do ecossistema e cérebro evolutivo |
| [Estrutura de arquivos](ESTRUTURA_DE_ARQUIVOS_PROPOSTA.md) | responsabilidades dos módulos Rust |
| [Estrutura do repositório](ESTRUTURA_DO_REPOSITORIO.md) | finalidade de cada pasta e regra de organização |
| [Guia de leitura do tick](GUIA_DE_LEITURA_DO_TICK.md) | ordem de cálculo, chamadas entre módulos e como estudar `World::tick()` |
| [Medição de desempenho](MEDICAO_DE_DESEMPENHO.md) | benchmark reproduzível, cenários e primeira linha de base |
| [Energia e sensores hereditários](RELATORIO_ENERGIA_E_SENSORES_HEREDITARIOS.md) | inventário dos custos atuais e proposta de alcance sensorial evolutivo |
| [Telemetria demográfica](PROPOSTA_TELEMETRIA_DEMOGRAFICA.md) | proposta de referência de reprodução e marcos de mudanças nos gráficos |
| [Sessão reproduzível](ESPECIFICACAO_SESSAO_REPRODUZIVEL.md) | telemetria, salvar e restaurar uma execução |
| [Laboratório](DOCUMENTACAO_LABORATORIO.md) | campanhas, cenários e execuções em lote |
| [Análise genética](DOCUMENTACAO_ANALISE_GENETICA.md) | cor, similaridade, linhagens e KPIs |
| [Osmose e engolfamento](PROPOSTA_INTERACAO_CELULAR_E_COMBATE.md) | proposta de interação celular futura |
| [Mutação adaptativa](PROPOSTA_MUTACAO_ADAPTATIVA.md) | proposta de genes que regulam mutação |
| [Feromônios e comunicação](PROPOSTA_FEROMONIOS_E_COMUNICACAO.md) | proposta de linguagem química, assinatura genética e comunicação emergente |
| [Doação de energia e Val 2.0](PROPOSTA_DOACAO_ENERGIA_E_VAL_2.md) | proposta de transferência física de energia entre células em contato |
| [Longevidade hereditária](LONGEVIDADE_HEREDITARIA.md) | alelo fisiológico de idade máxima e referência de 100 mil ticks |
| [Distribuição portátil](DISTRIBUICAO_PORTATIL.md) | entrega de um único executável para Windows |

## Convenção de estado

- **Implementado/publicado**: presente no executável principal de produção.
- **Implementado no código**: presente no fonte, mas ainda não publicado.
- **Proposta**: documentado para validação; não altera a simulação.
