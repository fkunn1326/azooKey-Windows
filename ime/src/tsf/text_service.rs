use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::thread;

use anyhow::{Context, Result};

use windows::core::{implement, AsImpl, Interface};
use windows::Win32::Foundation::{BOOL, E_FAIL};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, IEnumTfDisplayAttributeInfo, ITfCategoryMgr, ITfCompositionSink,
    ITfCompositionSink_Impl, ITfDisplayAttributeInfo, ITfDisplayAttributeProvider,
    ITfDisplayAttributeProvider_Impl, ITfKeyEventSink, ITfKeystrokeMgr, ITfLangBarItemButton,
    ITfSource, ITfTextInputProcessor, ITfTextInputProcessor_Impl, ITfThreadMgr,
    ITfThreadMgrEventSink,
};

use crate::handle_result;
use crate::ui::{CandidateList, UiEvent};
use crate::utils::globals::{
    GUID_DISPLAY_ATTRIBUTE_CONVERTED, GUID_DISPLAY_ATTRIBUTE_FOCUSED, GUID_DISPLAY_ATTRIBUTE_INPUT,
};
use crate::utils::winutils::co_create_inproc;
use ipc::socket::SocketManager;

use super::composition_mgr::CompositionMgr;
use super::display_attribute;
use super::key_event_sink::KeyEventSink;
use super::language_bar::LanguageBar;
use super::thread_mgr_event_sink::ThreadMgrEventSink;

// すべてを取りまとめるメインのクラス
// Activate()とDeactivate()を実装しておけばいい
#[implement(ITfTextInputProcessor, ITfCompositionSink, ITfDisplayAttributeProvider)]
pub struct TextService {
    this: RefCell<Option<ITfTextInputProcessor>>,
    client_id: RefCell<u32>,
    // thread manager
    thread_mgr: RefCell<Option<ITfThreadMgr>>,
    thread_mgr_event_sink: RefCell<Option<ITfThreadMgrEventSink>>,
    thread_mgr_event_sink_cookie: RefCell<u32>,
    // category manager
    category_mgr: RefCell<Option<ITfCategoryMgr>>,
    // language bar
    language_bar: RefCell<Option<ITfLangBarItemButton>>,
    // key event sink
    key_event_sink: RefCell<Option<ITfKeyEventSink>>,
    // display attribute
    display_attribute_atom: RefCell<HashMap<&'static str, u32>>,
    // composition manager
    composition_mgr: RefCell<Option<CompositionMgr>>,
    // socket manager
    socket_mgr: RefCell<Option<SocketManager>>,
    // ui proxy
    ui_proxy: RefCell<Option<Sender<UiEvent>>>,
}

impl TextService {
    pub fn new() -> Self {
        TextService {
            this: RefCell::new(None),
            client_id: RefCell::new(0),
            thread_mgr: RefCell::new(None),
            thread_mgr_event_sink: RefCell::new(None),
            thread_mgr_event_sink_cookie: RefCell::new(0),
            category_mgr: RefCell::new(None),
            language_bar: RefCell::new(None),
            key_event_sink: RefCell::new(None),
            display_attribute_atom: RefCell::new(HashMap::new()),
            composition_mgr: RefCell::new(None),
            socket_mgr: RefCell::new(None),
            ui_proxy: RefCell::new(None),
        }
    }

    pub fn set_this(&self, this: ITfTextInputProcessor) {
        self.this.replace(Some(this));
    }

    // activate()
    fn activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        if let Some(ptim) = ptim {
            self.thread_mgr.replace(Some(ptim.clone()));
        }

        self.category_mgr
            .replace(Some(co_create_inproc::<ITfCategoryMgr>(
                &CLSID_TF_CategoryMgr,
            )?));
        self.client_id.replace(tid);

        self.activate_language_bar()?;
        self.activate_display_attribute()?;
        self.activate_socket()?;

        let (tx, rx) = std::sync::mpsc::channel();

        thread::spawn(|| {
            let _ = CandidateList::create(rx);
        });

        self.ui_proxy.replace(Some(tx));

        self.activate_composition_mgr()?;
        self.activate_thread_mgr_event_sink()?;
        self.activate_key_event_sink()?;

