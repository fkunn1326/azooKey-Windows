use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows_core::PCWSTR;

use crate::extension::StringExt;

// const LOG_FOLDER: &str = "D:/azookey-windows/logs";

pub fn setup_logger() -> anyhow::Result<()> {
    // let timestamp = chrono::Local::now().format("%Y-%m-%d-%H.%M.%S");
    // let log_file = format!("{}/{}.log", LOG_FOLDER, timestamp);

    // std::fs::create_dir_all(LOG_FOLDER)?;
    // std::fs::File::create(&log_file)?;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .chain(fern::Output::call(|record| {
            let str = format!("{}", record.args());
            let wide: Vec<u16> = str.as_str().to_wide_16();
            unsafe { OutputDebugStringW(PCWSTR::from_raw(wide.as_ptr())) };
        }))
        // .chain(fern::log_file(&log_file)?)
        .apply()?;

    Ok(())
}
