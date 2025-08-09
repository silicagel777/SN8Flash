use clap::{Parser, Subcommand, ValueEnum};
use indicatif::ProgressBar;
use sonixflash::firmware::load_firmware;
use sonixflash::flasher::{Flasher, RomBank};
use sonixflash::transport::{ResetType, SerialPortTransport};
use std::io::Write;
use structural_convert::StructuralConvert;

#[derive(Clone, Debug, StructuralConvert, ValueEnum)]
#[convert(into(ResetType))]
enum ArgResetType {
    Rts,
    Dtr,
}

#[derive(Clone, Debug, StructuralConvert, ValueEnum)]
#[convert(into(RomBank))]
enum ArgRomBank {
    /// Main flash memory
    Main,
    /// Some sort of boot parameter area
    Boot,
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
    reset_type: ArgResetType,

    /// Do not reset chip after running a command
    #[arg(long, default_value_t = false)]
    no_final_reset: bool,

    /// Invert reset pin
    #[arg(long, default_value_t = false)]
    reset_invert: bool,

    /// Custom reset duration in milliseconds
    #[arg(long, default_value_t = 10)]
    reset_duration: u64,

    /// Custom connect duration in microseconds
    #[arg(long, default_value_t = 1666)]
    connect_duration: u64,

    /// ROM bank to work with
    #[arg(long, default_value = "main")]
    rom_bank: ArgRomBank,

    /// Allow writing or erasing non-main ROM bank. Be careful, this can brick your chip!
    /// I've accidentally wiped boot parameter area it on SN8F570212, and the chip would
    /// no longer leave the built-in bootloader until I've restored it back. Fun stuff!
    #[arg(long, default_value_t = false)]
    dangerous_allow_write_non_main_bank: bool,

    #[command(subcommand)]
    command: Commands,
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

        /// Output file path (raw binary), use "-" for stdout
        #[arg(long)]
        path: Option<String>,
    },

    /// Write flash
    Write {
        /// Flash page size in bytes. It is usually 32 bytes, but
        /// can be 64 bytes for big chips. Check datasheet!
        #[arg(long, default_value_t = 0x20)]
        page_size: u8,

        /// Input file path (raw binary or Intel HEX)
        #[arg(short, long)]
        path: String,

        /// Do not erase chip before writing
        #[arg(long, default_value_t = false)]
        no_erase: bool,

        /// Do not verify after writing
        #[arg(long, default_value_t = false)]
        no_verify: bool,
    },

    /// Erase flash
    Erase,
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

    let transport = {
        log::info!("Opening port {}...", args.port);
        let mut serial = SerialPortTransport::new(&args.port);
        serial.set_reset_type(args.reset_type.into());
        serial.set_reset_invert(args.reset_invert);
        Box::new(serial)
    };

    let mut flasher = Flasher::new(transport);
    flasher.set_reset_duration_ms(args.reset_duration);
    flasher.set_connect_duration_us(args.connect_duration);
    flasher.set_rom_bank(args.rom_bank.into());
    flasher.set_dangerous_allow_write_non_main_bank(args.dangerous_allow_write_non_main_bank);

    log::info!("Connecting...");
    let chip_id = flasher.connect();
    log::info!("Chip ID is {chip_id:#X}");

    match args.command {
        Commands::ChipId => {
            // Already printed it!
        }
        Commands::Read {
            ref path,
            offset,
            size,
        } => {
            log::info!("Reading {size} bytes of flash...");
            let mut data_read = vec![0; size as usize];
            let bar = ProgressBar::new(data_read.len() as _);
            flasher.read_flash(offset, &mut data_read, &|x| bar.inc(x));
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
        }
        Commands::Write {
            ref path,
            page_size,
            no_erase,
            no_verify,
        } => {
            log::info!("Opening {path}...");
            let mut data_write = load_firmware(path);
            data_write.resize(data_write.len().next_multiple_of(page_size as usize), 0xFF);

            if !no_erase {
                log::info!("Erasing flash...");
                flasher.erase_flash();
            }

            log::info!("Writing {} bytes of flash...", data_write.len());
            let bar = ProgressBar::new(data_write.len() as _);
            flasher.write_flash(&data_write, page_size, &|x| bar.inc(x));
            bar.finish();

            if !no_verify {
                log::info!("Verifying write...");
                let mut data_verify = vec![0; data_write.len()];
                let bar = ProgressBar::new(data_verify.len() as _);
                flasher.read_flash(0, &mut data_verify, &|x| bar.inc(x));
                bar.finish();
                let verify_errors: Vec<_> = std::iter::zip(data_write, data_verify)
                    .enumerate()
                    .filter(|(_, (x, y))| x != y)
                    .map(|(i, _)| i)
                    .collect();
                if !verify_errors.is_empty() {
                    log::error!("Verify error! Mismatched offsets are: {verify_errors:08X?}");
                }
            }
        }
        Commands::Erase => {
            log::info!("Erasing flash...");
            flasher.erase_flash();
        }
    };

    if !args.no_final_reset {
        log::info!("Resetting chip...");
        flasher.reset();
    }

    log::info!("Done!");
}
