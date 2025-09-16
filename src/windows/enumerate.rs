use std::ptr::{null, null_mut};

use windows_sys::{
    Win32::{
        Devices::DeviceAndDriverInstallation::{
            DICS_FLAG_GLOBAL, DIGCF_PRESENT, DIREG_DEV, SP_DEVINFO_DATA, SPDRP_FRIENDLYNAME,
            SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
            SetupDiGetDeviceRegistryPropertyW, SetupDiOpenDevRegKey,
        },
        Foundation::{GetLastError, INVALID_HANDLE_VALUE},
        System::Registry::{KEY_READ, RegCloseKey, RegQueryValueExW},
    },
    core::GUID,
};

use crate::{PortInfo, Result};

fn as_utf16(utf8: &str) -> Vec<u16> {
    utf8.encode_utf16().chain(Some(0)).collect()
}

fn from_utf16_lossy_trimmed(utf16: &[u16]) -> String {
    String::from_utf16_lossy(utf16)
        .trim_end_matches('\0')
        .to_owned()
}

const GUID_DEVCLASS_PORTS: GUID = GUID {
    data1: 0x4d36e978,
    data2: 0xe325,
    data3: 0x11ce,
    data4: [0xbf, 0xc1, 0x08, 0x00, 0x2b, 0xe1, 0x03, 0x18],
};

pub fn available_ports() -> Result<Vec<PortInfo>> {
    let mut infos = Vec::new();

    unsafe {
        let ports = SetupDiGetClassDevsW(&GUID_DEVCLASS_PORTS, null(), null_mut(), DIGCF_PRESENT);

        let mut info = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };

        let mut index = 0;
        loop {
            if SetupDiEnumDeviceInfo(ports, index, &mut info) == 0 {
                if GetLastError() != 0 {
                    break;
                }
            }

            let mut buffer = [0u16; 64];

            let mut _info = PortInfo::default();

            // Path
            let hkey = SetupDiOpenDevRegKey(ports, &info, DICS_FLAG_GLOBAL, 0, DIREG_DEV, KEY_READ);

            if hkey != INVALID_HANDLE_VALUE {
                let mut _size = (buffer.len() * 2) as u32;

                if RegQueryValueExW(
                    hkey,
                    as_utf16("PortName").as_ptr(),
                    null_mut(),
                    null_mut(),
                    buffer.as_mut_ptr() as *mut u8,
                    &mut _size,
                ) == 0
                {
                    _info.path = from_utf16_lossy_trimmed(&buffer);
                }
                RegCloseKey(hkey);
            }

            // Friendly name
            if SetupDiGetDeviceRegistryPropertyW(
                ports,
                &info,
                SPDRP_FRIENDLYNAME,
                null_mut(),
                buffer.as_mut_ptr() as *mut u8,
                (buffer.len() * 2) as u32,
                null_mut(),
            ) != 0
            {
                _info.name = from_utf16_lossy_trimmed(&buffer);
            }

            index += 1;
            infos.push(_info);
        }

        SetupDiDestroyDeviceInfoList(ports);
    }

    Ok(infos)
}
