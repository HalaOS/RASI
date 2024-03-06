use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use dashmap::DashMap;
use divan::{bench, Bencher};

fn main() {
    divan::main();
}

#[bench(threads, sample_size = 1000)]
fn bench_park_mutex_map(bencher: Bencher) {
    let map = parking_lot::Mutex::new(HashMap::<usize, usize>::new());

    let counter = AtomicUsize::new(0);

    bencher.bench(|| {
        let id = counter.fetch_add(1, Ordering::Relaxed);

        map.lock().insert(id, id);
        map.lock().remove(&id);
    })
}

#[bench(threads, sample_size = 1000)]
fn bench_std_mutex_map(bencher: Bencher) {
    let map = std::sync::Mutex::new(HashMap::<usize, usize>::new());

    let counter = AtomicUsize::new(0);

    bencher.bench(|| {
        let id = counter.fetch_add(1, Ordering::Relaxed);

        map.lock().unwrap().insert(id, id);
        map.lock().unwrap().remove(&id);
    })
}

#[bench(threads, sample_size = 1000)]
fn bench_dash_map(bencher: Bencher) {
    let map = DashMap::<usize, usize>::new();

    let counter = AtomicUsize::new(0);

    bencher.bench(|| {
        let id = counter.fetch_add(1, Ordering::Relaxed);

        map.insert(id, id);
        map.remove(&id);
    })
}
