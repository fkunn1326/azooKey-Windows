use windows::{
    core::{implement, AsImpl, IUnknown, Interface, GUID},
    Win32::{
        Foundation::{BOOL, E_NOINTERFACE},
        System::Com::{IClassFactory, IClassFactory_Impl},
        UI::TextServices::ITfTextInputProcessor,
    },
};

use crate::{dll::DllModule, handle_result, tsf::text_service::TextService};

#[implement(IClassFactory)]
pub struct IMEClassFactory;

impl IMEClassFactory {
    pub fn new() -> Self {
        IMEClassFactory
    }
}

impl IClassFactory_Impl for IMEClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut std::ffi::c_void,
    ) -> windows::core::Result<()> {
        let riid = &unsafe { *riid };
        let ppvobject = unsafe { &mut *ppvobject };

        *ppvobject = std::ptr::null_mut();

        if punkouter.is_some() {
            return Err(E_NOINTERFACE.into());
        }

        if *riid != ITfTextInputProcessor::IID && *riid != IUnknown::IID {
            return Err(E_NOINTERFACE.into());
        }

        let text_service: ITfTextInputProcessor = TextService::new().into();

        let it: &TextService = unsafe { text_service.as_impl() };
        it.set_this(text_service.clone());

        *ppvobject = unsafe {
            core::mem::transmute::<
                windows::Win32::UI::TextServices::ITfTextInputProcessor,
                *mut std::ffi::c_void,
            >(text_service)
        };

        Ok(())
    }

    fn LockServer(&self, flock: BOOL) -> windows::core::Result<()> {
        let result: anyhow::Result<()> = (|| {
            let mut dll_instance = DllModule::global()
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to get DllModule"))?;
            if flock.as_bool() {
                dll_instance.lock();
            } else {
                dll_instance.unlock();
            }

            Ok(())
        })();

        handle_result!(result)
    }
}
