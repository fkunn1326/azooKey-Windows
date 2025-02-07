use macros::anyhow;
use windows::{
    core::{implement, AsImpl, VARIANT},
    Win32::{
        Foundation::RECT,
        UI::TextServices::{
            ITfComposition, ITfCompositionSink, ITfContext, ITfContextComposition, ITfEditSession,
            ITfEditSession_Impl, ITfInsertAtSelection, ITfRange, GUID_PROP_ATTRIBUTE, TF_AE_NONE,
            TF_ANCHOR_END, TF_ANCHOR_START, TF_ES_READWRITE, TF_IAS_QUERYONLY, TF_SELECTION,
            TF_SELECTIONSTYLE, TF_ST_CORRECTION, TF_TF_MOVESTART,
        },
    },
};

use std::{cell::Cell, mem::ManuallyDrop, rc::Rc};

use anyhow::{Context, Result};

use crate::{engine::state::IMEState, extension::StringExt as _, globals::GUID_DISPLAY_ATTRIBUTE};

use super::factory::TextServiceFactory;

#[implement(ITfEditSession)]
struct EditSession<'a, T> {
    callback: Rc<dyn Fn(u32) -> anyhow::Result<T>>,
    pub result: Cell<Option<T>>,
    phantom: std::marker::PhantomData<&'a T>,
}

// edit action will be performed within this function
pub fn edit_session<T>(
    client_id: u32,
    context: ITfContext,
    callback: Rc<dyn Fn(u32) -> anyhow::Result<T>>,
) -> Result<Option<T>> {
    let session: ITfEditSession = EditSession {
        callback,
        result: Cell::new(None),
        phantom: std::marker::PhantomData,
    }
    .into();

    let result = unsafe { context.RequestEditSession(client_id, &session, TF_ES_READWRITE) };

    match result {
        Ok(_) => {
            let session = unsafe { session.as_impl() };
            Ok(session.result.take())
        }
        Err(e) => Err(anyhow::Error::new(e)),
    }
}

impl<'a, T> ITfEditSession_Impl for EditSession_Impl<'a, T> {
    #[anyhow]
    fn DoEditSession(&self, cookie: u32) -> Result<()> {
        let result = (self.callback)(cookie)?;
        self.result.set(Some(result));
        Ok(())
    }
}

impl TextServiceFactory {
    #[tracing::instrument]
    pub fn start_composition(&self) -> Result<()> {
        tracing::debug!("start_composition");

        let text_service = self.borrow_mut()?;
        let context = text_service.context()?;
        let context_composition = text_service.context::<ITfContextComposition>()?;
        let sink = text_service.this::<ITfCompositionSink>()?;
        let insert = text_service.context::<ITfInsertAtSelection>()?;

        let tip_exists = {
            let composition = text_service.borrow_composition()?;
            composition.tip_composition.is_some()
        };

        if tip_exists {
            self.end_composition()?;
            return Ok(());
        }

        let composition = edit_session::<ITfComposition>(
            text_service.tid,
            context,
            Rc::new({
                move |cookie| unsafe {
                    let range = insert.InsertTextAtSelection(cookie, TF_IAS_QUERYONLY, &[])?;
                    let composition =
                        context_composition.StartComposition(cookie, &range, &sink)?;

                    Ok(composition)
                }
            }),
        )?;

        tracing::debug!("Composition started {composition:?}");
        text_service.borrow_mut_composition()?.tip_composition = composition;

        Ok(())
    }

    #[tracing::instrument]
    pub fn end_composition(&self) -> Result<()> {
        tracing::debug!("end_composition");
        let text_service = self.borrow()?;

        if let Some(composition) = text_service.borrow_composition()?.tip_composition.clone() {
            edit_session(
                text_service.tid,
                text_service.context()?,
                Rc::new({
                    let context = text_service.context::<ITfContext>()?;

                    move |cookie| unsafe {
                        // clear display attribute first
                        let range: ITfRange = composition.GetRange()?;

                        // set existing text to the composition
                        let mut text = vec![0; 1024];
                        let mut text_len = 1024;

                        let range_new = range.Clone()?;
                        range_new.GetText(cookie, TF_TF_MOVESTART, &mut text, &mut text_len)?;

                        text = text[..text_len as usize].to_vec();
                        range.SetText(cookie, TF_ST_CORRECTION, &text)?;

                        let prop = context.GetProperty(&GUID_PROP_ATTRIBUTE)?;
                        prop.Clear(cookie, &range)?;

                        // shift the start of the composition
                        range.Collapse(cookie, TF_ANCHOR_END)?;
                        let selection = TF_SELECTION {
                            range: ManuallyDrop::new(Some(range.clone())),
                            style: TF_SELECTIONSTYLE {
                                ase: TF_AE_NONE,
                                fInterimChar: false.into(),
                            },
                        };

                        context.SetSelection(cookie, &[selection])?;

                        composition.EndComposition(cookie)?;
                        Ok(())
                    }
                }),
            )?;
        } else {
            tracing::warn!("Composition is not started");
        }

        text_service.borrow_mut_composition()?.tip_composition = None;

        Ok(())
    }

