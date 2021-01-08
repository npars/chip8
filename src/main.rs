extern crate clap;
use clap::{Arg, App, value_t_or_exit};
use chip8;

fn main() {
    let matches = App::new("chip8")
        .arg(
            Arg::with_name("FREQUENCY")
                .short("f")
                .long("freq")
                .takes_value(true)
                .default_value("500")
                .help("Sets the CPU frequency in hz"),
        )
        .arg(Arg::with_name("FILE")
            .help("The ch8 binary file to load")
            .required(true)
            .index(1))
        .get_matches();

    let frequency = value_t_or_exit!(matches.value_of("FREQUENCY"), u32);
    let file_path = matches.value_of("FILE").unwrap();

    chip8::run(frequency, file_path);
}
