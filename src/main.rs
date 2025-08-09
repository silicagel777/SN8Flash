use clap::{Parser, Subcommand, ValueEnum};
use indicatif::ProgressBar;
use sonixflash::flasher::{Flasher, RomBank};
use sonixflash::transport::{ResetType, SerialPortTransport};
use std::io::Write;

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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Connect and read Chip ID
    ChipId,

    /// Read flash
    Read {
        /// Read offset in bytes
        #[arg(long, default_value_t = 0)]
        offset: u16,

        /// Read size in bytes
        #[arg(long)]
        size: u16,

        /// Output file path, use "-" for stdout
        #[arg(long)]
        path: Option<String>,
    },

    /// Write flash
    Write {
        /// Write page size in bytes. It is usually 32 bytes, but
        /// can be 64 bytes for big chips. Check datasheet!
        #[arg(long, default_value_t = 0x20)]
        page_size: u8,

        /// Input file path
        #[arg(short, long)]
        path: String,
    },
    /// Erase flash
    Erase,

    /// Reset chip
    Reset,
}

fn main() {
    let args = Cli::parse();

    simplelog::TermLogger::init(
        args.verbose.log_level_filter(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
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
    flasher.set_rom_bank(RomBank::Main);

    match args.command {
        Some(Commands::ChipId) => {
            log::info!("Connecting...");
            let chip_id = flasher.connect();
            log::info!("Chip ID is {chip_id:#X}");

            log::info!("Resetting chip...");
            flasher.reset();
        }
        Some(Commands::Read {
            ref path,
            offset,
            size,
        }) => {
            log::info!("Connecting...");
            let chip_id = flasher.connect();
            log::info!("Chip ID is {chip_id:#X}");

            log::info!("Reading {size} bytes of flash...");
            let mut data_read = vec![0; size as usize];
            let bar = ProgressBar::new(data_read.len() as _);
            flasher.read_flash(offset, &mut data_read, &|x| bar.set_position(x));
            bar.finish();

            match path {
                None => {
                    let cfg = nu_pretty_hex::HexConfig {
                        address_offset: offset as usize,
                        ..nu_pretty_hex::HexConfig::default()
                    };
                    println!("{}", nu_pretty_hex::config_hex(&data_read, cfg));
                }
                Some(path) if path == "-" => {
                    std::io::stdout().write_all(&data_read).unwrap();
                }
                Some(path) => {
                    log::info!("Saving to {path}...");
                    std::fs::write(path, &data_read).unwrap();
                }
            }

            log::info!("Resetting chip...");
            flasher.reset();
        }
        Some(Commands::Write {
            ref path,
            page_size,
        }) => {
            log::info!("Opening {path}...");
            let mut data_write = std::fs::read(path).unwrap();
            data_write.resize(data_write.len().next_multiple_of(page_size as usize), 0);

            log::info!("Connecting...");
            let chip_id = flasher.connect();
            log::info!("Chip ID is {chip_id:#X}");

            log::info!("Erasing flash...");
            flasher.erase_flash();

            log::info!("Writing {} bytes of flash...", data_write.len());
            let bar = ProgressBar::new(data_write.len() as _);
            flasher.write_flash(&data_write, page_size, &|x| bar.set_position(x));
            bar.finish();

            log::info!("Verifying write...");
            let mut data_verify = vec![0; data_write.len()];
            let bar = ProgressBar::new(data_verify.len() as _);
            flasher.read_flash(0, &mut data_verify, &|x| bar.set_position(x));
            bar.finish();

            let verify_errors: Vec<_> = std::iter::zip(data_write, data_verify)
                .enumerate()
                .filter(|(_, (x, y))| x != y)
                .map(|(i, _)| i)
                .collect();
            if !verify_errors.is_empty() {
                log::error!("Verify error! Mismatched offsets are: {verify_errors:08X?}");
            }

            log::info!("Resetting chip...");
            flasher.reset();
        }
        Some(Commands::Erase) => {
            log::info!("Connecting...");
            let chip_id = flasher.connect();
            log::info!("Chip ID is {chip_id:#X}");

            log::info!("Erasing flash...");
            flasher.erase_flash();

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
