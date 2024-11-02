use anyhow::Result;
use windows::{
    core::{Interface, GUID, PCWSTR},
    Win32::{
        Foundation::{HMODULE, MAX_PATH},
        System::{
            Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
            LibraryLoader::GetModuleFileNameW,
        },
        UI::WindowsAndMessaging::{MessageBoxW, MB_OK},
    },
};

use crate::dll::DllModule;

pub trait GUIDExt {
    fn to_string(&self) -> String;
}

impl GUIDExt for GUID {
    fn to_string(&self) -> String {
        format!(
            "{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7],
        )
    }
}

pub fn co_create_inproc<T: Interface>(clsid: &GUID) -> Result<T> {
    Ok(unsafe { CoCreateInstance(clsid, None, CLSCTX_INPROC_SERVER)? })
}

pub fn to_wide(s: &str) -> Vec<u8> {
    let mut wide: Vec<u8> = s
        .encode_utf16()
        .flat_map(|c| c.to_le_bytes())
        .collect::<Vec<u8>>();
    wide.push(0);
    wide
}

pub fn to_wide_16(s: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    wide.push(0);
    wide
}

pub fn get_module_path() -> Result<String> {
    unsafe {
        // Get a handle to the current module
        let dll_instance = DllModule::global()
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to get DllModule"))?;
        let h_module: HMODULE = dll_instance.hinst;

        // Get the module file name
        let mut buffer: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
        let length = GetModuleFileNameW(h_module, &mut buffer);

        // Convert the wide string to a Rust string
        Ok(String::from_utf16_lossy(&buffer[..length as usize]))
    }
}

#[allow(dead_code)]
pub fn alert(message: &str) -> Result<()> {
    // with MessageBoxW
    let message_clone = message.to_string();
    std::thread::spawn(move || unsafe {
        let title = to_wide_16("Alert");
        let message = to_wide_16(&message_clone);
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK,
        );
    });
    Ok(())
}
