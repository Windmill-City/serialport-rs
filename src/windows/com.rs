use std::{
    io::Error,
    mem::MaybeUninit,
    os::windows::prelude::{AsRawHandle, IntoRawHandle, RawHandle},
    ptr::{null, null_mut},
};

use windows_sys::Win32::{
    Devices::Communication::{
        CLRDTR, CLRRTS, ClearCommBreak, ClearCommError, EVENPARITY, EscapeCommFunction,
        GetCommModemStatus, MS_CTS_ON, MS_DSR_ON, MS_RING_ON, MS_RLSD_ON, NOPARITY, ODDPARITY,
        ONE5STOPBITS, ONESTOPBIT, PURGE_RXABORT, PURGE_RXCLEAR, PURGE_TXABORT, PURGE_TXCLEAR,
        PurgeComm, SETDTR, SETRTS, SetCommBreak, TWOSTOPBITS,
    },
    Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{CreateFileW, FILE_FLAG_OVERLAPPED, OPEN_EXISTING},
    System::{IO::OVERLAPPED, Threading::CreateEventW},
};

use crate::{
    Clear, DataBits, FlowControl, Parity, Result, SerialPort, SerialPortBuilder, StopBits,
    windows::dcb::{self, BitOperation},
};

pub struct COMPort {
    path: String,
    handle: HANDLE,
    r_overlap: OVERLAPPED,
    w_overlap: OVERLAPPED,
}

impl COMPort {
    pub fn open(builder: &SerialPortBuilder) -> Result<COMPort> {
        let mut name = Vec::<u16>::with_capacity(4 + builder.path.len() + 1);

        if !builder.path.starts_with('\\') {
            name.extend(r"\\.\".encode_utf16());
        }

        name.extend(builder.path.encode_utf16());
        name.push(0);

        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                0 as HANDLE,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(Error::last_os_error().into());
        }

        let mut dcb = dcb::get_dcb(handle)?;
        dcb::default(&mut dcb);
        dcb::set_baud_rate(&mut dcb, builder.baudrate);
        dcb::set_data_bits(&mut dcb, builder.data_bits)?;
        dcb::set_parity(&mut dcb, builder.parity)?;
        dcb::set_stop_bits(&mut dcb, builder.stop_bits)?;
        dcb::set_flow_control(&mut dcb, builder.flow_control)?;
        dcb::set_dcb(handle, dcb)?;

        let r_event = unsafe { CreateEventW(null_mut(), 1, 0, null()) };
        let w_event = unsafe { CreateEventW(null_mut(), 1, 0, null()) };

        if r_event == 0 as HANDLE || w_event == 0 as HANDLE {
            unsafe {
                CloseHandle(r_event as *mut _);
                CloseHandle(w_event as *mut _);
            }
            return Err(Error::last_os_error().into());
        }

        let mut r_overlap: OVERLAPPED = unsafe { std::mem::zeroed() };
        let mut w_overlap: OVERLAPPED = unsafe { std::mem::zeroed() };

        r_overlap.hEvent = r_event;
        w_overlap.hEvent = w_event;

        Ok(COMPort {
            path: builder.path.to_owned(),
            handle: handle as HANDLE,
            r_overlap,
            w_overlap,
        })
    }

    fn escape_comm_function(&mut self, function: u32) -> Result<()> {
        match unsafe { EscapeCommFunction(self.handle, function) } {
            0 => Err(Error::last_os_error().into()),
            _ => Ok(()),
        }
    }

    fn read_pin(&mut self, pin: u32) -> Result<bool> {
        let mut status: u32 = 0;

        match unsafe { GetCommModemStatus(self.handle, &mut status) } {
            0 => Err(Error::last_os_error().into()),
            _ => Ok(status & pin != 0),
        }
    }
}

unsafe impl Send for COMPort {}

impl Drop for COMPort {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

impl AsRawHandle for COMPort {
    fn as_raw_handle(&self) -> RawHandle {
        self.handle as RawHandle
    }
}

impl IntoRawHandle for COMPort {
    fn into_raw_handle(self) -> RawHandle {
        let Self { handle, .. } = self;
        handle as RawHandle
    }
}

impl SerialPort for COMPort {
    fn name(&self) -> String {
        self.path.clone()
    }

    fn baudrate(&self) -> Result<u32> {
        let dcb = dcb::get_dcb(self.handle)?;
        Ok(dcb.BaudRate)
    }

    fn data_bits(&self) -> Result<DataBits> {
        let dcb = dcb::get_dcb(self.handle)?;
        match dcb.ByteSize {
            5 => Ok(DataBits::Five),
            6 => Ok(DataBits::Six),
            7 => Ok(DataBits::Seven),
            8 => Ok(DataBits::Eight),
            _ => Ok(DataBits::Unknown),
        }
    }

