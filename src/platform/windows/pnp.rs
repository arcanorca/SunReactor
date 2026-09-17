//! Windows SetupAPI device property resolution for monitor PnP devnodes.
//!
//! Uses SetupAPI (`SetupDiCreateDeviceInfoList`, `SetupDiOpenDeviceInterfaceW`,
//! `SetupDiGetDeviceInterfaceDetailW`, `SetupDiGetDevicePropertyW`) to extract:
//! - Container ID (`DEVPKEY_Device_ContainerId`)
//! - Device Instance ID (`DEVPKEY_Device_InstanceId`)
//! - Hardware IDs (`DEVPKEY_Device_HardwareIds`)

use std::ptr::{null, null_mut};
use windows_sys::core::GUID;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiCreateDeviceInfoList, SetupDiDestroyDeviceInfoList, SetupDiGetDeviceInterfaceDetailW,
    SetupDiGetDevicePropertyW, SetupDiOpenDeviceInterfaceW, HDEVINFO, SP_DEVICE_INTERFACE_DATA,
    SP_DEVINFO_DATA,
};
use windows_sys::Win32::Devices::Properties::{
    DEVPKEY_Device_ContainerId, DEVPKEY_Device_HardwareIds, DEVPKEY_Device_InstanceId,
    DEVPROP_TYPE_GUID, DEVPROP_TYPE_STRING, DEVPROP_TYPE_STRING_LIST,
};
use windows_sys::Win32::Foundation::DEVPROPKEY;

/// RAII wrapper around SetupAPI device information set `HDEVINFO`.
pub struct DeviceInfoSetGuard {
    handle: HDEVINFO,
}

impl DeviceInfoSetGuard {
    #[must_use]
    pub fn new() -> Option<Self> {
        // Safety justification:
        // - `SetupDiCreateDeviceInfoList(null, null_mut())` creates an empty device information set.
        let handle = unsafe { SetupDiCreateDeviceInfoList(null(), null_mut()) };
        if handle == -1 || handle == 0 {
            None
        } else {
            Some(Self { handle })
        }
    }

    #[must_use]
    pub fn handle(&self) -> HDEVINFO {
        self.handle
    }
}

impl Drop for DeviceInfoSetGuard {
    fn drop(&mut self) {
        if self.handle != -1 && self.handle != 0 {
            // Safety justification:
            // - `handle` is a valid `HDEVINFO` allocated by `SetupDiCreateDeviceInfoList`.
            unsafe {
                SetupDiDestroyDeviceInfoList(self.handle);
            }
        }
    }
}

/// Extracted PnP device properties for a monitor interface.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PnpDeviceInfo {
    /// Windows device Container ID (e.g. `{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}`).
    pub container_id: Option<String>,
    /// Windows PnP device instance ID (e.g. `DISPLAY\DEL4198\5&383f58e1&0&UID4357`).
    pub device_instance_id: Option<String>,
    /// Hardware IDs identifying the monitor model (e.g. `["MONITOR\DEL4198", "*DEL4198"]`).
    pub hardware_ids: Vec<String>,
}

/// Formats a raw Win32 `GUID` struct into standard `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}` string.
#[must_use]
pub fn format_guid(guid: &GUID) -> String {
    format!(
        "{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7],
    )
}

/// Checks whether a Container ID is a generic system or nil container rather than a physical external device.
#[must_use]
pub fn is_generic_or_system_container(guid_str: &str) -> bool {
    let lower = guid_str.to_ascii_lowercase();
    lower == "{00000000-0000-0000-0000-000000000000}"
        || lower == "{00000000-0000-0000-ffff-ffffffffffff}"
}

