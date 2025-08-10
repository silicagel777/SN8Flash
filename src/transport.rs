use crate::error::{Error, Result};
use std::time::Duration;

pub trait Transport {
    fn write(&mut self, data: &[u8]) -> Result<()>;
    fn read(&mut self, data: &mut [u8]) -> Result<()>;
    fn set_reset(&mut self, level: bool) -> Result<()>;
    fn set_timeout(&mut self, value: Duration) -> Result<()>;
    fn timeout(&self) -> Result<Duration>;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResetType {
    Rts,
    Dtr,
}

#[derive(gset::Getset)]
pub struct SerialPortTransport {
    port: serial2::SerialPort,

    #[getset(get_copy, vis = "pub")]
    #[getset(set, vis = "pub")]
    reset_type: ResetType,

    #[getset(get_copy, vis = "pub")]
    #[getset(set, vis = "pub")]
    reset_invert: bool,
}

impl SerialPortTransport {
    pub fn new(path: &str) -> Result<Self> {
        let mut port = serial2::SerialPort::open(path, 750_000)?;
        port.set_read_timeout(Duration::from_millis(50))?;
        port.set_dtr(false)?;

        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "Opened serial port {} with baud rate {} and timeout {:?}",
                path,
                port.get_configuration()
                    .and_then(|conf| conf.get_baud_rate())
                    .unwrap_or_default(),
                port.get_read_timeout().unwrap_or_default(),
            );
        }

        Ok(Self {
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
        let mut mismatch = false;
        for byte in data {
            self.port
                .read_exact(&mut res)
                .map_err(Error::WriteReadFailed)?;
            if *byte != res[0] {
                mismatch = true;
            }
        }
        if mismatch {
            return Err(Error::WriteReadMismatch);
        }
        log::trace!("Written {} bytes", data.len());
        Ok(())
    }

    fn read(&mut self, data: &mut [u8]) -> Result<()> {
        log::trace!("Reading {} bytes", data.len());
        for i in 0..data.len() {
            self.port.read_exact(&mut data[i..=i])?;
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
                self.port.set_rts(level)?;
            }
            ResetType::Dtr => {
                self.port.set_dtr(level)?;
            }
        }
        log::trace!("Set reset to {level}");
        Ok(())
    }

    fn set_timeout(&mut self, value: Duration) -> Result<()> {
        Ok(self.port.set_read_timeout(value)?)
    }

    fn timeout(&self) -> Result<Duration> {
        Ok(self.port.get_read_timeout()?)
    }
}
