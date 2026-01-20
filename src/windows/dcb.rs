use std::io::Error;
use windows_sys::Win32::Devices::Communication::{
    DCB, EVENPARITY, GetCommState, NOPARITY, ODDPARITY, ONE5STOPBITS, ONESTOPBIT, SetCommState,
    TWOSTOPBITS,
};
use windows_sys::Win32::Foundation::HANDLE;

use crate::{DataBits, FlowControl, Parity, Result, StopBits};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DtrControl {
    Disable = 0x00,
    Enable = 0x01,
    Handshake = 0x02,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RtsControl {
    Disable = 0x00,
    Enable = 0x01,
    Handshake = 0x02,
    Toggle = 0x03,
}

#[allow(non_snake_case, dead_code)]
pub(crate) trait BitOperation {
    fn set_fBinary(&mut self, value: bool);
    fn set_fParity(&mut self, value: bool);
    fn set_fOutxCtsFlow(&mut self, value: bool);
    fn set_fOutxDsrFlow(&mut self, value: bool);
    fn set_fDtrControl(&mut self, value: DtrControl);
    fn set_fDsrSensitivity(&mut self, value: bool);
    fn set_fTXContinueOnXoff(&mut self, value: bool);
    fn set_fOutX(&mut self, value: bool);
    fn set_fInX(&mut self, value: bool);
    fn set_fErrorChar(&mut self, value: bool);
    fn set_fNull(&mut self, value: bool);
    fn set_fRtsControl(&mut self, value: RtsControl);
    fn set_fAbortOnError(&mut self, value: bool);

    fn fOutxCtsFlow(&self) -> bool;
    fn fRtsControl(&self) -> RtsControl;
    fn fOutX(&self) -> bool;
    fn fInX(&self) -> bool;
}

impl BitOperation for DCB {
    fn set_fBinary(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 0;
        } else {
            self._bitfield &= !(1 << 0);
        }
    }

    fn set_fParity(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 1;
        } else {
            self._bitfield &= !(1 << 1);
        }
    }

    fn set_fOutxCtsFlow(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 2;
        } else {
            self._bitfield &= !(1 << 2);
        }
    }

    fn set_fOutxDsrFlow(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 3;
        } else {
            self._bitfield &= !(1 << 3);
        }
    }

    fn set_fDtrControl(&mut self, value: DtrControl) {
        // Clear bits 4-5 and set new value
        self._bitfield &= !(0b11 << 4);
        self._bitfield |= ((value as u32) & 0b11) << 4;
    }

    fn set_fDsrSensitivity(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 6;
        } else {
            self._bitfield &= !(1 << 6);
        }
    }

    fn set_fTXContinueOnXoff(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 7;
        } else {
            self._bitfield &= !(1 << 7);
        }
    }

    fn set_fOutX(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 8;
        } else {
            self._bitfield &= !(1 << 8);
        }
    }

    fn set_fInX(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 9;
        } else {
            self._bitfield &= !(1 << 9);
        }
    }

    fn set_fErrorChar(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 10;
        } else {
            self._bitfield &= !(1 << 10);
        }
    }

    fn set_fNull(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 11;
        } else {
            self._bitfield &= !(1 << 11);
        }
    }

    fn set_fRtsControl(&mut self, value: RtsControl) {
        // Clear bits 12-13 and set new value
        self._bitfield &= !(0b11 << 12);
        self._bitfield |= ((value as u32) & 0b11) << 12;
    }

    fn set_fAbortOnError(&mut self, value: bool) {
        if value {
            self._bitfield |= 1 << 14;
        } else {
            self._bitfield &= !(1 << 14);
        }
    }

    fn fOutxCtsFlow(&self) -> bool {
        (self._bitfield & (1 << 2)) != 0
    }

    fn fRtsControl(&self) -> RtsControl {
        let bits = (self._bitfield >> 12) & 0b11;
        match bits {
            0 => RtsControl::Disable,
            1 => RtsControl::Enable,
            2 => RtsControl::Handshake,
            3 => RtsControl::Toggle,
            _ => unreachable!(),
        }
    }

    fn fOutX(&self) -> bool {
        (self._bitfield & (1 << 8)) != 0
    }

    fn fInX(&self) -> bool {
        (self._bitfield & (1 << 9)) != 0
    }
}

