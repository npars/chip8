mod cpu;
mod mmu;
mod window;

use mmu::Mmu;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::time::{self, Duration, Instant};

pub async fn run(frequency: u32, file_path: &str) {
    let duration_60hz: Duration = Duration::from_secs_f64(1f64 / 60f64);

    let mmu = Rc::new(RefCell::new(mmu::Chip8Mmu::new()));
    mmu.borrow_mut().load_program(file_path).unwrap();
    let window = Rc::new(RefCell::new(window::MiniFbWindow::new()));
    let mut cpu = cpu::Cpu::new(mmu.clone(), window.clone());

    let mut last_60hz_tick = Instant::now();
    let mut interval = time::interval(Duration::from_secs_f64(1f64 / (frequency as f64)));
    loop {
        let now = interval.tick().await;

        if (now - last_60hz_tick) >= duration_60hz {
            last_60hz_tick += duration_60hz;
            cpu.run_60hz_cycle();
        }

        cpu.run_cycle()
    }
}
