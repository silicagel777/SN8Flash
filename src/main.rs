use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::ProgressBar;
use sonixflash::firmware::Firmware;
use sonixflash::flasher::{Flasher, RomBank};
use sonixflash::transport::{ResetType, SerialPortTransport};
use std::io::{Read, Write};
use std::process::ExitCode;
use structural_convert::StructuralConvert;

#[derive(Clone, Copy, Debug, StructuralConvert, ValueEnum)]
#[convert(into(ResetType))]
enum ArgResetType {
    Rts,
    Dtr,
}

#[derive(Clone, Copy, Debug, StructuralConvert, ValueEnum)]
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
    #[arg(short = 'p', long)]
    port: String,

    /// Reset signal type
    #[arg(short = 'r', long, default_value = "rts")]
    reset_type: ArgResetType,

    /// Do not reset chip after running a command
    #[arg(long, default_value_t = false)]
    no_final_reset: bool,

    /// Invert reset pin
    #[arg(short = 'i', long, default_value_t = false)]
    reset_invert: bool,

    /// Custom reset duration in milliseconds
    #[arg(long, default_value_t = 10)]
    reset_duration: u64,

    /// Custom connect duration in microseconds
    #[arg(long, default_value_t = 1666)]
    connect_duration: u64,

    /// Flash page size in bytes. It is usually 32 bytes, but
    /// can be 64 bytes for big chips. Check datasheet!
    #[arg(short = 'x', long, default_value_t = 0x20)]
    page_size: u8,

    /// ROM bank to work with
    #[arg(long, default_value = "main")]
    rom_bank: ArgRomBank,

    /// Allow writing or erasing non-main ROM bank
    ///
    /// Be careful, this can brick your chip! I've accidentally wiped boot
    /// parameter area on SN8F570212, and the chip would no longer leave the
    /// built-in bootloader until I've restored it back. Fun stuff!
    #[arg(long, default_value_t = false)]
    dangerous_allow_write_non_main_bank: bool,

    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Connect and read chip ID
    ChipId,

    /// Erase flash
    Erase,

    /// Read flash
    Read {
        /// Read size in bytes
        #[arg(short = 's', long)]
        size: u16,

        /// Read offset in bytes
        #[arg(short = 'o', long, default_value_t = 0)]
        offset: u16,

        /// Output file path (raw binary),
        /// use "-" for stdout dump or omit for pretty-print
        #[arg(short = 'f', long = "file")]
        path: Option<String>,
    },

    /// Verify flash
    Verify {
        /// Input file path (raw binary or Intel HEX),
        /// use "-" for raw binary from stdout
        #[arg(short = 'f', long = "file")]
        path: String,

        /// Verify offset in bytes
        #[arg(short = 'o', long, default_value_t = 0)]
        offset: u16,
    },

    /// Write flash
    Write {
        /// Input file path (raw binary or Intel HEX),
        /// use "-" for raw binary from stdout
        #[arg(short = 'f', long = "file")]
        path: String,

        /// Write offset in bytes
        #[arg(short = 'o', long, default_value_t = 0)]
        offset: u16,

        /// Do not erase chip before writing
        #[arg(long, default_value_t = false)]
        no_erase: bool,

        /// Do not verify after writing
        #[arg(long, default_value_t = false)]
        no_verify: bool,
    },
}

fn load_firmware(path: &str, page_size: u8, offset: u16) -> anyhow::Result<Firmware> {
    let firmware = if path == "-" {
        log::info!("Reading raw binary from stdin...");
        let mut raw = Vec::new();
        std::io::stdin().read_to_end(&mut raw)?;
        Firmware::from_raw_bytes(raw, page_size.into(), offset.into())?
    } else {
        log::info!("Opening {path}...");
        Firmware::from_file(path, page_size.into(), offset.into())?
    };
    Ok(firmware)
}

fn dump_firmware(path: Option<&str>, data: &[u8], offset: u16) -> anyhow::Result<()> {
    match path {
        None => {
            let cfg = nu_pretty_hex::HexConfig {
                address_offset: offset as usize,
                ..nu_pretty_hex::HexConfig::default()
            };
            println!("{}", nu_pretty_hex::config_hex(&data, cfg));
        }
        Some("-") => {
            log::info!("Dumping to stdout...");
            std::io::stdout()
                .write_all(data)
                .context("Failed to write to standard output")?;
        }
        Some(path) => {
            log::info!("Saving to {path}...");
            std::fs::write(path, data).context(format!("Failed to save {path}"))?;
        }
    }
    Ok(())
}

fn run(args: &Cli) -> anyhow::Result<()> {
    let transport = {
        log::info!("Opening port {}...", args.port);
        let mut serial = SerialPortTransport::new(&args.port)?;
        serial.set_reset_type(args.reset_type.into());
        serial.set_reset_invert(args.reset_invert);
        Box::new(serial)
    };

    let mut flasher = Flasher::new(transport);
    flasher.set_final_reset(!args.no_final_reset);
    flasher.set_reset_duration_ms(args.reset_duration);
    flasher.set_connect_duration_us(args.connect_duration);
    flasher.set_rom_bank(args.rom_bank.into());
    flasher.set_dangerous_allow_write_non_main_bank(args.dangerous_allow_write_non_main_bank);

    log::info!("Connecting...");
    let chip_id = flasher.connect()?;
    log::info!("Chip ID is {chip_id:#X}");

    match args.command {
        Commands::ChipId => {
            // Already printed it!
        }
        Commands::Erase => {
            log::info!("Erasing flash...");
            flasher.erase_flash()?;
        }
        Commands::Read {
            ref path,
            offset,
            size,
        } => {
            log::info!("Reading {size} bytes of flash...");
            let mut data_read = vec![0; size as usize];
            let bar = ProgressBar::new(data_read.len() as _);
            flasher.read_flash(offset, &mut data_read, &|x| bar.inc(x))?;
            bar.finish();

            dump_firmware(path.as_deref(), &data_read, offset)?;
        }
        Commands::Verify { ref path, offset } => {
            let firmware = load_firmware(path, args.page_size, offset)?;

            log::info!("Verifying flash...");
            let bar = ProgressBar::new(firmware.len() as _);
            flasher.verify_flash(&firmware, &|x| bar.inc(x))?;
            bar.finish();
        }
        Commands::Write {
            ref path,
            offset,
            no_erase,
            no_verify,
        } => {
            let firmware = load_firmware(path, args.page_size, offset)?;

            if !no_erase {
                log::info!("Erasing flash...");
                flasher.erase_flash()?;
            }

            log::info!("Writing {} bytes of flash...", firmware.len());
            let bar = ProgressBar::new(firmware.len() as _);
            flasher.write_flash(&firmware, &|x| bar.inc(x))?;
            bar.finish();

            if !no_verify {
                log::info!("Verifying write...");
                let bar = ProgressBar::new(firmware.len() as _);
                flasher.verify_flash(&firmware, &|x| bar.inc(x))?;
                bar.finish();
            }
        }
    }

    log::info!("Done!");
    Ok(())
}

fn main() -> ExitCode {
    let args = Cli::parse();

    simplelog::TermLogger::init(
        args.verbose.log_level_filter(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");

    if let Err(err) = run(&args) {
        if log::log_enabled!(log::Level::Debug) {
            log::error!("{err:?}");
        } else {
            log::error!("{err:#}");
        }
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
