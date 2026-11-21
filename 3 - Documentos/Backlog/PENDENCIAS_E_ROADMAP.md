# Pendências e roadmap

Levantamento de 21/11/2026 de tudo que os documentos marcam como pendente,
conferido contra o código. Agrupado por tema, na ordem de dependência sugerida.

## Ordem sugerida

1. ~~Reorganização das pastas e fluxo de versões~~ (feito em 21/11/2026)
2. **V2 — Revamp da interface**: V2.0.0 em teste; faltam salvar/carregar, presets e Laboratório
3. Publicar visão + PF na versão oficial
4. Livro-caixa de energia e marcos na Demografia
5. Calibração de custos com campanhas
6. Luz contínua → temperatura → correntes

## 0. Interface

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
