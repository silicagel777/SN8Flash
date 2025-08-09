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

#[derive(getset::Getters, getset::Setters)]
pub struct SerialPortTransport {
    port: Box<dyn serialport::SerialPort>,

    #[getset(get = "pub with_prefix", set = "pub")]
    reset_type: ResetType,

    #[getset(get = "pub with_prefix", set = "pub")]
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
        let written = self
            .port
            .write(data)
            .expect("Failed to write into serial port");
        assert_eq!(written, data.len(), "Some bytes were not written");
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
    }

    fn read(&mut self, data: &mut [u8]) {
        for i in 0..data.len() {
            self.port
                .read_exact(&mut data[i..i + 1])
                .expect("Failed to read from serial port");
        }
    }

    fn set_reset(&mut self, mut level: bool) {
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
