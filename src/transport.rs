pub trait Transport {
    fn write(&mut self, data: &[u8]);
    fn write_batch_begin(&mut self);
    fn write_batch_commit(&mut self);
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
    batch_counter: u64,
    batch_data: Vec<u8>,

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
            batch_counter: 0,
            batch_data: Vec::new(),
        }
    }
}

impl Transport for SerialPortTransport {
    fn write(&mut self, data: &[u8]) {
        if self.batch_counter > 0 {
            log::trace!("Appending write batch with {data:02X?}");
            self.batch_data.extend(data);
        } else {
            log::trace!("Writing {data:02X?}");
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
            log::trace!("Written {} bytes", data.len());
        }
    }

    fn write_batch_begin(&mut self) {
        log::trace!("Write batch begin");
        self.batch_counter += 1;
        log::trace!("Write batch counter {}", self.batch_counter);
    }

    fn write_batch_commit(&mut self) {
        log::trace!("Write batch commit");
        self.batch_counter -= 1;
        if self.batch_counter == 0 {
            let batch_data = std::mem::take(&mut self.batch_data);
            self.write(&batch_data);
        }
        log::trace!("Write batch counter {}", self.batch_counter);
    }

    fn read(&mut self, data: &mut [u8]) {
        if self.batch_counter != 0 {
            panic!("Can't read during write batch")
        }
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
