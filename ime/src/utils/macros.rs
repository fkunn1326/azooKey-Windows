#[macro_export]
macro_rules! check_win32 {
    ($result:ident) => {
        if $result.is_ok() {
            return Ok(());
        } else {
            return Err(windows::core::Error::from($result.to_hresult()));
        }
    };
    ($result:ident, $value:ident) => {
        if $result.is_ok() {
            Ok($value)
        } else {
            Err(windows::core::Error::from($result.to_hresult()))
        }
    };
}

#[macro_export]
macro_rules! check_err {
    ($result:ident) => {
        if $result.is_ok() {
            windows::Win32::Foundation::S_OK
        } else {
            windows::Win32::Foundation::S_FALSE
        }
    };
}

#[macro_export]
macro_rules! handle_result {
    ($result:ident) => {
        match $result {
            Ok(v) => Ok(v),
            Err(e) => {
                let _ = $crate::utils::winutils::alert(&format!("Error: {:?}", e));
                Err(windows::core::Error::from(
                    windows::Win32::Foundation::E_FAIL,
                ))
            }
        }
    };
}
