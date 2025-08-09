use clap::{Parser, ValueEnum};
use indicatif::ProgressBar;
use sonixflash::flasher::{Flasher, ResetType};
use std::{
    fs::File,
    io::{Read, Write},
};

#[derive(ValueEnum, Clone, Debug)]
enum CommandResetType {
    Rts,
    Dtr,
}

impl From<CommandResetType> for ResetType {
    fn from(value: CommandResetType) -> Self {
        match value {
            CommandResetType::Rts => ResetType::Rts,
            CommandResetType::Dtr => ResetType::Dtr,
        }
    }
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
    #[arg(long, default_value = "rts")]
    reset_type: CommandResetType,

    /// Invert reset pin
    #[arg(long, default_value_t = false)]
    reset_invert: bool,

    /// Custom reset duration in milliseconds (for debugging)
    #[arg(long, default_value_t = 10)]
    reset_duration: u64,

    /// Custom reset duration in microseconds (for debugging)
    #[arg(long, default_value_t = 1666)]
    connect_duration: u64,
}

fn main() {
    let args = CommandArgs::parse();
    let port = &args.port;

    println!("Opening port {port}...");
    let mut sf = Flasher::new(
        port,
        args.reset_type.into(),
        args.reset_invert,
        args.reset_duration,
        args.connect_duration,
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
