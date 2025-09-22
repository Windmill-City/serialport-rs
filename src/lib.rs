use std::{io, os::windows::io::AsRawHandle};

#[cfg(unix)]
mod posix;
#[cfg(unix)]
pub use posix::TTYPort;

#[cfg(windows)]
mod windows;
use tokio::io::{AsyncRead, AsyncWrite};
#[cfg(windows)]
pub use windows::COMPort;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Not implemented for target platform")]
    NotImplemented,
}

pub type Result<T> = std::result::Result<T, Error>;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Parity {
    None,
    Odd,
    Even,
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StopBits {
    One,
    OnePointFive,
    Two,
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlowControl {
    None,
    Software,
    Hardware,
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Clear {
    Input,
    Output,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPortBuilder {
    path: String,
    baudrate: u32,
    data_bits: DataBits,
    flow_control: FlowControl,
    parity: Parity,
    stop_bits: StopBits,
}

impl SerialPortBuilder {
    #[must_use]
    pub fn path(mut self, path: &str) -> Self {
        path.clone_into(&mut self.path);
        self
    }

    #[must_use]
    pub fn baud_rate(mut self, baud_rate: u32) -> Self {
        self.baudrate = baud_rate;
        self
    }

    #[must_use]
    pub fn data_bits(mut self, data_bits: DataBits) -> Self {
        self.data_bits = data_bits;
        self
    }

    #[must_use]
    pub fn flow_control(mut self, flow_control: FlowControl) -> Self {
        self.flow_control = flow_control;
        self
    }

    #[must_use]
    pub fn parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    #[must_use]
    pub fn stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    #[cfg(windows)]
    pub fn open(self) -> Result<COMPort> {
        return windows::COMPort::open(&self);
    }
}

pub trait SerialPort: Send + AsyncRead + AsyncWrite + AsRawHandle {
    fn name(&self) -> String;
    fn baudrate(&self) -> Result<u32>;
    fn data_bits(&self) -> Result<DataBits>;
    fn flow_control(&self) -> Result<FlowControl>;
    fn parity(&self) -> Result<Parity>;
    fn stop_bits(&self) -> Result<StopBits>;
    fn set_baud_rate(&mut self, baud_rate: u32) -> Result<()>;
    fn set_data_bits(&mut self, data_bits: DataBits) -> Result<()>;
    fn set_flow_control(&mut self, flow_control: FlowControl) -> Result<()>;
    fn set_parity(&mut self, parity: Parity) -> Result<()>;
    fn set_stop_bits(&mut self, stop_bits: StopBits) -> Result<()>;
    fn set_rts(&mut self, level: bool) -> Result<()>;
    fn set_dtr(&mut self, level: bool) -> Result<()>;
    fn set_break(&mut self, level: bool) -> Result<()>;
    fn cts(&mut self) -> Result<bool>;
    fn dsr(&mut self) -> Result<bool>;
    fn ri(&mut self) -> Result<bool>;
    fn cd(&mut self) -> Result<bool>;
    fn bytes_to_read(&self) -> Result<u32>;
    fn bytes_to_write(&self) -> Result<u32>;
    fn clear(&self, buffer_to_clear: Clear) -> Result<()>;
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PortInfo {
    // path
    pub path: String,
    // friendly name
    pub name: String,
}

pub fn new<'a>(path: &str, baudrate: u32) -> SerialPortBuilder {
    SerialPortBuilder {
        path: path.into(),
        baudrate,
        data_bits: DataBits::Eight,
        flow_control: FlowControl::None,
        parity: Parity::None,
        stop_bits: StopBits::One,
    }
}

pub fn available_ports() -> Result<Vec<PortInfo>> {
    #[cfg(unix)]
    return crate::posix::available_ports();

    #[cfg(windows)]
    return crate::windows::available_ports();

    #[cfg(not(any(unix, windows)))]
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use crate::{available_ports, new};

    #[test]
    #[ignore = "manual"]
    fn test_available_ports() {
        println!("{:#?}", available_ports())
    }

    #[test]
    #[ignore = "manual"]
    fn test_open_close() {
        let builder = new("COM11", 115200);
        match builder.open() {
            Ok(serial) => drop(serial),
            Err(err) => println!("{}", err),
        }
    }
}
