mod cpu;
mod mmu;
mod window;

pub fn run(frequency: u32, file_path: &str) {
    print!("{:?}", frequency);
    print!("{:?}", file_path);
}