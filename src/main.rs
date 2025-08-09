use clap::{Parser, ValueEnum};
use indicatif::ProgressBar;
use std::{
    fs::File,
    io::{Read, Write},
};

mod sonixflash;

#[derive(ValueEnum, Clone, Debug)]
enum CommandResetType {
    Rts,
    Dtr,
}

/// Sonix SN8F flash tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CommandArgs {
    /// Serial port
    #[arg(short, long)]
    port: String,

    /// Reset signal type. RTS is recommended, as DTR is toggled on serial port
    /// open, resulting in extra reset
    #[arg(short, long, default_value = "rts")]
    reset_type: CommandResetType,

    /// Invert reset pin
    #[arg(short, long, default_value_t = false)]
    reset_invert: bool,

    /// Custom reset duration in milliseconds (for debugging)
    #[arg(short, long, default_value_t = 10)]
    reset_duration: u64,

    /// Custom reset duration in microseconds (for debugging)
    #[arg(short, long, default_value_t = 1666)]
    connect_duration: u64,
}

fn main() {
    let args = CommandArgs::parse();
    let port = &args.port;
    let reset_type = match args.reset_type {
        CommandResetType::Rts => sonixflash::ResetType::Rts,
        CommandResetType::Dtr => sonixflash::ResetType::Dtr,
    };
    let reset_invert = args.reset_invert;
    let reset_duration_ms = args.reset_duration;
    let connect_duration_us = args.connect_duration;

    println!("Opening port {port}...");
    let mut sf = sonixflash::SonixFlash::new(
        port,
        reset_type,
        reset_invert,
        reset_duration_ms,
        connect_duration_us,
    );

    println!("Connecting...");
    let chip_id = sf.connect();
    println!("Chip ID is {chip_id:#X}");

    sf.test();

    println!("Reading flash...");
    let mut data_read = [0; 4096];
    let bar = ProgressBar::new(100);
    sf.read_flash(0, &mut data_read, 0, &|x| bar.set_position(x.into()));
    bar.finish();
    let file_name = "dump_read.bin";
    println!("Saving to {file_name}...");
    let mut file = File::create(file_name).unwrap();
    file.write_all(&data_read).unwrap();

    println!("Erasing flash...");
    sf.erase_flash();

    println!("Writing flash...");
    let mut data_write = [0; 4096];
    let file_name = "src_blink.bin";
    let mut file = File::open(file_name).unwrap();
    file.read_exact(&mut data_write).unwrap();
    let bar = ProgressBar::new(100);
    sf.write_flash(&data_write, 0, &|x| bar.set_position(x.into()));

    println!("Verifying write...");
    let mut data_verify = [0; 4096];
    let bar = ProgressBar::new(100);
    sf.read_flash(0, &mut data_verify, 0, &|x| bar.set_position(x.into()));
    bar.finish();
    let file_name = "dump_verify.bin";
    println!("Saving to {file_name}...");
    let mut file = File::create(file_name).unwrap();
    file.write_all(&data_verify).unwrap();

    println!("Resetting chip...");
    sf.reset();

    assert_eq!(data_write, data_verify);

    println!("Done!");
}
