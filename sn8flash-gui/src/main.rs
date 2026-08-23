#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod logger;
use logger::CallbackLogger;

use image::GenericImageView;
use sn8flash_lib::chip::ChipInfo;
use sn8flash_lib::flasher::{Flasher, RomBank};
use sn8flash_lib::transport::{ResetType, SerialPortTransport};
use std::rc::Rc;
use wxdragon::prelude::*;

wxdragon::include_xrc!("ui/main.xrc", MainUI);
wxdragon::include_xrc!("ui/connection_circuit.xrc", ConnectionCircuitUI);

fn main() {
    let _ = wxdragon::main(|_| {
        wxdragon::app::set_appearance(Appearance::System);
        let ui = MainUI::new(None, false);
        let config_rc = Rc::new(Config::new(
            env!("CARGO_PKG_NAME"),
            None,
            None,
            None,
            ConfigStyle::USE_LOCAL_FILE | ConfigStyle::USE_SUBDIR,
        ));

        let config = config_rc.clone();
        let load_config = move || {
            if config.read_long("config_version", 1) == 1 {
                ui.serial_port.set_value(&config.read_string("port", ""));
                ui.reset_type
                    .set_selection(config.read_long("reset_type", 0) as _);
                let def_reset_duration = Flasher::DEFAULT_RESET_DURATION_MS as i64;
                ui.reset_duration
                    .set_value(config.read_long("reset_duration", def_reset_duration) as _);
                let def_connect_delay = Flasher::DEFAULT_CONNECT_DELAY_US as i64;
                ui.connect_delay
                    .set_value(config.read_long("connect_delay", def_connect_delay) as _);
                ui.no_final_reset
                    .set_value(config.read_bool("no_final_reset", false));
                ui.invert_reset
                    .set_value(config.read_bool("invert_reset", false));
                ui.rom_bank
                    .set_selection(config.read_long("rom_bank", 0) as _);
                ui.page_size
                    .set_selection(config.read_long("page_size", 0) as _);
                ui.read_file_input
                    .set_value(&config.read_string("read_file_path", ""));
                ui.read_offset_enable
                    .set_value(config.read_bool("read_offset_enable", false));
                ui.read_offset_input
                    .enable(ui.read_offset_enable.get_value());
                ui.read_offset_input
                    .set_value(config.read_long("read_offset", 0) as _);
                ui.read_size_enable
                    .set_value(config.read_bool("read_size_enable", false));
                ui.read_size_input.enable(ui.read_size_enable.get_value());
                ui.read_size_input
                    .set_value(config.read_long("read_size", 4096) as _);
                ui.write_file_input
                    .set_value(&config.read_string("write_file_path", ""));
                ui.write_offset_enable
                    .set_value(config.read_bool("write_offset_enable", false));
                ui.write_offset_input
                    .enable(ui.write_offset_enable.get_value());
                ui.write_offset_input
                    .set_value(config.read_long("write_offset", 0) as _);
                ui.write_skip_erase
                    .set_value(config.read_bool("write_skip_erase", false));
                ui.write_skip_verify
                    .set_value(config.read_bool("write_skip_verify", false));
            }
        };

        let config = config_rc.clone();
        let save_config = move || {
            config.write_long("config_version", 1);
            config.write_string("port", &ui.serial_port.get_value());
            config.write_long(
                "reset_type",
                ui.reset_type.get_selection().unwrap_or(0) as _,
            );
            config.write_long("reset_duration", ui.reset_duration.value() as _);
            config.write_long("connect_delay", ui.connect_delay.value() as _);
            config.write_bool("no_final_reset", ui.no_final_reset.get_value());
            config.write_bool("invert_reset", ui.invert_reset.get_value());
            config.write_long("rom_bank", ui.rom_bank.get_selection().unwrap_or(0) as _);
            config.write_long("page_size", ui.page_size.get_selection().unwrap_or(0) as _);
            config.write_string("read_file_path", &ui.read_file_input.get_value());
            config.write_bool("read_offset_enable", ui.read_offset_enable.get_value());
            config.write_long("read_offset", ui.read_offset_input.value() as _);
            config.write_bool("read_size_enable", ui.read_size_enable.get_value());
            config.write_long("read_size", ui.read_size_input.value() as _);
            config.write_string("write_file_path", &ui.write_file_input.get_value());
            config.write_bool("write_offset_enable", ui.write_offset_enable.get_value());
            config.write_long("write_offset", ui.write_offset_input.value() as _);
            config.write_bool("write_skip_erase", ui.write_skip_erase.get_value());
            config.write_bool("write_skip_verify", ui.write_skip_verify.get_value());
            config.flush(false)
        };

        let refresh_ports = move || {
            let prev_value = ui.serial_port.get_value();
            ui.serial_port.clear();
            for port in SerialPortTransport::available_ports().unwrap_or_default() {
                if let Ok(port) = port.into_os_string().into_string() {
                    ui.serial_port.append(&port);
                }
            }
            if ui.serial_port.get_value().is_empty() && ui.serial_port.get_count() > 0 {
                ui.serial_port.set_selection(0);
            }
            if !prev_value.is_empty() {
                ui.serial_port.set_value(&prev_value);
            }
        };

        let log_error = move |result: anyhow::Result<()>| match result {
            Err(err) => {
                if log::log_enabled!(log::Level::Debug) {
                    log::error!("{err:?}");
                } else {
                    log::error!("{err:#}");
                }
            }
            _ => {}
        };

        let run_connect = move || -> anyhow::Result<Flasher> {
            let transport = {
                let port = ui.serial_port.get_value();
                log::info!("Opening port {}...", port);
                let mut serial = SerialPortTransport::new(&port)?;

                serial.set_reset_type(match ui.reset_type.get_selection() {
                    Some(0) => ResetType::Rts,
                    Some(1) => ResetType::Dtr,
                    _ => ResetType::Rts,
                });
                serial.set_reset_invert(ui.invert_reset.get_value());
                Box::new(serial)
            };

            let mut flasher = Flasher::new(transport);
            flasher.set_final_reset(!ui.no_final_reset.get_value());
            flasher.set_reset_duration_ms(ui.reset_duration.value() as _);
            flasher.set_connect_delay_us(ui.connect_delay.value() as _);
            flasher.set_rom_bank(match ui.rom_bank.get_selection() {
                Some(0) => RomBank::Main,
                Some(1) => RomBank::Boot,
                _ => RomBank::Main,
            });
            flasher.set_dangerous_allow_write_non_main_bank(false);

            log::info!("Connecting...");
            flasher.connect()?;

            let chip_id = flasher.chip_id()?;
            let chip_info = ChipInfo::from_chip_id(chip_id);
            log::info!(
                "Chip ID is {:#X} ({})",
                chip_id,
                chip_info.map_or("unknown chip".to_string(), |x| x.to_string())
            );
            Ok(flasher)
        };

        ui.read_offset_enable.on_toggled(move |e| {
            ui.read_offset_input.enable(e.is_checked());
        });

        ui.read_size_enable.on_toggled(move |e| {
            ui.read_size_input.enable(e.is_checked());
        });

        ui.write_offset_enable.on_toggled(move |e| {
            ui.write_offset_input.enable(e.is_checked());
        });

        ui.item_refresh_ports.on_click(move |_| {
            refresh_ports();
        });

        ui.item_clear_log.on_click(move |_| {
            ui.log_edit.clear();
        });

        ui.item_exit.on_click(move |_| {
            ui.main_frame.close(false);
        });

        ui.item_connection_circuit.on_click(move |_| {
            let dialog_ui = ConnectionCircuitUI::new(Some(&ui.main_frame), false);

            let doc_link = dialog_ui.doc_link;
            doc_link.set_url(env!("CARGO_PKG_HOMEPAGE"));
            doc_link.set_label(env!("CARGO_PKG_HOMEPAGE"));

            let bitmap = dialog_ui.circuit_bitmap;
            let image_bytes = include_bytes!("../../docs/sn8flash-connection.png");
            let image_object =
                image::load_from_memory_with_format(image_bytes, image::ImageFormat::Png).unwrap();
            let image_rgba = image_object.to_rgba8();
            let (image_w, image_h) = image_object.dimensions();
            let bitmap_object = Bitmap::from_rgba(image_rgba.as_raw(), image_w, image_h).unwrap();
            bitmap.set_bitmap(&bitmap_object);
            bitmap.set_scale_mode(ScaleMode::AspectFit);

            let dialog = dialog_ui.connection_circuit_dialog;
            dialog.set_size(Size::new(800, 380));
            dialog.center();
            dialog.show_modal();
        });

        ui.item_about.on_click(move |_| {
            let mut info = AboutDialogInfo::new();
            info.set_name(env!("CARGO_PKG_NAME"));
            info.set_version(env!("CARGO_PKG_VERSION"));
            info.set_description(env!("CARGO_PKG_DESCRIPTION"));
            info.set_licence(env!("CARGO_PKG_LICENSE"));
            info.set_website(env!("CARGO_PKG_HOMEPAGE"));
            for developer in env!("CARGO_PKG_AUTHORS").split(":") {
                info.add_developer(developer);
            }
            show_about_box(&info, Some(&ui.main_frame));
        });

        ui.run_chip_id_button.on_click({
            move |_| {
                log_error((move || -> anyhow::Result<()> {
                    run_connect()?;
                    Ok(())
                })());
            }
        });

        ui.main_frame.on_close(move |_| {
            save_config();
        });

        log::set_boxed_logger(Box::new(CallbackLogger::new(
            log::Level::Info,
            Box::new({
                move |message| {
                    if ui.log_edit.is_modified() {
                        ui.log_edit.append_text("\n");
                    }
                    ui.log_edit.append_text(&message);
                }
            }),
        )))
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .expect("Could not initialize log");

        load_config();
        refresh_ports();

        ui.main_frame.set_title(&format!(
            "{} ({})",
            ui.main_frame.get_title(),
            env!("CARGO_PKG_VERSION")
        ));
        ui.log_edit.auto_scroll_if_at_end(0);
        ui.main_frame.centre();
        ui.main_frame.show(true);
        log::info!("Ready");
    });
}
