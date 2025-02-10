use std::collections::HashMap;

use crate::{
    engine::{ipc_service, state::IMEState},
    globals::{DllModule, GUID_DISPLAY_ATTRIBUTE},
};

use super::factory::TextServiceFactory_Impl;
use windows::{
    core::Interface as _,
    Win32::{
        Foundation::BOOL,
        System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
        UI::TextServices::{
            CLSID_TF_CategoryMgr, ITfCategoryMgr, ITfKeyEventSink, ITfKeystrokeMgr,
            ITfLangBarItemButton, ITfLangBarItemMgr, ITfSource, ITfTextInputProcessorEx_Impl,
            ITfTextInputProcessor_Impl, ITfThreadMgr, ITfThreadMgrEventSink,
        },
    },
};

use anyhow::{Context, Result};

impl ITfTextInputProcessor_Impl for TextServiceFactory_Impl {
    #[macros::anyhow]
    #[tracing::instrument]
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        tracing::debug!("Activated with tid: {tid}");

        // add reference to the dll instance to prevent it from being unloaded
        let mut dll_instance = DllModule::get()?;
        dll_instance.add_ref();

        // initialize ipc_service
        if let Ok(mut ipc_service) = ipc_service::IPCService::new() {
            ipc_service.append_text("".to_string())?;
            IMEState::get()?.ipc_service = Some(ipc_service);
        } else {
            // Activate() should not return an error
            // if Activate() returns an error, the icon of the previously activated TextService will be displayed, which may confuse the user
            tracing::error!("Failed to initialize IPC service");
            return Ok(());
        }

        let mut text_service = self.borrow_mut()?;

        text_service.tid = tid;
        let thread_mgr = ptim.context("Thread manager is null")?;
        text_service.thread_mgr = Some(thread_mgr.clone());

        // initialize key event sink
        tracing::debug!("AdviseKeyEventSink");

        unsafe {
            thread_mgr.cast::<ITfKeystrokeMgr>()?.AdviseKeyEventSink(
                tid,
                &text_service.this::<ITfKeyEventSink>()?,
                BOOL::from(true),
            )?;
        };

        // initialize thread manager event sink
        tracing::debug!("AdviseThreadMgrEventSink");
        unsafe {
            let cookie = thread_mgr.cast::<ITfSource>()?.AdviseSink(
                &ITfThreadMgrEventSink::IID,
                &text_service.this::<ITfThreadMgrEventSink>()?,
            )?;
            IMEState::get()?
                .cookies
                .insert(ITfThreadMgrEventSink::IID, cookie);
        };

        // initialize text layout sink
        tracing::debug!("AdviseTextLayoutSink");
        let doc_mgr = unsafe { thread_mgr.GetFocus() };
        if let Ok(doc_mgr) = doc_mgr {
            text_service.advise_text_layout_sink(doc_mgr)?;
        }

        // initialize display attribute
        tracing::debug!("Initialize display attribute");
        let atom_map = unsafe {
            let mut map = HashMap::new();
            let category_mgr: ITfCategoryMgr =
                CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;

            let atom = category_mgr.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE)?;
            map.insert(GUID_DISPLAY_ATTRIBUTE, atom);
            map
        };

        text_service.display_attribute_atom = atom_map;

        // initialize langbar
        tracing::debug!("Initialize langbar");
        unsafe {
            thread_mgr
                .cast::<ITfLangBarItemMgr>()?
                .AddItem(&text_service.this::<ITfLangBarItemButton>()?)?;
        };

        tracing::debug!("Activate success");

        Ok(())
    }

    #[macros::anyhow]
    #[tracing::instrument]
    fn Deactivate(&self) -> Result<()> {
        tracing::debug!("Deactivated");

        // remove reference to the dll instance
        let mut dll_instance = DllModule::get()?;
        dll_instance.release();

        {
            let text_service = self.borrow()?;
            let thread_mgr = text_service.thread_mgr()?;

            // end composition
            self.end_composition()?;

            // remove key event sink
            tracing::debug!("UnadviseKeyEventSink");
            unsafe {
                thread_mgr
                    .cast::<ITfKeystrokeMgr>()?
                    .UnadviseKeyEventSink(text_service.tid)?;
            };

            tracing::debug!("Remove langbar");
            unsafe {
                thread_mgr
                    .cast::<ITfLangBarItemMgr>()?
                    .RemoveItem(&text_service.this::<ITfLangBarItemButton>()?)
            }?;
        }

        let mut text_service = self.borrow_mut()?;
        let thread_mgr = text_service.thread_mgr()?;

        // remove thread manager event sink
        tracing::debug!("UnadviseThreadMgrEventSink");
        unsafe {
            if let Some(cookie) = IMEState::get()?.cookies.remove(&ITfThreadMgrEventSink::IID) {
                thread_mgr.cast::<ITfSource>()?.UnadviseSink(cookie)?;
            }
        };

        // remove text layout sink
        tracing::debug!("UnadviseTextLayoutSink");
        text_service.unadvise_text_layout_sink()?;

        // clear display attribute
        text_service.display_attribute_atom.clear();

        text_service.tid = 0;
        text_service.thread_mgr = None;

        tracing::debug!("Deactivate success");

        Ok(())
    }
}

impl ITfTextInputProcessorEx_Impl for TextServiceFactory_Impl {
    #[macros::anyhow]
    fn ActivateEx(&self, ptim: Option<&ITfThreadMgr>, tid: u32, _dwflags: u32) -> Result<()> {
        // called when the text service is activated
        // if this function is implemented, the Activate() function won't be called
        // so we need to call the Activate function manually
        tracing::debug!("Activated(Ex) with tid: {tid}");
        self.Activate(ptim, tid)?;
        Ok(())
    }
}
