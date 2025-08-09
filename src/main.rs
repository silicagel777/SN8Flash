use clap::{Parser, ValueEnum};
use indicatif::ProgressBar;
use sonixflash::flasher::{Flasher, RomBank};
use sonixflash::transport::{ResetType, SerialPortTransport};
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

    /// Verbosity level
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

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

    /// Flash page size for writing (check datasheet)
    #[arg(long, default_value_t = 0x20)]
    page_size: usize,
}

fn main() {
    let args = CommandArgs::parse();

    simplelog::TermLogger::init(
        args.verbose.log_level_filter(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");

    let port = &args.port;
    let transport = {
        log::info!("Opening port {port}...");
        let mut serial = SerialPortTransport::new(port);
        serial.set_reset_type(args.reset_type.into());
        serial.set_reset_invert(args.reset_invert);
        Box::new(serial)
    };

    let mut flasher = Flasher::new(transport);
    flasher.set_reset_duration_ms(args.reset_duration);
    flasher.set_connect_duration_us(args.connect_duration);
    flasher.set_page_size(args.page_size);
    flasher.set_rom_bank(RomBank::Main);

    log::info!("Connecting...");
    let chip_id = flasher.connect();
    log::info!("Chip ID is {chip_id:#X}");

    log::info!("Reading flash...");
    let mut data_read = [0; 4096];
    let bar = ProgressBar::new(100);
    flasher.read_flash(0, &mut data_read, &|x| bar.set_position(x.into()));
    bar.finish();
    let file_name = "dump_read.bin";
    log::info!("Saving to {file_name}...");
    let mut file = File::create(file_name).unwrap();
    file.write_all(&data_read).unwrap();

    log::info!("Erasing flash...");
    flasher.erase_flash();

    log::info!("Writing flash...");
    let mut data_write = [0; 4096];
    let file_name = "src_blink.bin";
    let mut file = File::open(file_name).unwrap();
    file.read_exact(&mut data_write).unwrap();
    let bar = ProgressBar::new(100);
    flasher.write_flash(&data_write, &|x| bar.set_position(x.into()));

    log::info!("Verifying write...");
    let mut data_verify = [0; 4096];
    let bar = ProgressBar::new(100);
    flasher.read_flash(0, &mut data_verify, &|x| bar.set_position(x.into()));
    bar.finish();
    let file_name = "dump_verify.bin";
    log::info!("Saving to {file_name}...");
    let mut file = File::create(file_name).unwrap();
    file.write_all(&data_verify).unwrap();

    log::info!("Resetting chip...");
    flasher.reset();

    assert_eq!(data_write, data_verify);

    log::info!("Done!");
}
