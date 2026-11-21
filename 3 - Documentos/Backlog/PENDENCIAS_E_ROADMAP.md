# Pendências e roadmap

Levantamento de 21/11/2026 de tudo que os documentos marcam como pendente,
conferido contra o código. Agrupado por tema, na ordem de dependência sugerida.

## Ordem sugerida

1. ~~Reorganização das pastas e fluxo de versões~~ (feito em 21/11/2026)
2. **V2 — Revamp da interface**: fechada em 21/11/2026 na V2.0.3; salvar/carregar runs, presets e Laboratório ficam para uma retomada da V2
3. **V3 — Otimização**: estudo de desempenho e paralelização (ver `Relatórios/AVALIACAO_DE_DESEMPENHO.md`)
4. Publicar visão + PF na versão oficial
5. Livro-caixa de energia e marcos na Demografia
6. Calibração de custos com campanhas
7. Luz contínua → temperatura → correntes

## 0. Interface

- **Melhorar a apresentação das informações da célula** (pedido de 21/11/2026): a forma como Células › Indivíduo descreve a célula e o código genético ainda não está boa; revisitar numa próxima ocasião.
- **Aba Física** (pedido de 21/11/2026, a avaliar): ao lado de Biologia e Ambiente, com sol e intensidade do sol. Conversa com a luz contínua do item 2.
- **Nova interface** em Tauri + React, com o motor intocado. Ver [NOVA_INTERFACE.md](NOVA_INTERFACE.md).
- Reorganizar a aba Células / indivíduo em grupos recolhíveis (adiado do backlog; já previsto na nova interface).

## 1. Pronto, fora da versão oficial

- **Visão genérica + quimiorrecepção PF**: está no código (commit `516e938`) e na versão `V1.1.0`, em validação. Falta validar e publicar.

## 2. Ambiente: luz → temperatura → correntes

- Máscara solar de intensidade contínua (hoje é 0 ou 1), interpolando reprodução e bônus de vida do fitoplâncton.
- Curva que liga luz ao raio efetivo de visão.
- PF com persistência, difusão, decaimento e transporte pela corrente.
- Temperatura derivada da luz, com custo térmico para as células.
- Temperatura afetando as correntes.

## 3. Telemetria e análise

- **Livro-caixa de energia** por tick (saídas, transferências, entradas), com balanço que fecha. Pré-requisito para recalibrar custos.
- Gastos de energia na Demografia.
- **Marcos de mudança de configuração** nos gráficos e linha em degraus do mínimo de reprodução. Ver [proposta](../Propostas/PROPOSTA_TELEMETRIA_DEMOGRAFICA.md).
- Genealogia real (`parent_id` / `root_lineage_id`).
- Evolução temporal dos grupos genéticos no pós-morte da sessão.
- Telemetria química: massa total, emissoras, custo agregado.

## 4. Calibração (depende do livro-caixa)

- Campanhas comparando alcance 240 migrado com descendentes mutados.
- Recalibrar custo-base do sensor e escolher a curva de produção.
- Avaliar um gene de alcance para a percepção química.

## 5. Genética

- **Canais químicos sobrepostos à fiação** (achado de 21/11/2026): os 9 bits que escolhem P1/P2/P3 ficam dentro do registro de fiação #63 (bits 1207–1215). Decidir se separa numa região própria, o que exigiria migrar genomas e saves.
- Atualizar `DOCUMENTACAO_ANALISE_GENETICA.md`, que ainda descreve o genoma antigo; o mapa atual está na V2.0.2 (Células › Código genético).
- **Mutação adaptativa**: três coeficientes hereditários. Decisão em aberto: faixa 0,25×–3,36× ou teto 4×. Ver [proposta](../Propostas/PROPOSTA_MUTACAO_ADAPTATIVA.md).
- Decidir se a longevidade entra no `Genome` (hoje não afeta cor, PG nem similaridade).

## 6. Laboratório

- Matriz seed × ambiente.
- Ler `.ecosrun` para campanhas derivadas (reexecutar eventos, variações controladas).
- Val 2.0 no Laboratório.

## 7. Desempenho

- Separar no perfil as consultas de `World::update_sensors()` e otimizar (33,6% no cenário de estresse).

## 8. Interação celular

- Telemetria de predação (tentativas, energia, mortes por engolfamento, distribuição de raios).
- Sensores direcionais de outras células, se ainda fizerem sentido com a visão genérica.

## Sem plano por enquanto

Genoma de tamanho variável e criação de neurônios, obstáculos, convecção física,
evolução de potência e alcance dos receptores químicos.

## Documentos desatualizados

- `Propostas/PROPOSTA_INTERACAO_CELULAR_E_COMBATE.md` diz que osmose e engolfamento são só proposta, mas fagocitose e sensores de força já existem.
- `Documentação/DOCUMENTACAO_ANALISE_GENETICA.md` descreve o genoma antigo (32 slots, 1.088 bits).
