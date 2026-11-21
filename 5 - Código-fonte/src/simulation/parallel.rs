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
