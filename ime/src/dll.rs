use std::ffi::c_void;

use anyhow::Ok;
use windows::core::{Interface, GUID, HRESULT};
use windows::Win32::Foundation::{BOOL, CLASS_E_CLASSNOTAVAILABLE, E_UNEXPECTED, HMODULE, S_FALSE};
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

use crate::check_err;
use crate::factory::IMEClassFactory;
use crate::register::*;
use crate::utils::error::set_panic_hook;
use crate::utils::globals::GUID_TEXT_SERVICE;
use crate::utils::winutils::get_module_path;

use std::sync::{Mutex, OnceLock};

static DLL_INSTANCE: OnceLock<Mutex<DllModule>> = OnceLock::new();

unsafe impl Sync for DllModule {}
unsafe impl Send for DllModule {}

pub struct DllModule {
    ref_count: u32,
    ref_lock: u32,
    pub hinst: HMODULE,
}

impl DllModule {
    pub fn global() -> &'static Mutex<DllModule> {
        DLL_INSTANCE.get().expect("DllModule is not initialized")
    }

    pub fn new() -> Self {
        Self {
            ref_count: 0,
            ref_lock: 0,
            hinst: HMODULE::default(),
        }
    }

    pub fn lock(&mut self) -> u32 {
        self.ref_lock += 1;
        self.ref_lock
    }

    pub fn unlock(&mut self) -> u32 {
        self.ref_lock -= 1;
        self.ref_lock
    }

    pub fn can_unload(&self) -> bool {
        self.ref_count == 0 && self.ref_lock == 0
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn DllMain(
    hinst: HMODULE,
    fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> BOOL {
    if fdw_reason == DLL_PROCESS_ATTACH {
        let mut dll_instance = DllModule::new();
        dll_instance.hinst = hinst;
        let _ = DLL_INSTANCE.set(Mutex::new(dll_instance));
    }

    BOOL::from(true)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    let result: anyhow::Result<()> = (|| {
        let dll_instance = DllModule::global()
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to get DllModule"))?;
        if dll_instance.can_unload() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(S_FALSE))
        }
    })();

    check_err!(result)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    let result: anyhow::Result<()> = (|| {
        let rclsid = &unsafe { *rclsid };
        let riid = &unsafe { *riid };
        let ppv = unsafe { &mut *ppv };

        *ppv = std::ptr::null_mut();

        if *rclsid != GUID_TEXT_SERVICE {
            return Err(anyhow::anyhow!(CLASS_E_CLASSNOTAVAILABLE));
        }

        if *riid != IClassFactory::IID {
            return Err(anyhow::anyhow!(E_UNEXPECTED));
        }

        let factory: IMEClassFactory = IMEClassFactory::new();
        let factory: IClassFactory = factory.into();

        *ppv = unsafe {
            std::mem::transmute::<windows::Win32::System::Com::IClassFactory, *mut std::ffi::c_void>(
                factory,
            )
        };

        Ok(())
    })();

    check_err!(result)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    let result: anyhow::Result<()> = (|| {
        set_panic_hook()?;

        ProfileMgr::register(get_module_path()?.as_str())?;
        ClsidMgr::register(get_module_path()?.as_str())?;
        CategiryMgr::register()?;

        Ok(())
    })();

    check_err!(result)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    let result: anyhow::Result<()> = (|| {
        ProfileMgr::unregister()?;
        ClsidMgr::unregister()?;
        CategiryMgr::unregister()?;

        Ok(())
    })();

    check_err!(result)
}
