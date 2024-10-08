use windows::{
    core::{implement, AsImpl, IUnknown, Interface, Result, GUID},
    Win32::{
        Foundation::{BOOL, E_NOINTERFACE},
        System::Com::{IClassFactory, IClassFactory_Impl},
        UI::TextServices::ITfTextInputProcessor,
    },
};

use crate::{dll::DllModule, tsf::text_service::TextService};

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
    ) -> Result<()> {
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

        *ppvobject = unsafe { core::mem::transmute(text_service) };

        Ok(())
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        let mut dll_instance = DllModule::global().lock().unwrap();
        if flock.as_bool() {
            dll_instance.lock();
        } else {
            dll_instance.unlock();
        }
        Ok(())
    }
}
