extern crate clap;

use clap::Parser;

/// chip8 - A Chip-8 interpreter written in Rust
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The ch8 binary file to load
    file: String,

    /// Sets the CPU frequency in hz
    #[arg(short, long, default_value_t = 500)]
    freq: u32,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();
    chip8::run(args.freq, &args.file).await;
}
