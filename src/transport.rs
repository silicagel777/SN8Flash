pub trait Transport {
    fn write(&mut self, data: &[u8]);
    fn read(&mut self, data: &mut [u8]);
    fn set_reset(&mut self, level: bool);
}

#[derive(Clone, Copy, PartialEq)]
pub enum ResetType {
    Rts,
    Dtr,
}

#[derive(gset::Getset)]
pub struct SerialPortTransport {
    port: Box<dyn serialport::SerialPort>,

    #[getset(get_copy, vis = "pub")]
    #[getset(set, vis = "pub")]
    reset_type: ResetType,

    #[getset(get_copy, vis = "pub")]
    #[getset(set, vis = "pub")]
    reset_invert: bool,
}

impl SerialPortTransport {
    pub fn new(port: &str) -> Self {
        let port = serialport::new(port, 750_000)
            .timeout(std::time::Duration::from_millis(50))
            .open()
            .expect("Failed to open serial port");
        SerialPortTransport {
            port,
            reset_type: ResetType::Rts,
            reset_invert: false,
        }
    }
}

impl Transport for SerialPortTransport {
    fn write(&mut self, data: &[u8]) {
        log::trace!("Writing {data:02X?}");
        self.port
            .write_all(data)
            .expect("Failed to write into serial port");
        self.port.flush().expect("Failed to flush serial port");
        let mut res = [0];
        for byte in data {
            self.port
                .read_exact(&mut res)
                .expect("Failed to read written data from serial port, check RX+TX connection");
            assert_eq!(
                *byte, res[0],
                "Write/read data mismatch, check RX+TX connection"
            );
        }
        log::trace!("Written {} bytes", data.len());
    }

    fn read(&mut self, data: &mut [u8]) {
        log::trace!("Reading {} bytes", data.len());
        for i in 0..data.len() {
            self.port
                .read_exact(&mut data[i..i + 1])
                .expect("Failed to read from serial port");
        }
        log::trace!("Read {data:02X?}");
    }

    fn set_reset(&mut self, mut level: bool) {
        log::trace!("Setting reset to {level}");
        if self.reset_invert {
            level = !level;
        }
        match self.reset_type {
            ResetType::Rts => self
                .port
                .write_request_to_send(level)
                .expect("Failed to set RTS pin"),
            ResetType::Dtr => self
                .port
                .write_data_terminal_ready(level)
                .expect("Failed to set DTR pin"),
        }
    }
}
