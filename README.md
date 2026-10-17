# New Gods Unlimited

Um ecossistema celular evolutivo, escrito em Rust e visualizado com Macroquad.
O núcleo reúne organismos, ambiente, rede neural, genoma e persistência de
sessões; a interface permite observar e configurar uma execução.

## Começar por aqui

- [Visão do projeto](docs/DOCUMENTACAO_PRELIMINAR.md)
- [Mapa da arquitetura](docs/ESTRUTURA_DE_ARQUIVOS_PROPOSTA.md)
- [Índice completo da documentação](docs/README.md)
- [Guia da entrega portátil](docs/DISTRIBUICAO_PORTATIL.md)

## Estrutura

```text
src/                 código-fonte Rust
docs/                decisões de produto, arquitetura e propostas
tests/               testes de integração (a criar)
assets/presets/      presets distribuídos com o projeto
data/runs/           sessões salvas fora do executável
data/campaigns/      campanhas do laboratório
scripts/             compilação e publicação local
releases/windows/    entrega portátil oficial e seu histórico
target*/             saída recriável do compilador (ignorada pelo Git)
```

## Executar e publicar

Para desenvolver, execute `cargo run --release`. Para gerar a entrega portátil
do Windows, rode `scripts/COMPILAR_PORTATIL.bat`. A saída oficial é criada em
`releases/windows/current/` e o arquivo compactado em `releases/windows/`.

## Git

O repositório registra fonte, documentação, scripts e presets de referência.
Saídas de compilação, execuções salvas, campanhas e entregas são locais e não
entram nos commits por padrão.
