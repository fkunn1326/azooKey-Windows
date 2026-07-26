use std::collections::HashMap;

use crate::globals::{DllModule, GUID_DISPLAY_ATTRIBUTE};
use crate::tsf::compartment;

use super::text_service::TextService_Impl;
use windows::Win32::UI::TextServices::ITfCompartmentMgr;
use windows::{
    core::{Interface as _, GUID},
    Win32::{
        Foundation::BOOL,
        System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
        UI::TextServices::{
            CLSID_TF_CategoryMgr, ITfCategoryMgr, ITfCompartmentEventSink, ITfKeyEventSink,
            ITfKeystrokeMgr, ITfLangBarItemButton, ITfLangBarItemMgr, ITfSource,
            ITfTextInputProcessorEx_Impl, ITfTextInputProcessor_Impl, ITfThreadMgr,
            ITfThreadMgrEventSink,
        },
    },
};

use anyhow::Context;

impl ITfTextInputProcessor_Impl for TextService_Impl {
    #[macros::anyhow]
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        tracing::debug!("Activated with tid: {tid}");

        let mut dll_instance = DllModule::get()?;
        dll_instance.add_ref();

        self.tid.set(tid);
        let thread_mgr = ptim.context("Thread manager is null")?;
        self.thread_mgr.set(Some(thread_mgr.clone()));

        if let Ok(context) = self.get_active_context() {
            self.contexts.borrow_mut().register(&context);
        }

        // COM インターフェースを取得
        let this_key_event = self.this::<ITfKeyEventSink>()?;
        let this_event_sink = self.this::<ITfThreadMgrEventSink>()?;
        let this_lang_bar = self.this::<ITfLangBarItemButton>()?;

        // COM 登録
        tracing::debug!("AdviseKeyEventSink");
        unsafe {
            thread_mgr.cast::<ITfKeystrokeMgr>()?.AdviseKeyEventSink(
                tid,
                &this_key_event,
                BOOL::from(true),
            )?;
        }

        tracing::debug!("AdviseThreadMgrEventSink");
        let cookie = unsafe {
            thread_mgr
                .cast::<ITfSource>()?
                .AdviseSink(&ITfThreadMgrEventSink::IID, &this_event_sink)?
        };
        self.thread_mgr_event_sink_cookie.set(Some(cookie));

        tracing::debug!("AdviseTextLayoutSink");
        let doc_mgr = unsafe { thread_mgr.GetFocus() };
        if let Ok(doc_mgr) = doc_mgr {
            self.advise_text_layout_sink(doc_mgr)?;
        }

        tracing::debug!("Initialize display attribute");
        let atom_map = unsafe {
            let mut map = HashMap::new();
            let category_mgr: ITfCategoryMgr =
                CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;

            let atom = category_mgr.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE)?;
            map.insert(GUID_DISPLAY_ATTRIBUTE, atom);
            map
        };
        self.display_attribute_atom.set(atom_map);

        tracing::debug!("Initialize langbar");
        unsafe {
            thread_mgr
                .cast::<ITfLangBarItemMgr>()?
                .AddItem(&this_lang_bar)?;
        }

        // コンパートメントの初期化
        tracing::debug!("Initialize compartments");
        let compartment_mgr = thread_mgr.cast::<ITfCompartmentMgr>()?;
        let compartment_sink = self.this::<ITfCompartmentEventSink>()?;

        {
            let mut map = self.compartments.borrow_mut();
            for &guid in compartment::WATCHED_COMPARTMENTS {
                tracing::debug!("AdviseCompartmentEventSink: {:?}", guid);
                let entry = compartment::advise_compartment_sink(
                    &compartment_mgr,
                    &compartment_sink,
                    &guid,
                )?;
                map.insert(guid, entry);
            }
        }

        tracing::debug!("Activate success");

        Ok(())
    }

    #[macros::anyhow]
    fn Deactivate(&self) -> Result<()> {
        tracing::debug!("Deactivated");

        // remove reference to the dll instance
        let mut dll_instance = DllModule::get()?;
        dll_instance.release();

        let thread_mgr = self.thread_mgr();

        // remove key event sink
        tracing::debug!("UnadviseKeyEventSink");
        if let Ok(thread_mgr) = &thread_mgr {
            unsafe {
                thread_mgr
                    .cast::<ITfKeystrokeMgr>()?
                    .UnadviseKeyEventSink(self.tid.get())?;
            };

            tracing::debug!("Remove langbar");
            let this_lang_bar = self.this::<ITfLangBarItemButton>()?;
            unsafe {
                thread_mgr
                    .cast::<ITfLangBarItemMgr>()?
                    .RemoveItem(&this_lang_bar)
            }?;
        }

        // remove thread manager event sink
        tracing::debug!("UnadviseThreadMgrEventSink");
        if let Ok(thread_mgr) = &thread_mgr {
            if let Some(cookie) = self.thread_mgr_event_sink_cookie.take() {
                unsafe {
                    thread_mgr.cast::<ITfSource>()?.UnadviseSink(cookie)?;
                }
            }
        }

        // コンパートメントの監視を解除し、クリーンアップ
        tracing::debug!("UnadviseCompartmentEventSink");
        if let Ok(thread_mgr) = &thread_mgr {
            let entries: Vec<(GUID, compartment::CompartmentEntry)> =
                self.compartments.borrow_mut().drain().collect();
            let mgr = thread_mgr.cast::<ITfCompartmentMgr>().ok();

            for (guid, entry) in entries {
                compartment::unadvise_compartment_sink(&entry);
                if let Some(mgr) = &mgr {
                    let _ = compartment::clear_compartment(mgr, self.tid.get(), &guid);
                }
            }
        }

        // clear all contexts (Drop handles sink cleanup automatically)
        tracing::debug!("DropContexts");
        self.contexts.borrow_mut().clear();

        // clear display attribute
        self.display_attribute_atom.set(HashMap::new());

        self.tid.set(0);
        self.thread_mgr.set(None);

        tracing::debug!("Deactivate success");

        Ok(())
    }
}

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
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
