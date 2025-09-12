pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error")]
    IOError(#[from] std::io::Error),

    #[error("Failed to read written data from serial port, check RX+TX connection")]
    WriteReadFailed(#[source] std::io::Error),

    #[error("Write/read data mismatch, check RX+TX connection")]
    WriteReadMismatch,

    #[error("No handshake response, check reset circuit and chip connection")]
    HandshakeResponseTimeout,

    #[error("Invalid handshake response {0:X?}")]
    HandshakeResponseMismatch([u8; 4]),

    #[error("Invalid write check result {0:X}")]
    WriteCheckError(u16),

    #[error("Writing to a non-main ROM bank is not allowed")]
    NonMainBankWrite,

    #[error("Erasing a non-main ROM bank is not allowed")]
    NonMainBankErase,

    #[error("Verify mismatch at offsets {0:X?}")]
    VerifyMismatch(Vec<usize>),

    #[error("Intel HEX data is not valid UTF-8")]
    IHexDecodeError(#[source] std::str::Utf8Error),

    #[error("Intel HEX parse error on line {1}")]
    IHexParseError(#[source] ihex::ReaderError, usize),
}
