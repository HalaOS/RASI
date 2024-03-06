use std::{
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering},
};

use divan::{bench, Bencher};

fn main() {
    divan::main();
}

#[bench(threads, sample_size = 1000)]
fn bench_park_mutex_queue(bencher: Bencher) {
    let map = parking_lot::Mutex::new(VecDeque::new());

    let counter = AtomicUsize::new(0);

    bencher.bench(|| {
        let id = counter.fetch_add(1, Ordering::Relaxed);

        map.lock().push_back(id);
        map.lock().pop_back();
    })
}

#[bench(threads, sample_size = 1000)]
fn bench_std_mutex_queue(bencher: Bencher) {
    let map = std::sync::Mutex::new(VecDeque::new());

    let counter = AtomicUsize::new(0);

    bencher.bench(|| {
        let id = counter.fetch_add(1, Ordering::Relaxed);

        map.lock().unwrap().push_back(id);
        map.lock().unwrap().pop_back();
    })
}

// #[bench(threads, sample_size = 1000)]
// fn bench_dash_queue(bencher: Bencher) {
//     let map = Queue::new();

//     let counter = AtomicUsize::new(0);

//     bencher.bench(|| {
//         let id = counter.fetch_add(1, Ordering::Relaxed);

//         map.push(id);
//         map.pop();
//     })
// }
