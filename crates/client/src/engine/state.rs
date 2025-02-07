use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, MutexGuard},
};

use windows::{core::GUID, Win32::UI::TextServices::ITfContext};

use super::{input_mode::InputMode, ipc_service::IPCService};

#[derive(Debug)]
pub struct IMEState {
    pub ipc_service: Option<IPCService>,
    pub input_mode: InputMode,
    pub cookies: HashMap<GUID, u32>,
    pub context: Option<ITfContext>,
}

pub static IME_STATE: LazyLock<Mutex<IMEState>> = LazyLock::new(|| {
    tracing::debug!("Creating IMEState");
    Mutex::new(IMEState {
        ipc_service: None,
        input_mode: InputMode::default(),
        cookies: HashMap::new(),
        context: None,
    })
});
unsafe impl Sync for IMEState {}
unsafe impl Send for IMEState {}

impl IMEState {
    pub fn get() -> anyhow::Result<MutexGuard<'static, IMEState>> {
        match IME_STATE.try_lock() {
            Ok(guard) => Ok(guard),
            Err(e) => anyhow::bail!("Failed to lock state: {:?}", e),
        }
    }
}