/// Queries PnP device properties for a monitor device interface path.
#[must_use]
pub fn query_pnp_device_info(monitor_device_path: &str) -> PnpDeviceInfo {
    let mut info = PnpDeviceInfo::default();

    let Some(guard) = DeviceInfoSetGuard::new() else {
        return info;
    };

    let wide_path: Vec<u16> = monitor_device_path
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut interface_data = SP_DEVICE_INTERFACE_DATA {
        cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
        ..Default::default()
    };

    // Safety justification:
    // - `guard.handle()` is a valid HDEVINFO.
    // - `wide_path` is null-terminated.
    // - `interface_data` cbSize is set correctly.
    let ok = unsafe {
        SetupDiOpenDeviceInterfaceW(
            guard.handle(),
            wide_path.as_ptr(),
            0,
            &raw mut interface_data,
        )
    };

    if ok == 0 {
        return info;
    }

    let mut devinfo_data = SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };

    // Safety justification:
    // - When `DeviceInterfaceDetailData` is NULL and size is 0, SetupDiGetDeviceInterfaceDetailW
    //   returns the corresponding `SP_DEVINFO_DATA` for the interface.
    let _ = unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            guard.handle(),
            &raw const interface_data,
            null_mut(),
            0,
            null_mut(),
            &raw mut devinfo_data,
        )
    };

    // 1. Query Container ID (DEVPKEY_Device_ContainerId)
    let key_container_id = DEVPKEY_Device_ContainerId;
    if let Some(container_guid) =
        unsafe { query_guid_property(guard.handle(), &devinfo_data, &raw const key_container_id) }
    {
        let guid_str = format_guid(&container_guid);
        if !is_generic_or_system_container(&guid_str) {
            info.container_id = Some(guid_str);
        }
    }

    // 2. Query Device Instance ID (DEVPKEY_Device_InstanceId)
    let key_instance_id = DEVPKEY_Device_InstanceId;
    if let Some(instance_id) =
        unsafe { query_string_property(guard.handle(), &devinfo_data, &raw const key_instance_id) }
    {
        info.device_instance_id = Some(instance_id);
    }

    // 3. Query Hardware IDs (DEVPKEY_Device_HardwareIds)
    let key_hardware_ids = DEVPKEY_Device_HardwareIds;
    if let Some(hw_ids) = unsafe {
        query_string_list_property(guard.handle(), &devinfo_data, &raw const key_hardware_ids)
    } {
        info.hardware_ids = hw_ids;
    }

    info
}

unsafe fn query_guid_property(
    hdevinfo: HDEVINFO,
    devinfo: &SP_DEVINFO_DATA,
    key: *const DEVPROPKEY,
) -> Option<GUID> {
    let mut prop_type: u32 = 0;
    let mut guid = GUID {
        data1: 0,
        data2: 0,
        data3: 0,
        data4: [0; 8],
    };
    let mut req_size: u32 = 0;

    let ok = SetupDiGetDevicePropertyW(
        hdevinfo,
        devinfo,
        key,
        &raw mut prop_type,
        (&raw mut guid).cast(),
        std::mem::size_of::<GUID>() as u32,
        &raw mut req_size,
        0,
    );

    if ok != 0 && prop_type == DEVPROP_TYPE_GUID {
        Some(guid)
    } else {
        None
    }
}

unsafe fn query_string_property(
    hdevinfo: HDEVINFO,
    devinfo: &SP_DEVINFO_DATA,
    key: *const DEVPROPKEY,
) -> Option<String> {
    let mut prop_type: u32 = 0;
    let mut req_size: u32 = 0;

    let _ = SetupDiGetDevicePropertyW(
        hdevinfo,
        devinfo,
        key,
        &raw mut prop_type,
        null_mut(),
        0,
        &raw mut req_size,
        0,
    );

    if req_size == 0 {
        return None;
    }

    let mut buf = vec![0u16; (req_size as usize).div_ceil(2)];
    let ok = SetupDiGetDevicePropertyW(
        hdevinfo,
        devinfo,
        key,
        &raw mut prop_type,
        buf.as_mut_ptr().cast(),
        req_size,
        &raw mut req_size,
        0,
    );

    if ok != 0 && prop_type == DEVPROP_TYPE_STRING {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    } else {
        None
    }
}

unsafe fn query_string_list_property(
    hdevinfo: HDEVINFO,
    devinfo: &SP_DEVINFO_DATA,
    key: *const DEVPROPKEY,
) -> Option<Vec<String>> {
    let mut prop_type: u32 = 0;
    let mut req_size: u32 = 0;

    let _ = SetupDiGetDevicePropertyW(
        hdevinfo,
        devinfo,
        key,
        &raw mut prop_type,
        null_mut(),
        0,
        &raw mut req_size,
        0,
    );

    if req_size == 0 {
        return None;
    }

    let mut buf = vec![0u16; (req_size as usize).div_ceil(2)];
    let ok = SetupDiGetDevicePropertyW(
        hdevinfo,
        devinfo,
        key,
        &raw mut prop_type,
        buf.as_mut_ptr().cast(),
        req_size,
        &raw mut req_size,
        0,
    );

    if ok != 0 && prop_type == DEVPROP_TYPE_STRING_LIST {
        let mut list = Vec::new();
        let mut start = 0;

        for (i, &c) in buf.iter().enumerate() {
            if c == 0 {
                if i > start {
                    let s = String::from_utf16_lossy(&buf[start..i]);
                    if !s.is_empty() {
                        list.push(s);
                    }
                }
                start = i + 1;
            }
        }
        Some(list)
    } else {
        None
    }
}
