//! Ajudantes de paralelismo do tick. Abaixo de um limite de itens a etapa
//! roda em sequência: dividir trabalho pequeno entre threads custa mais do
//! que o próprio trabalho. O resultado é idêntico nos dois caminhos, pois
//! cada item é calculado de forma independente e a ordem é preservada.

use rayon::prelude::*;

/// Aplica `step` a cada item, em paralelo quando há itens suficientes.
pub fn for_each_mut<T: Send>(
    items: &mut [T],
    threshold: usize,
    min_len: usize,
    step: impl Fn(&mut T) + Sync + Send,
) {
    if items.len() >= threshold && rayon::current_num_threads() > 1 {
        items.par_iter_mut().with_min_len(min_len).for_each(step);
    } else {
        items.iter_mut().for_each(step);
    }
}

/// `(0..count).map(f).collect()`, em paralelo quando vale a pena.
pub fn map_indices<R: Send>(
    count: usize,
    threshold: usize,
    min_len: usize,
    f: impl Fn(usize) -> R + Sync + Send,
) -> Vec<R> {
    if count >= threshold && rayon::current_num_threads() > 1 {
        (0..count).into_par_iter().with_min_len(min_len).map(f).collect()
    } else {
        (0..count).map(f).collect()
    }
}

/// Como `for_each_mut`, mas devolve um valor por item, na ordem da lista.
/// Serve para fundir duas passadas: mover o item e já anotar algo sobre ele.
pub fn map_mut<T: Send, R: Send>(
    items: &mut [T],
    threshold: usize,
    min_len: usize,
    f: impl Fn(&mut T) -> R + Sync + Send,
) -> Vec<R> {
    if items.len() >= threshold && rayon::current_num_threads() > 1 {
        items.par_iter_mut().with_min_len(min_len).map(f).collect()
    } else {
        items.iter_mut().map(f).collect()
    }
}

/// Divide `items` em faixas contíguas de `chunk_len` e chama
/// `step(início_da_faixa, faixa)` em cada uma, em paralelo quando `parallel`
/// é verdadeiro e há mais de uma thread. No caminho sequencial a lista
/// inteira é uma faixa só. Como cada faixa é exclusiva de uma thread, quem
/// escreve em `items[i]` é sempre a mesma chamada, na mesma ordem interna.
pub fn for_each_chunk_mut<T: Send>(
    items: &mut [T],
    chunk_len: usize,
    parallel: bool,
    step: impl Fn(usize, &mut [T]) + Sync + Send,
) {
    if parallel && chunk_len > 0 && rayon::current_num_threads() > 1 {
        items
            .par_chunks_mut(chunk_len)
            .enumerate()
            .for_each(|(chunk, slice)| step(chunk * chunk_len, slice));
    } else {
        step(0, items);
    }
}
