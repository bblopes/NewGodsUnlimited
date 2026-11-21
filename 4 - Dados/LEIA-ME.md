# Dados

Arquivos produzidos durante o desenvolvimento e os testes. São pessoais ou
resultados de experimentos, então ficam fora do GitHub.

| Pasta | Conteúdo |
| --- | --- |
| `runs/` | sessões salvas (`.ecosrun`); o benchmark usa a mais avançada daqui |
| `campanhas/` | campanhas do Laboratório (`.ecos`) |
| `presets/` | presets criados durante uma sessão |
| `benchmarks/` | relatórios JSON do `benchmark_simulacao` |

O programa oficial salva as runs dele em `1 - Versão oficial/runs_salvas/`.
Para guardar uma run como referência de experimento, copie o `.ecosrun` para
`runs/`.

Para transformar um preset em referência do projeto (embutido no programa),
mova uma cópia revisada para `5 - Código-fonte/assets/presets/` e faça um commit.