        Ok(())
    }

    // deactivate()
    fn deactivate(&self) -> Result<()> {
        self.deactivate_thread_mgr_event_sink()?;
        self.deactivate_language_bar()?;
        self.deactivate_display_attribute()?;
        self.deactivate_composition_mgr()?;
        self.deactivate_key_event_sink()?;
        self.deactivate_socket()?;
        Ok(())
    }

    // ThreadMgrEventSink
    fn activate_thread_mgr_event_sink(&self) -> Result<()> {
        let composition_mgr = self
            .composition_mgr
            .borrow()
            .clone()
            .context("failed to get composition_mgr")?;
        let socket_mgr = self
            .socket_mgr
            .borrow()
            .clone()
            .context("failed to get socket_mgr")?;

        let sink: ITfThreadMgrEventSink =
            ThreadMgrEventSink::new(composition_mgr.clone(), socket_mgr.clone()).into();
        let source: ITfSource = self
            .thread_mgr
            .borrow()
            .clone()
            .context("failed to get thread_mgr")?
            .cast()?;

        let cookie = unsafe { source.AdviseSink(&ITfThreadMgrEventSink::IID, &sink) }?;

        self.thread_mgr_event_sink_cookie.replace(cookie);
        self.thread_mgr_event_sink
            .borrow_mut()
            .replace(ThreadMgrEventSink::new(composition_mgr, socket_mgr).into());

        Ok(())
    }

    fn deactivate_thread_mgr_event_sink(&self) -> Result<()> {
        let source: ITfSource = self
            .thread_mgr
            .borrow()
            .clone()
            .context("failed to get thread_mgr")?
            .cast()?;
        let cookie = *self.thread_mgr_event_sink_cookie.borrow();
        unsafe {
            source.UnadviseSink(cookie)?;
        }
        Ok(())
    }

    // language bar ("あ"とか"A"とかのやつ)
    fn activate_language_bar(&self) -> Result<()> {
        let language_bar = LanguageBar::new(
            self.thread_mgr
                .borrow()
                .clone()
                .context("failed to get thread_mgr")?,
        )?;
        self.language_bar.replace(Some(language_bar));

        Ok(())
    }

    fn deactivate_language_bar(&self) -> Result<()> {
        let item = self
            .language_bar
            .borrow()
            .clone()
            .context("failed to get language_bar")?;
        let language_bar = unsafe { item.as_impl() };
        language_bar.deactivate(item.clone())?;

        self.language_bar.replace(None);

        Ok(())
    }

    // Display attribute (表示属性、下線入れたり色変えたり)
    fn activate_display_attribute(&self) -> Result<()> {
        let category_mgr = self
            .category_mgr
            .borrow()
            .clone()
            .context("failed to get category_mgr")?;
        let mut atom_map = HashMap::new();

        unsafe {
            let input_atom = category_mgr.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE_INPUT)?;
            let focused_atom = category_mgr.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE_FOCUSED)?;
            let converted_atom = category_mgr.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE_CONVERTED)?;

            atom_map.insert("input", input_atom);
            atom_map.insert("focused", focused_atom);
            atom_map.insert("converted", converted_atom);
        }

        self.display_attribute_atom.replace(atom_map);

        Ok(())
    }

    fn deactivate_display_attribute(&self) -> Result<()> {
        self.display_attribute_atom.borrow_mut().clear();
        Ok(())
    }

    fn activate_composition_mgr(&self) -> Result<()> {
        let client_id = *self.client_id.borrow();

        let this: ITfTextInputProcessor =
            self.this.borrow().clone().context("failed to get this")?;
        let sink: ITfCompositionSink = this.cast()?;

        let display_attribute_atom = self.display_attribute_atom.borrow().clone();
        let display_attribute = display_attribute_atom
            .get("focused")
            .context("failed to get focused")?;

        let composition_mgr = CompositionMgr::new(client_id, sink, *display_attribute);
        self.composition_mgr.replace(Some(composition_mgr));

        Ok(())
    }

    fn deactivate_composition_mgr(&self) -> Result<()> {
        let composition_mgr = self
            .composition_mgr
            .borrow_mut()
            .take()
            .context("failed to get composition_mgr")?;
        composition_mgr.end_composition()?;
        Ok(())
    }

    // Key event sink (キーボードイベント関連)
    fn activate_key_event_sink(&self) -> Result<()> {
        let sink: ITfKeyEventSink = KeyEventSink::new(
            self.composition_mgr
                .borrow()
                .clone()
                .context("failed to get composition_mgr")?,
            self.socket_mgr
                .borrow()
                .clone()
                .context("failed to get socket_mgr")?,
            self.ui_proxy
                .borrow()
                .clone()
                .context("failed to get ui_proxy")?,
        )
        .into();

        let source: ITfKeystrokeMgr = self
            .thread_mgr
            .borrow()
            .clone()
            .context("failed to get thread_mgr")?
            .cast()?;

        unsafe {
            source.AdviseKeyEventSink(*self.client_id.borrow(), &sink, BOOL::from(true))?;
        }

        self.key_event_sink.borrow_mut().replace(sink);

        Ok(())
    }

    fn deactivate_key_event_sink(&self) -> Result<()> {
        let source: ITfKeystrokeMgr = self
            .thread_mgr
            .borrow()
            .clone()
            .context("failed to get thread_mgr")?
            .cast()?;
        unsafe {
            source.UnadviseKeyEventSink(*self.client_id.borrow())?;
        }

        Ok(())
    }

    fn activate_socket(&self) -> Result<()> {
        let socket_mgr = SocketManager::new()?;
        self.socket_mgr.replace(Some(socket_mgr));
        Ok(())
    }

    fn deactivate_socket(&self) -> Result<()> {
        Ok(())
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> windows::core::Result<()> {
        let result = self.activate(ptim, tid);
        handle_result!(result)
    }

    fn Deactivate(&self) -> windows::core::Result<()> {
        let result = self.deactivate();
        handle_result!(result)
    }
}

impl ITfCompositionSink_Impl for TextService_Impl {
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _pcomposition: Option<&windows::Win32::UI::TextServices::ITfComposition>,
    ) -> windows_core::Result<()> {
        Ok(())
    }
}

impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> windows::core::Result<IEnumTfDisplayAttributeInfo> {
        let enum_info = display_attribute::EnumDisplayAttributeInfo::new();
        Ok(enum_info.into())
    }

    fn GetDisplayAttributeInfo(
        &self,
        guid: *const windows_core::GUID,
    ) -> windows::core::Result<ITfDisplayAttributeInfo> {
        let guid = unsafe { *guid };
        let attributes = display_attribute::EnumDisplayAttributeInfo::new();
        for attribute in attributes.attributes {
            if attribute.guid == guid {
                return Ok(attribute.into());
            }
        }
        Err(E_FAIL.into())
    }
}
