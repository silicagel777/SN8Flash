use clap::{Parser, Subcommand, ValueEnum};
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
struct Cli {
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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Connect and read Chip ID
    ChipId,
    /// Read flash
    Read {
        /// Output file path
        #[arg(short, long)]
        path: String,
    },
    /// Write flash
    Write {
        /// Input file path
        #[arg(short, long)]
        path: String,
    },
    /// Reset chip
    Reset,
}

fn main() {
    let args = Cli::parse();

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

    match args.command {
        Some(Commands::ChipId) => {
            log::info!("Connecting...");
            let chip_id = flasher.connect();
            log::info!("Chip ID is {chip_id:#X}");

            log::info!("Resetting chip...");
            flasher.reset();
        }
        Some(Commands::Read { path }) => {
            log::info!("Connecting...");
            let chip_id = flasher.connect();
            log::info!("Chip ID is {chip_id:#X}");

            log::info!("Reading flash...");
            let mut data_read = [0; 4096];
            let bar = ProgressBar::new(100);
            flasher.read_flash(0, &mut data_read, &|x| bar.set_position(x.into()));
            bar.finish();

            log::info!("Saving to {path}...");
            let mut file = File::create(path).unwrap();
            file.write_all(&data_read).unwrap();

            log::info!("Resetting chip...");
            flasher.reset();
        }
        Some(Commands::Write { path }) => {
            log::info!("Opening {path}...");
            let mut file = File::open(path).unwrap();
            let file_size = file.metadata().unwrap().len();
            let mut data_write = vec![0; file_size.try_into().unwrap()];
            file.read_exact(&mut data_write).unwrap();

            log::info!("Connecting...");
            let chip_id = flasher.connect();
            log::info!("Chip ID is {chip_id:#X}");

            log::info!("Erasing flash...");
            flasher.erase_flash();

            log::info!("Writing flash...");
            let bar = ProgressBar::new(100);
            flasher.write_flash(&data_write, &|x| bar.set_position(x.into()));

            log::info!("Verifying write...");
            let mut data_verify = vec![0; data_write.len()];
            let bar = ProgressBar::new(100);
            flasher.read_flash(0, &mut data_verify, &|x| bar.set_position(x.into()));
            bar.finish();

            if data_write != data_verify {
                log::error!("Verify error!");
            }

            log::info!("Resetting chip...");
            flasher.reset();
        }
        Some(Commands::Reset) => {
            log::info!("Resetting chip...");
            flasher.reset();
        }
        None => {
            log::warn!("No command chosen, doing nothing...");
        }
    };

    log::info!("Done!");
}
