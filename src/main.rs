extern crate clap;
use chip8;
use clap::{value_t_or_exit, App, Arg};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let matches = App::new("chip8")
        .arg(
            Arg::with_name("FREQUENCY")
                .short("f")
                .long("freq")
                .takes_value(true)
                .default_value("500")
                .help("Sets the CPU frequency in hz"),
        )
        .arg(
            Arg::with_name("FILE")
                .help("The ch8 binary file to load")
                .required(true)
                .index(1),
        )
        .get_matches();

    let frequency = value_t_or_exit!(matches.value_of("FREQUENCY"), u32);
    let file_path = matches.value_of("FILE").unwrap();

    chip8::run(frequency, file_path).await;
}
