# Distribuição portátil

## Entrega

O arquivo `NewGodsUnlimited.exe` é autônomo: basta copiá-lo para uma pasta
gravável no computador ou HD externo e executá-lo. Não exige Rust, Cargo nem
instalação.

## Dados ao lado do executável

Na primeira vez que o usuário salvar algo, o programa cria na mesma pasta:

- `runs_salvas/` — sessões e telemetria exportadas;
- `presets_sessao.ecopreset` — presets criados pelo usuário.

Assim, mover a pasta inteira preserva os dados. Para compartilhar um ambiente
limpo, entregue somente o `.exe`.

## Compatibilidade

Distribuição validada para Windows 64 bits. O teste de dependências confirma
que o executável usa apenas bibliotecas do próprio Windows; não há DLL de
terceiro para copiar junto.
