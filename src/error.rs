pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Serial port error")]
    SerialError(#[from] serialport::Error),

    #[error("IO error")]
    IOError(#[from] std::io::Error),

    #[error("Failed to read written data from serial port, check RX+TX connection")]
    WriteReadFailed(#[source] std::io::Error),

    #[error("Write/read data mismatch, check RX+TX connection")]
    WriteReadMismatch,

    #[error("Invalid handshake response")]
    HandshakeError,

    #[error("Invalid write check result")]
    WriteCheckError,

    #[error("Verify mismatch at offsets {0:X?}")]
    VerifyMismatch(Vec<usize>),

    #[error("Intel HEX data is not valid UTF-8")]
    IHexDecodeError(#[source] std::str::Utf8Error),

    #[error("Intel HEX parse error on line {1}")]
    IHexParseError(#[source] ihex::ReaderError, usize),
}