    #[tracing::instrument]
    pub fn set_text(&self, text: &str, subtext: &str) -> Result<()> {
        let text_service = self.borrow()?;

        if let Some(composition) = text_service.borrow_composition()?.tip_composition.clone() {
            edit_session(
                text_service.tid,
                text_service.context()?,
                Rc::new({
                    let text_len = text.chars().count() as i32;

                    // unpadded is all you need!
                    let text = format!("{text}{subtext}").as_str().to_wide_16_unpadded();
                    let context = text_service.context::<ITfContext>()?;
                    let display_attribute_atom = text_service.display_attribute_atom.clone();

                    move |cookie| unsafe {
                        let range = composition.GetRange()?;
                        range.SetText(cookie, TF_ST_CORRECTION, &text)?;

                        // first, set the display attribute to the "text" part
                        let text_range = range.Clone()?;
                        text_range.Collapse(cookie, TF_ANCHOR_START)?;
                        let mut shifted: i32 = 0;
                        text_range.ShiftEnd(cookie, text_len, &mut shifted, std::ptr::null())?;
                        let display_attribute = display_attribute_atom.get(&GUID_DISPLAY_ATTRIBUTE);
                        if let Some(display_attribute) = display_attribute {
                            let pvar = VARIANT::from(*display_attribute as i32);
                            let prop = context.GetProperty(&GUID_PROP_ATTRIBUTE)?;
                            prop.SetValue(cookie, &text_range, &pvar)?;
                        }

                        range.Collapse(cookie, TF_ANCHOR_END)?;
                        let selection = TF_SELECTION {
                            range: ManuallyDrop::new(Some(range.clone())),
                            style: TF_SELECTIONSTYLE {
                                ase: TF_AE_NONE,
                                fInterimChar: false.into(),
                            },
                        };

                        context.SetSelection(cookie, &[selection])?;

                        Ok(())
                    }
                }),
            )?;
        } else {
            tracing::warn!("Composition is not started");
        }

        Ok(())
    }

    #[tracing::instrument]
    pub fn shift_start(&self, text: &str, subtext: &str) -> Result<()> {
        let text_service = self.borrow()?;

        if let Some(composition) = text_service.borrow_composition()?.tip_composition.clone() {
            edit_session(
                text_service.tid,
                text_service.context()?,
                Rc::new({
                    let text_len = text.chars().count() as i32;
                    let subtext = subtext.to_wide_16_unpadded();
                    let context = text_service.context::<ITfContext>()?;
                    let display_attribute_atom = text_service.display_attribute_atom.clone();

                    move |cookie| unsafe {
                        // first, shift the start of the composition
                        let range = composition.GetRange()?;
                        let mut shifted: i32 = 0;

                        // and clear the display attribute
                        let prop = context.GetProperty(&GUID_PROP_ATTRIBUTE)?;
                        prop.Clear(cookie, &range)?;

                        range.Collapse(cookie, TF_ANCHOR_START)?;
                        range.ShiftStart(cookie, text_len, &mut shifted, std::ptr::null())?;

                        composition.ShiftStart(cookie, &range)?;

                        // then, set the display attribute
                        let range = composition.GetRange()?;

                        range.SetText(cookie, TF_ST_CORRECTION, &subtext)?;

                        let display_attribute = display_attribute_atom.get(&GUID_DISPLAY_ATTRIBUTE);
                        if let Some(display_attribute) = display_attribute {
                            let pvar = VARIANT::from(*display_attribute as i32);
                            let prop = context.GetProperty(&GUID_PROP_ATTRIBUTE)?;
                            prop.SetValue(cookie, &range, &pvar)?;
                        }

                        range.Collapse(cookie, TF_ANCHOR_END)?;
                        let selection = TF_SELECTION {
                            range: ManuallyDrop::new(Some(range)),
                            style: TF_SELECTIONSTYLE {
                                ase: TF_AE_NONE,
                                fInterimChar: false.into(),
                            },
                        };

                        context.SetSelection(cookie, &[selection])?;

                        Ok(())
                    }
                }),
            )?;
        } else {
            tracing::warn!("Composition is not started");
        }

        Ok(())
    }

    #[tracing::instrument]
    pub fn update_pos(&self) -> Result<()> {
        let text_service = self.borrow()?;
        let composition = text_service.borrow_composition()?;

        if let Some(tip_composition) = composition.tip_composition.clone() {
            edit_session(
                text_service.tid,
                text_service.context()?,
                Rc::new({
                    let context = text_service.context::<ITfContext>()?;

                    move |cookie| unsafe {
                        let view = context.GetActiveView()?;
                        let range = tip_composition.GetRange()?;
                        let mut ipc_service = IMEState::get()?
                            .ipc_service
                            .clone()
                            .context("ipc_service is None")?;

                        let mut rect = RECT::default();
                        let mut clipped = false.into();
                        view.GetTextExt(cookie, &range, &mut rect, &mut clipped)?;

                        ipc_service.set_window_position(
                            rect.top,
                            rect.left,
                            rect.bottom,
                            rect.right,
                        )?;

                        Ok(())
                    }
                }),
            )?;

            return Ok(());
        } else {
            anyhow::bail!("Composition is not started");
        }
    }
}
