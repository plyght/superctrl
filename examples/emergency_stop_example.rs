use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

fn main() {
    let stop_flag = Arc::new(AtomicBool::new(false));

    let stop_flag_worker = Arc::clone(&stop_flag);
    let worker = std::thread::spawn(move || {
        for i in 0..100 {
            if stop_flag_worker.load(Ordering::Acquire) {
                println!("🛑 Worker stopped at iteration {}", i);
                return;
            }

            println!("Worker iteration {}", i);
            std::thread::sleep(Duration::from_millis(500));
        }
        println!("Worker completed all iterations");
    });

    std::thread::sleep(Duration::from_secs(3));

    println!("Triggering emergency stop...");
    stop_flag.store(true, Ordering::Release);

    worker.join().unwrap();
    println!("Worker thread has stopped");
}
