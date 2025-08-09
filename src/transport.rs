use crate::error::{Error, Result};

pub trait Transport {
    fn write(&mut self, data: &[u8]) -> Result<()>;
    fn read(&mut self, data: &mut [u8]) -> Result<()>;
    fn set_reset(&mut self, level: bool) -> Result<()>;
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
    pub fn new(port: &str) -> Result<Self> {
        let port = serialport::new(port, 750_000)
            .timeout(std::time::Duration::from_millis(50))
            .dtr_on_open(false)
            .open()?;

        Ok(SerialPortTransport {
            port,
            reset_type: ResetType::Rts,
            reset_invert: false,
        })
    }
}

impl Transport for SerialPortTransport {
    fn write(&mut self, data: &[u8]) -> Result<()> {
        log::trace!("Writing {data:02X?}");
        self.port.write_all(data)?;
        self.port.flush()?;
        let mut res = [0];
        for byte in data {
            self.port
                .read_exact(&mut res)
                .map_err(Error::WriteReadFailed)?;
            if *byte != res[0] {
                return Err(Error::WriteReadMismatch);
            }
        }
        log::trace!("Written {} bytes", data.len());
        Ok(())
    }

    fn read(&mut self, data: &mut [u8]) -> Result<()> {
        log::trace!("Reading {} bytes", data.len());
        for i in 0..data.len() {
            self.port.read_exact(&mut data[i..i + 1])?
        }
        log::trace!("Read {data:02X?}");
        Ok(())
    }

    fn set_reset(&mut self, mut level: bool) -> Result<()> {
        log::trace!("Setting reset to {level}");
        if self.reset_invert {
            level = !level;
        }
        match self.reset_type {
            ResetType::Rts => {
                self.port.write_request_to_send(level)?;
            }
            ResetType::Dtr => {
                self.port.write_data_terminal_ready(level)?;
            }
        }
        Ok(())
    }
}
