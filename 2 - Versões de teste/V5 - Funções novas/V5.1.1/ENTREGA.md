# V5.1.1 — Presets de sol, de construção e mundos embutidos

22/11/2026 · V5 - Funções novas

## Pedido
Logo depois do sol: presets de disposição do sol (Física › Sol) e de formato
da construção (Mapa › Construção). Os presets de mapa passam a juntar os dois.
Vários embutidos: labirintos, vários focos, luz basal. Presets de corrente ainda não.

## Entregue
- **Física › Presets de sol:** salva a malha de luz aplicada e também
  intensidade, basal e limiar. Carregar troca só o sol (paredes ficam).
- **Mapa › Presets de construção:** salva só as paredes. Carregar troca só as
  paredes (sol fica).
- **Mapa › Presets de mapa:** agora guardam também intensidade, basal e limiar.
  Novo: **combinar** uma construção com um sol em dois menus e aplicar; depois
  é só salvar com um nome. Mapas antigos abrem com o sol padrão.
- **Embutidos** (recriados se faltarem):
  - Sol: Foco central, Arquipélago (6 focos), Gradiente oeste-leste, Dois
    polos, Metade clara, Penumbra (40 manchas fracas com basal 2%).
  - Construção: Vazio, Oásis original, Labirinto, Duas câmaras, Espiral, Recifes.
  - Mapas: Oásis original, Labirinto com focos, Duas câmaras: clara e escura,
    Espiral com foco central, Recifes no arquipélago, Gradiente aberto.
- Pastas novas em `4 - Dados`: `Sol/` (.ngusol) e `Construção/` (.nguconst).
- A gravação de sessão registra a troca de sol ou de construção.

## Medição
Mesma máquina do GitHub, main (V5.1.0) × V5.1.1:

| Caso | Ganho | Estado igual | Comportamento igual |
|---|---:|---|---|
| Realista (100 células) | 0,99× | sim | sim |
| Estresse (300 células) | 0,89× | sim | sim |
| Sessão (100 células) | 1,01× | sim | sim |
| Sessão (200 células) | 0,97× | sim | sim |

**Resultado honesto:** a simulação não mudou (estado idêntico). Esta entrega
só mexe em arquivos e interface, então as diferenças de velocidade são
variação da máquina. 66 testes passam.

## Como testar
1. Física › Presets de sol › Carregar "Arquipélago" e ligar "Ver intensidade".
2. Mapa › Presets de construção › Carregar "Labirinto": o sol continua.
3. Mapa › Presets de mapa: escolher "Espiral" + "Dois polos", Aplicar
   combinação, dar nome e Salvar.
4. Pintar um sol seu, Aplicar, salvar como preset de sol e recarregar.

## Ficou de fora
Presets de corrente, visão limitada pela luz, otimização da luz por partícula
(a sessão ficou ~15% mais lenta na V5.1.0).
