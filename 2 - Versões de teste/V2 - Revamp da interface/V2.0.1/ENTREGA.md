# V2.0.1 — Salvar indivíduo

**Data:** 21/11/2026 · **Versão maior:** V2 — Revamp da interface

## Pedido

Clicar numa célula, salvar esse indivíduo e ele aparecer em Sessão ›
Configuração neural. Poder abrir cada configuração (Clássica, Motor, PF guia e
os salvos) para ver o indivíduo, apagar os salvos e salvar novos. O banco tem
que ser permanente: uma versão nova precisa encontrar o que foi salvo antes.

## Entregue

- **Células › Indivíduo › Salvar este indivíduo no banco**: pede um nome e
  guarda o genoma da célula.
- **Sessão › Configuração neural** lista os geradores fixos e, abaixo, os
  indivíduos salvos. O ícone de olho abre o indivíduo: origem, geração, cor,
  fiação da rede e código genético.
- No painel do indivíduo: **Usar como configuração neural** e, para os salvos,
  **Apagar do banco** (com confirmação).
- Reiniciar com um indivíduo salvo cria todos os fundadores com esse genoma
  (no modo Amadeus, uma célula no centro). Inserir célula também usa o salvo.
- O banco fica em `%APPDATA%\com.bblopes.newgodsunlimited\banco_de_individuos.json`,
  fora das pastas de versão. O genoma é gravado com o tamanho do formato, para
  versões futuras poderem migrá-lo.
- Motor: duas funções novas para criar e inserir células com um genoma exato.
  Nenhuma regra da simulação mudou.

## Como testar

1. Abra `New Gods Unlimited V2.0.1.exe`, clique numa célula e salve-a.
2. Em Sessão › Configuração neural, abra o salvo pelo olho e clique em
   **Usar como configuração neural**; reinicie.
3. Feche o programa, abra de novo (ou uma versão futura): o indivíduo continua lá.

## Ficou de fora

- Val 1.0 e Val 2.0 (decidido não trazer de volta).
- Renomear um indivíduo salvo e salvar grupos de vários indivíduos de uma vez.
