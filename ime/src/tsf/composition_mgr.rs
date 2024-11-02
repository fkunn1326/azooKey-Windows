use std::{cell::RefCell, rc::Rc};

use anyhow::Context;
use windows::core::{IUnknown, Interface, VARIANT};
use windows::Win32::Foundation::{BOOL, RECT};
use windows::Win32::UI::TextServices::{
    ITfCompartmentMgr, ITfComposition, ITfCompositionSink, ITfContext, ITfContextComposition,
    ITfDocumentMgr, ITfInsertAtSelection, ITfRange, GUID_COMPARTMENT_TRANSITORYEXTENSION_PARENT,
    GUID_PROP_ATTRIBUTE, TF_ANCHOR_START, TF_DEFAULT_SELECTION, TF_HALTCOND, TF_HF_OBJECT,
    TF_IAS_QUERYONLY, TF_SELECTION, TF_TF_MOVESTART,
};

use std::mem::ManuallyDrop;

use crate::ui::LocateEvent;
use crate::utils::winutils::to_wide_16;

use super::edit_session::EditSession;

#[derive(Clone)]
pub struct CompositionMgr {
    pub composition: Rc<RefCell<Option<ITfComposition>>>,
    context: Rc<RefCell<Option<ITfContext>>>,
    sink: ITfCompositionSink,
    client_id: u32,
    display_attribute: u32,
    pub preedit: RefCell<String>,
}

impl CompositionMgr {
    pub fn new(client_id: u32, sink: ITfCompositionSink, display_attribute: u32) -> Self {
        CompositionMgr {
            composition: Rc::new(RefCell::new(None)),
            context: Rc::new(RefCell::new(None)),
            sink,
            client_id,
            display_attribute,
            preedit: RefCell::new(String::new()),
        }
    }

    pub fn start_composition(&self, context: ITfContext) -> anyhow::Result<()> {
        let insert: ITfInsertAtSelection = context.cast()?;
        let context_composition: ITfContextComposition = context.cast()?;

        self.context.replace(Some(context.clone()));

        EditSession::handle(
            self.client_id,
            context,
            Rc::new({
                let composition_clone = Rc::clone(&self.composition);
                let sink = self.sink.clone();
                move |cookie| unsafe {
                    let range = insert.InsertTextAtSelection(cookie, TF_IAS_QUERYONLY, &[])?;
                    let new_composition =
                        context_composition.StartComposition(cookie, &range, &sink)?;
                    *composition_clone.borrow_mut() = Some(new_composition);
                    Ok(())
                }
            }),
        )?;

        Ok(())
    }

    pub fn end_composition(&self) -> anyhow::Result<()> {
        let composition = self
            .composition
            .borrow()
            .clone()
            .context("Composition not found")?;
        EditSession::handle(
            self.client_id,
            self.context.borrow().clone().context("Context not found")?,
            Rc::new(move |cookie| unsafe {
                composition.EndComposition(cookie)?;
                Ok(())
            }),
        )?;
        self.composition.replace(None);

        Ok(())
    }

    pub fn set_text(&self, text: &str) -> anyhow::Result<()> {
        self.preedit.replace(text.to_string());
        let composition = self
            .composition
            .borrow()
            .clone()
            .context("Composition not found")?;
        let context = self.context.borrow().clone().context("Context not found")?;
        let wide_text = to_wide_16(text);
        let pvar = VARIANT::from(self.display_attribute as i32);

        EditSession::handle(
            self.client_id,
            self.context.borrow().clone().context("Context not found")?,
            Rc::new(move |cookie| unsafe {
                let range = composition.GetRange()?;
                range.SetText(cookie, 0, &wide_text)?;

                let prop = context.GetProperty(&GUID_PROP_ATTRIBUTE)?;
                prop.SetValue(cookie, &range, &pvar)?;
                Ok(())
            }),
        )?;

        Ok(())
    }

