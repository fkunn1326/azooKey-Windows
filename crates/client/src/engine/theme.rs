use windows::{
    core::w,
    Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, REG_VALUE_TYPE, RRF_RT_REG_DWORD},
};

use anyhow::Result;

pub fn get_theme() -> Result<bool> {
    // return true if the system uses light theme
    let mut value_type = REG_VALUE_TYPE::default();
    let mut data = [0u8; 4];
    let mut data_size = data.len() as u32;

    let _ = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            Some(&mut value_type),
            Some(data.as_mut_ptr().cast()),
            Some(&mut data_size),
        )
    };

    Ok(data[0] != 0)
}
