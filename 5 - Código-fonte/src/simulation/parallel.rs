//! Ajudantes de paralelismo do tick. Abaixo de um limite de itens a etapa
//! roda em sequência: dividir trabalho pequeno entre threads custa mais do
//! que o próprio trabalho. O resultado é idêntico nos dois caminhos, pois
//! cada item é calculado de forma independente e a ordem é preservada.
//!
//! Desde a V4.0.1 as etapas paralelas não usam mais as filas do `rayon`.
//! Elas usam um grupo próprio de threads que **fica acordado** entre uma
//! etapa e outra: depois de terminar um trabalho, cada thread gira por até
//! `SPIN_TIME` esperando o próximo, em vez de dormir e precisar ser acordada
//! pelo sistema. Com o tick em fração de milissegundo, acordar threads
//! custava tanto quanto o trabalho dividido. O `rayon` continua dizendo
//! quantas threads usar (`rayon::current_num_threads()`), então o seletor de
//! threads da interface, o `--threads` do benchmark e os testes seguem
//! valendo como antes.
//!
//! O trabalho é cortado em blocos contíguos. Cada bloco é feito inteiro por
//! uma única thread e escreve só nos próprios índices, portanto quem pega
//! qual bloco não muda nenhum resultado.

use std::{
    cell::Cell,
    collections::HashMap,
    panic::{self, AssertUnwindSafe},
    sync::{
        Arc, Mutex, MutexGuard, OnceLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    thread::{self, Thread},
    time::{Duration, Instant},
};

/// Quanto tempo uma thread do grupo gira à espera da próxima etapa antes de
/// dormir. Entre as etapas de um tick a espera é bem menor que isso; com o
/// limitador de velocidade da interface, entre um tick e outro, elas dormem
/// e não gastam processador.
const SPIN_TIME: Duration = Duration::from_micros(300);

/// Blocos por thread quando o tamanho é livre: alguns a mais que o número de
/// threads, para que uma thread que termine cedo pegue outro bloco.
const BLOCKS_PER_THREAD: usize = 4;

/// Aplica `step` a cada item, em paralelo quando há itens suficientes.
pub fn for_each_mut<T: Send>(
    items: &mut [T],
    threshold: usize,
    min_len: usize,
    step: impl Fn(&mut T) + Sync + Send,
) {
    let Some(threads) = parallel_threads(items.len(), threshold) else {
        items.iter_mut().for_each(step);
        return;
    };
    let len = items.len();
    let base = SharedPtr(items.as_mut_ptr());
    run_blocks(
        threads,
        len,
        block_len(len, threads, min_len),
        &|start, end| {
            // Segurança: os blocos são disjuntos e cada um é de uma só thread.
            let block =
                unsafe { std::slice::from_raw_parts_mut(base.get().add(start), end - start) };
            block.iter_mut().for_each(&step);
        },
    );
}

/// `(0..count).map(f).collect()`, em paralelo quando vale a pena.
pub fn map_indices<R: Send>(
    count: usize,
    threshold: usize,
    min_len: usize,
    f: impl Fn(usize) -> R + Sync + Send,
) -> Vec<R> {
    let Some(threads) = parallel_threads(count, threshold) else {
        return (0..count).map(f).collect();
    };
    let mut results: Vec<R> = Vec::with_capacity(count);
    let out = SharedPtr(results.as_mut_ptr());
    run_blocks(
        threads,
        count,
        block_len(count, threads, min_len),
        &|start, end| {
            for index in start..end {
                // Segurança: cada índice é escrito uma única vez, por uma thread.
                unsafe { out.get().add(index).write(f(index)) };
            }
        },
    );
    // Segurança: `run_blocks` só volta depois que todos os blocos 0..count
    // foram escritos.
    unsafe { results.set_len(count) };
    results
}

/// Como `for_each_mut`, mas devolve um valor por item, na ordem da lista.
/// Serve para fundir duas passadas: mover o item e já anotar algo sobre ele.
pub fn map_mut<T: Send, R: Send>(
    items: &mut [T],
    threshold: usize,
    min_len: usize,
    f: impl Fn(&mut T) -> R + Sync + Send,
) -> Vec<R> {
    let Some(threads) = parallel_threads(items.len(), threshold) else {
        return items.iter_mut().map(f).collect();
    };
    let len = items.len();
    let base = SharedPtr(items.as_mut_ptr());
    let mut results: Vec<R> = Vec::with_capacity(len);
    let out = SharedPtr(results.as_mut_ptr());
    run_blocks(
        threads,
        len,
        block_len(len, threads, min_len),
        &|start, end| {
            for index in start..end {
                // Segurança: cada índice é lido e escrito por uma só thread.
                unsafe {
                    let item = &mut *base.get().add(index);
                    out.get().add(index).write(f(item));
                }
            }
        },
    );
    // Segurança: todos os índices 0..len foram escritos.
    unsafe { results.set_len(len) };
    results
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
    let threads = if parallel && chunk_len > 0 {
        parallel_threads(items.len(), 0)
    } else {
        None
    };
    let Some(threads) = threads else {
        step(0, items);
        return;
    };
    let len = items.len();
    let base = SharedPtr(items.as_mut_ptr());
    run_blocks(threads, len, chunk_len, &|start, end| {
        // Segurança: as faixas são disjuntas e cada uma é de uma só thread.
        let band = unsafe { std::slice::from_raw_parts_mut(base.get().add(start), end - start) };
        step(start, band);
    });
}

/// Quantas threads usar numa etapa com `len` itens, ou `None` para rodar em
/// sequência. Dentro de um trabalho do grupo tudo é sequencial: não há
/// paralelismo aninhado.
fn parallel_threads(len: usize, threshold: usize) -> Option<usize> {
    if len < threshold || INSIDE_JOB.with(Cell::get) {
        return None;
    }
    // Threads que giram só rendem com um núcleo para cada uma: pedir mais
    // que o processador tem deixaria threads esperando por núcleo.
    let threads = rayon::current_num_threads().min(available_cores());
    (threads > 1).then_some(threads)
}

fn available_cores() -> usize {
    static CORES: OnceLock<usize> = OnceLock::new();
    *CORES.get_or_init(|| thread::available_parallelism().map_or(1, |cores| cores.get()))
}

fn block_len(len: usize, threads: usize, min_len: usize) -> usize {
    len.div_ceil(threads * BLOCKS_PER_THREAD)
        .max(min_len)
        .max(1)
}

/// Corta `0..len` em blocos de `block` itens e os distribui entre as threads
/// do grupo. Cada bloco vai inteiro para uma thread, que pega o próximo livre.
fn run_blocks(threads: usize, len: usize, block: usize, f: &(dyn Fn(usize, usize) + Sync)) {
    let block = block.max(1);
    let blocks = len.div_ceil(block);
    let next = AtomicUsize::new(0);
    pool_for(threads).run(&|_worker| {
        loop {
            let index = next.fetch_add(1, Ordering::Relaxed);
            if index >= blocks {
                break;
            }
            let start = index * block;
            f(start, (start + block).min(len));
        }
    });
}

/// Ponteiro compartilhado entre as threads de uma etapa. Só é usado para
/// índices disjuntos, e só enquanto `run` não voltou.
struct SharedPtr<T>(*mut T);

impl<T> Clone for SharedPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for SharedPtr<T> {}

impl<T> SharedPtr<T> {
    /// Por método, e não `.0`, para a closure capturar o `SharedPtr` inteiro
    /// (que é `Sync`) e não só o ponteiro cru.
    fn get(self) -> *mut T {
        self.0
    }
}

// Segurança: cada thread acessa índices diferentes de dados `Send`.
unsafe impl<T: Send> Send for SharedPtr<T> {}
unsafe impl<T: Send> Sync for SharedPtr<T> {}

// ── Grupo de threads que fica acordado ─────────────────────────────────

type Job<'a> = dyn Fn(usize) + Sync + 'a;

thread_local! {
    static INSIDE_JOB: Cell<bool> = const { Cell::new(false) };
}

/// Um grupo por número de threads, criado na primeira vez que é pedido e
/// mantido até o programa fechar (threads paradas dormem).
fn pool_for(threads: usize) -> Arc<SpinPool> {
    static POOLS: OnceLock<Mutex<HashMap<usize, Arc<SpinPool>>>> = OnceLock::new();
    let pools = POOLS.get_or_init(Default::default);
    Arc::clone(
        lock(pools)
            .entry(threads)
            .or_insert_with(|| Arc::new(SpinPool::new(threads))),
    )
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct Shared {
    /// Sobe a cada trabalho publicado; as threads giram olhando para ele.
    epoch: AtomicU64,
    job: Mutex<Option<&'static Job<'static>>>,
    /// Threads auxiliares que ainda não terminaram o trabalho atual.
    remaining: AtomicUsize,
    panicked: AtomicBool,
}

struct SpinPool {
    shared: Arc<Shared>,
    /// Threads auxiliares; quem chama `run` é a thread 0.
    workers: Vec<Thread>,
    /// Um trabalho por vez (os testes rodam em paralelo e dividem o grupo).
    run_lock: Mutex<()>,
}

impl SpinPool {
    fn new(threads: usize) -> Self {
        let shared = Arc::new(Shared {
            epoch: AtomicU64::new(0),
            job: Mutex::new(None),
            remaining: AtomicUsize::new(0),
            panicked: AtomicBool::new(false),
        });
        let workers = (1..threads)
            .map(|index| {
                let shared = Arc::clone(&shared);
                thread::Builder::new()
                    .name(format!("ngu-calculo-{index}"))
                    .spawn(move || worker_loop(&shared, index))
                    .expect("não foi possível criar uma thread de cálculo")
                    .thread()
                    .clone()
            })
            .collect();
        Self {
            shared,
            workers,
            run_lock: Mutex::new(()),
        }
    }

    /// Executa `job(índice_da_thread)` em todas as threads do grupo, inclusive
    /// a que chamou, e só volta quando todas terminaram.
    fn run(&self, job: &Job<'_>) {
        let _one_at_a_time = lock(&self.run_lock);
        // Segurança: o trabalho só é usado pelas threads até `remaining`
        // chegar a zero, e esta função não volta antes disso. Logo a
        // referência nunca é usada depois que `job` deixa de existir.
        let published: &'static Job<'static> = unsafe { std::mem::transmute(job) };
        *lock(&self.shared.job) = Some(published);
        self.shared.panicked.store(false, Ordering::Relaxed);
        self.shared
            .remaining
            .store(self.workers.len(), Ordering::Relaxed);
        self.shared.epoch.fetch_add(1, Ordering::Release);
        for worker in &self.workers {
            // Barato se a thread está girando; acorda-a se estiver dormindo.
            worker.unpark();
        }
        let mine = panic::catch_unwind(AssertUnwindSafe(|| run_job(job, 0)));
        let mut spins = 0u32;
        while self.shared.remaining.load(Ordering::Acquire) != 0 {
            spins = spins.wrapping_add(1);
            if spins % 1_024 == 0 {
                thread::yield_now();
            } else {
                std::hint::spin_loop();
            }
        }
        *lock(&self.shared.job) = None;
        if let Err(payload) = mine {
            panic::resume_unwind(payload);
        }
        if self.shared.panicked.load(Ordering::Relaxed) {
            panic!("uma thread de cálculo entrou em pânico durante uma etapa paralela");
        }
    }
}

fn worker_loop(shared: &Shared, index: usize) {
    let mut seen = 0u64;
    loop {
        let waiting_since = Instant::now();
        let mut spins = 0u32;
        loop {
            let epoch = shared.epoch.load(Ordering::Acquire);
            if epoch != seen {
                seen = epoch;
                break;
            }
            spins = spins.wrapping_add(1);
            if spins % 64 == 0 && waiting_since.elapsed() > SPIN_TIME {
                // `run` sempre chama `unpark` depois de publicar: se o aviso
                // chegar antes deste `park`, ele volta na hora.
                thread::park();
            } else {
                std::hint::spin_loop();
            }
        }
        let job = *lock(&shared.job);
        if let Some(job) = job {
            if panic::catch_unwind(AssertUnwindSafe(|| run_job(job, index))).is_err() {
                shared.panicked.store(true, Ordering::Relaxed);
            }
        }
        shared.remaining.fetch_sub(1, Ordering::AcqRel);
    }
}

fn run_job(job: &Job<'_>, worker: usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            INSIDE_JOB.with(|inside| inside.set(false));
        }
    }
    INSIDE_JOB.with(|inside| inside.set(true));
    let _reset = Reset;
    job(worker);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_threads<R: Send>(threads: usize, f: impl FnOnce() -> R + Send) -> R {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(f)
    }

    #[test]
    fn helpers_match_the_sequential_result_for_any_thread_count() {
        for threads in [1, 2, 3, 4] {
            for len in [0usize, 1, 7, 1_023, 5_000] {
                let expected: Vec<u64> = (0..len as u64).map(|i| i * i + 1).collect();
                let mapped = with_threads(threads, || {
                    map_indices(len, 0, 16, |i| (i as u64) * (i as u64) + 1)
                });
                assert_eq!(mapped, expected);

                let mut stepped: Vec<u64> = (0..len as u64).collect();
                with_threads(threads, || {
                    for_each_mut(&mut stepped, 0, 8, |v| *v = *v * *v + 1)
                });
                assert_eq!(stepped, expected);

                let mut moved: Vec<u64> = (0..len as u64).collect();
                let flags = with_threads(threads, || {
                    map_mut(&mut moved, 0, 32, |v| {
                        *v = *v * *v + 1;
                        *v % 2 == 0
                    })
                });
                assert_eq!(moved, expected);
                assert_eq!(
                    flags,
                    expected.iter().map(|v| v % 2 == 0).collect::<Vec<_>>()
                );

                let mut banded = vec![0u64; len];
                with_threads(threads, || {
                    for_each_chunk_mut(&mut banded, 100, true, |start, band| {
                        for (offset, value) in band.iter_mut().enumerate() {
                            let index = (start + offset) as u64;
                            *value = index * index + 1;
                        }
                    })
                });
                assert_eq!(banded, expected);
            }
        }
    }

    #[test]
    fn a_panic_in_a_parallel_step_reaches_the_caller_and_the_group_keeps_working() {
        let result = panic::catch_unwind(|| {
            with_threads(4, || {
                map_indices(10_000, 0, 16, |index| {
                    assert!(index != 9_000, "falha proposital do teste");
                    index
                })
            })
        });
        assert!(result.is_err());
        let after = with_threads(4, || map_indices(10_000, 0, 16, |index| index));
        assert_eq!(after[9_999], 9_999);
    }
}