pub(crate) fn default(dcb: &mut DCB) {
    dcb.XonChar = 0x11;
    dcb.XoffChar = 0x13;
    dcb.ErrorChar = 0x00;
    dcb.EofChar = 0x1A;

    dcb.set_fBinary(true);
    dcb.set_fOutxDsrFlow(false);
    dcb.set_fDtrControl(DtrControl::Disable);
    dcb.set_fDsrSensitivity(false);
    dcb.set_fErrorChar(false);
    dcb.set_fNull(false);
    dcb.set_fAbortOnError(false);
}

pub(crate) fn get_dcb(handle: HANDLE) -> Result<DCB> {
    let mut dcb = DCB::default();
    dcb.DCBlength = std::mem::size_of::<DCB>() as u32;

    if unsafe { GetCommState(handle, &mut dcb) } != 0 {
        Ok(dcb)
    } else {
        Err(Error::last_os_error().into())
    }
}

pub(crate) fn set_dcb(handle: HANDLE, mut dcb: DCB) -> Result<()> {
    if unsafe { SetCommState(handle, &mut dcb as *mut _) != 0 } {
        Ok(())
    } else {
        Err(Error::last_os_error().into())
    }
}

pub(crate) fn set_baud_rate(dcb: &mut DCB, baud_rate: u32) {
    dcb.BaudRate = baud_rate;
}

pub(crate) fn set_data_bits(dcb: &mut DCB, data_bits: DataBits) -> Result<()> {
    dcb.ByteSize = match data_bits {
        DataBits::Five => 5,
        DataBits::Six => 6,
        DataBits::Seven => 7,
        DataBits::Eight => 8,
        _ => return Err(crate::Error::InvalidArgument("DataBits::Unknown".to_owned())),
    };
    Ok(())
}

pub(crate) fn set_parity(dcb: &mut DCB, parity: Parity) -> Result<()> {
    dcb.Parity = match parity {
        Parity::None => NOPARITY,
        Parity::Odd => ODDPARITY,
        Parity::Even => EVENPARITY,
        _ => return Err(crate::Error::InvalidArgument("Parity::Unknown".to_owned())),
    };

    dcb.set_fParity(parity != Parity::None);
    Ok(())
}

pub(crate) fn set_stop_bits(dcb: &mut DCB, stop_bits: StopBits) -> Result<()> {
    dcb.StopBits = match stop_bits {
        StopBits::One => ONESTOPBIT,
        StopBits::Two => TWOSTOPBITS,
        StopBits::OnePointFive => ONE5STOPBITS,
        _ => return Err(crate::Error::InvalidArgument("StopBits::Unknown".to_owned())),
    };
    Ok(())
}

pub(crate) fn set_flow_control(dcb: &mut DCB, flow_control: FlowControl) -> Result<()> {
    match flow_control {
        FlowControl::None => {
            dcb.set_fOutxCtsFlow(false);
            dcb.set_fRtsControl(RtsControl::Disable);
            dcb.set_fOutX(false);
            dcb.set_fInX(false);
        }
        FlowControl::Software => {
            dcb.set_fOutxCtsFlow(false);
            dcb.set_fRtsControl(RtsControl::Disable);
            dcb.set_fOutX(true);
            dcb.set_fInX(true);
        }
        FlowControl::Hardware => {
            dcb.set_fOutxCtsFlow(true);
            dcb.set_fRtsControl(RtsControl::Enable);
            dcb.set_fOutX(false);
            dcb.set_fInX(false);
        }
        _ => return Err(crate::Error::InvalidArgument("StopBits::Unknown".to_owned())),
    }
    Ok(())
}