    fn flow_control(&self) -> Result<FlowControl> {
        let dcb = dcb::get_dcb(self.handle)?;
        if dcb.fOutxCtsFlow() || dcb.fRtsControl() != dcb::RtsControl::Disable {
            Ok(FlowControl::Hardware)
        } else if dcb.fOutX() || dcb.fInX() {
            Ok(FlowControl::Software)
        } else {
            Ok(FlowControl::None)
        }
    }

    fn parity(&self) -> Result<Parity> {
        let dcb = dcb::get_dcb(self.handle)?;
        match dcb.Parity {
            ODDPARITY => Ok(Parity::Odd),
            EVENPARITY => Ok(Parity::Even),
            NOPARITY => Ok(Parity::None),
            _ => Ok(Parity::Unknown),
        }
    }

    fn stop_bits(&self) -> Result<StopBits> {
        let dcb = dcb::get_dcb(self.handle)?;
        match dcb.StopBits {
            TWOSTOPBITS => Ok(StopBits::Two),
            ONESTOPBIT => Ok(StopBits::One),
            ONE5STOPBITS => Ok(StopBits::OnePointFive),
            _ => Ok(StopBits::Unknown),
        }
    }

    fn set_baud_rate(&mut self, baud_rate: u32) -> Result<()> {
        let mut dcb = dcb::get_dcb(self.handle)?;
        dcb::set_baud_rate(&mut dcb, baud_rate);
        dcb::set_dcb(self.handle, dcb)
    }

    fn set_data_bits(&mut self, data_bits: DataBits) -> Result<()> {
        let mut dcb = dcb::get_dcb(self.handle)?;
        dcb::set_data_bits(&mut dcb, data_bits)?;
        dcb::set_dcb(self.handle, dcb)
    }

    fn set_flow_control(&mut self, flow_control: FlowControl) -> Result<()> {
        let mut dcb = dcb::get_dcb(self.handle)?;
        dcb::set_flow_control(&mut dcb, flow_control)?;
        dcb::set_dcb(self.handle, dcb)
    }

    fn set_parity(&mut self, parity: Parity) -> Result<()> {
        let mut dcb = dcb::get_dcb(self.handle)?;
        dcb::set_parity(&mut dcb, parity)?;
        dcb::set_dcb(self.handle, dcb)
    }

    fn set_stop_bits(&mut self, stop_bits: StopBits) -> Result<()> {
        let mut dcb = dcb::get_dcb(self.handle)?;
        dcb::set_stop_bits(&mut dcb, stop_bits)?;
        dcb::set_dcb(self.handle, dcb)
    }

    fn set_rts(&mut self, level: bool) -> Result<()> {
        if level {
            self.escape_comm_function(SETRTS)
        } else {
            self.escape_comm_function(CLRRTS)
        }
    }

    fn set_dtr(&mut self, level: bool) -> Result<()> {
        if level {
            self.escape_comm_function(SETDTR)
        } else {
            self.escape_comm_function(CLRDTR)
        }
    }

    fn set_break(&mut self, level: bool) -> Result<()> {
        if level {
            if unsafe { SetCommBreak(self.handle) != 0 } {
                return Ok(());
            }
        } else {
            if unsafe { ClearCommBreak(self.handle) != 0 } {
                return Ok(());
            }
        }
        Err(Error::last_os_error().into())
    }

    fn cts(&mut self) -> Result<bool> {
        self.read_pin(MS_CTS_ON)
    }

    fn dsr(&mut self) -> Result<bool> {
        self.read_pin(MS_DSR_ON)
    }

    fn ri(&mut self) -> Result<bool> {
        self.read_pin(MS_RING_ON)
    }

    fn cd(&mut self) -> Result<bool> {
        self.read_pin(MS_RLSD_ON)
    }

    fn bytes_to_read(&self) -> Result<u32> {
        let mut errors: u32 = 0;
        let mut comstat = MaybeUninit::uninit();

        if unsafe { ClearCommError(self.handle, &mut errors, comstat.as_mut_ptr()) != 0 } {
            unsafe { Ok(comstat.assume_init().cbInQue) }
        } else {
            Err(Error::last_os_error().into())
        }
    }

    fn bytes_to_write(&self) -> Result<u32> {
        let mut errors: u32 = 0;
        let mut comstat = MaybeUninit::uninit();

        if unsafe { ClearCommError(self.handle, &mut errors, comstat.as_mut_ptr()) != 0 } {
            unsafe { Ok(comstat.assume_init().cbOutQue) }
        } else {
            Err(Error::last_os_error().into())
        }
    }

    fn clear(&self, buffer_to_clear: Clear) -> Result<()> {
        let buffer_flags = match buffer_to_clear {
            Clear::Input => PURGE_RXABORT | PURGE_RXCLEAR,
            Clear::Output => PURGE_TXABORT | PURGE_TXCLEAR,
            Clear::All => PURGE_RXABORT | PURGE_RXCLEAR | PURGE_TXABORT | PURGE_TXCLEAR,
        };

        if unsafe { PurgeComm(self.handle, buffer_flags) != 0 } {
            Ok(())
        } else {
            Err(Error::last_os_error().into())
        }
    }
}