    pub fn get_pos(&self) -> anyhow::Result<LocateEvent> {
        let rect = Rc::new(RefCell::new(RECT::default()));

        EditSession::handle(
            self.client_id,
            self.context.borrow().clone().context("Context not found")?,
            Rc::new({
                let context = self.context.borrow().clone().context("Context not found")?;
                let composition = self
                    .composition
                    .borrow()
                    .clone()
                    .context("Composition not found")?;
                let rect_clone = Rc::clone(&rect);
                let clipped = Rc::new(RefCell::new(BOOL::default()));

                move |cookie| unsafe {
                    let view = context.GetActiveView()?;
                    let range = composition.GetRange()?;
                    let mut rect_mut = rect_clone.borrow_mut();
                    let mut clipped_mut = clipped.borrow_mut();
                    view.GetTextExt(cookie, &range, &mut *rect_mut, &mut *clipped_mut)?;
                    Ok(())
                }
            }),
        )?;

        let rect = rect.borrow();

        Ok(LocateEvent {
            x: rect.left,
            y: rect.top,
        })
    }

    pub fn get_preceding_text(&self) -> anyhow::Result<String> {
        // mozcの実装を参考に
        // https://github.com/google/mozc/blob/master/src/win32/tip/tip_surrounding_text.cc
        unsafe {
            let preceding_text = Rc::new(RefCell::new(String::default()));

            let context = self.context.borrow().clone().context("Context not found")?;
            let docmgr = context.GetDocumentMgr()?;

            // mozcにはcontext_mgrがあるが、パス

            let compartment_mgr: ITfCompartmentMgr = docmgr.cast()?;
            let compartment =
                compartment_mgr.GetCompartment(&GUID_COMPARTMENT_TRANSITORYEXTENSION_PARENT)?;

            let variant = compartment.GetValue()?;
            let variant_punk = variant.as_raw().Anonymous.Anonymous.Anonymous.punkVal;
            let variant_unk: IUnknown = std::mem::transmute(variant_punk);

            let parent_docmgr: ITfDocumentMgr = variant_unk.cast()?;
            let parent_context = parent_docmgr.GetTop()?;

            EditSession::handle(
                self.client_id,
                parent_context.clone(),
                Rc::new({
                    // parent contextの取得
                    let preceding_text_clone = Rc::clone(&preceding_text);

                    move |cookie| {
                        // いろいろ準備
                        let mut pselection: [TF_SELECTION; 1] = [TF_SELECTION::default()];
                        let mut pfetched = 0;
                        parent_context.GetSelection(
                            cookie,
                            TF_DEFAULT_SELECTION,
                            &mut pselection,
                            &mut pfetched,
                        )?;

                        let prange = &pselection[0].range;
                        let range = <std::option::Option<ITfRange> as Clone>::clone(prange)
                            .context("Range not found")?;

                        // rangeの準備
                        let mut preceding_range_shifted = 0;

                        let halt_cond = TF_HALTCOND {
                            pHaltRange: ManuallyDrop::new(None),
                            aHaltPos: TF_ANCHOR_START,
                            dwFlags: TF_HF_OBJECT,
                        };

                        let preceding_range = range.Clone()?;
                        preceding_range.Collapse(cookie, TF_ANCHOR_START)?;
                        preceding_range.ShiftStart(
                            cookie,
                            -20,
                            &mut preceding_range_shifted,
                            &halt_cond,
                        )?;

                        // 前のテキストを取得
                        let mut pchtext = [0u16; 64];
                        let mut pcch = 0;
                        preceding_range.GetText(
                            cookie,
                            TF_TF_MOVESTART,
                            &mut pchtext,
                            &mut pcch,
                        )?;

                        preceding_text_clone
                            .replace(String::from_utf16_lossy(&pchtext[..pcch as usize]));

                        Ok(())
                    }
                }),
            )?;

            let result = preceding_text.borrow().clone();
            Ok(result)
        }
    }
}
