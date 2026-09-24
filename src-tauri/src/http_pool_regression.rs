// Compile the exact vendored pool module so this exercises the shipped scheduler.
include!("../../vendor/tiny_http/src/util/task_pool.rs");

#[test]
fn burst_connections_start_without_waiting_for_existing_connections_to_close() {
    for _ in 0..4 {
        let pool = TaskPool::new();
        let idle_deadline = std::time::Instant::now() + Duration::from_secs(2);
        while pool.sharing.waiting_tasks.load(Ordering::Acquire) < MIN_THREADS {
            assert!(std::time::Instant::now() < idle_deadline, "Pool did not become idle");
            thread::yield_now();
        }
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let (started, events) = std::sync::mpsc::channel();
        for _ in 0..32 {
            let gate = gate.clone(); let started = started.clone();
            pool.spawn(Box::new(move || {
                let _ = started.send(());
                let mut released = gate.0.lock().unwrap();
                while !*released { released = gate.1.wait(released).unwrap(); }
            }));
        }
        drop(started);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut count = 0;
        while count < 32 {
            if events.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())).is_err() { break; }
            count += 1;
        }
        // Release every job before asserting, including an unfixed pool's queued jobs.
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        assert_eq!(count, 32, "Connections were stranded behind long-running workers");
    }
}
