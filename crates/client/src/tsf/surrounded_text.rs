// reference to the original code:
// https://github.com/google/mozc/blob/master/src/win32/tip/tip_surrounding_text.cc

use std::{mem::ManuallyDrop, rc::Rc};

use anyhow::{Context as _, Result};
use windows::{
    core::{IUnknown, Interface},
    Win32::UI::TextServices::{
        ITfCompartmentMgr, ITfContext, ITfDocumentMgr, GUID_COMPARTMENT_TRANSITORYEXTENSION_PARENT,
        TF_ANCHOR_START, TF_DEFAULT_SELECTION, TF_HALTCOND, TF_HF_OBJECT, TF_SELECTION,
        TF_TF_MOVESTART, TS_SS_TRANSITORY,
    },
};

use crate::engine::state::IMEState;

use super::{edit_session::edit_session, factory::TextServiceFactory};

impl TextServiceFactory {
    fn to_parent_document_if_exists(
        &self,
        document_manager: Option<ITfDocumentMgr>,
    ) -> Result<ITfDocumentMgr> {
        let document_manager = match document_manager {
            Some(doc_mgr) => doc_mgr,
            None => return Err(anyhow::anyhow!("Document manager is null")),
        };

        unsafe {
            // Get top context
            let context = match document_manager.GetTop() {
                Ok(ctx) => ctx,
                Err(_) => return Ok(document_manager),
            };

            // Get status
            let status = match context.GetStatus() {
                Ok(s) => s,
                Err(_) => return Ok(document_manager),
            };

            // Check if context is transitory
            if (status.dwStaticFlags & TS_SS_TRANSITORY) != TS_SS_TRANSITORY {
                return Ok(document_manager);
            }

            // Get compartment manager
            let compartment_mgr = match document_manager.cast::<ITfCompartmentMgr>() {
                Ok(mgr) => mgr,
                Err(_) => return Ok(document_manager),
            };

            // Get compartment
            let compartment = match compartment_mgr
                .GetCompartment(&GUID_COMPARTMENT_TRANSITORYEXTENSION_PARENT)
            {
                Ok(comp) => comp,
                Err(_) => return Ok(document_manager),
            };

            // Get value
            let variant = match compartment.GetValue() {
                Ok(var) => var,
                Err(_) => return Ok(document_manager),
            };

            // Check if variant is VT_UNKNOWN and not null
            if variant.as_raw().Anonymous.Anonymous.vt != 13u16 || // VT_UNKNOWN = 13
               variant.as_raw().Anonymous.Anonymous.Anonymous.punkVal.is_null()
            {
                return Ok(document_manager);
            }

            // Get parent document manager
            let variant_punk = variant.as_raw().Anonymous.Anonymous.Anonymous.punkVal;
            let variant_unk: IUnknown = std::mem::transmute(variant_punk);

            match variant_unk.cast::<ITfDocumentMgr>() {
                Ok(parent_doc_mgr) => Ok(parent_doc_mgr),
                Err(_) => Ok(document_manager),
            }
        }
    }

    fn to_parent_context_if_exists(&self, context: Option<ITfContext>) -> Result<ITfContext> {
        let context = match context {
            Some(ctx) => ctx,
            None => return Err(anyhow::anyhow!("Context is null")),
        };

        unsafe {
            // Get document manager
            let document_mgr = match context.GetDocumentMgr() {
                Ok(doc_mgr) => doc_mgr,
                Err(_) => return Ok(context),
            };

            // Get parent document
            let parent_doc_mgr = self.to_parent_document_if_exists(Some(document_mgr))?;

            // Get top context from parent document
            let parent_context = match parent_doc_mgr.GetTop() {
                Ok(ctx) => ctx,
                Err(_) => return Ok(context),
            };

            Ok(parent_context)
        }
    }

    pub fn update_context(&self, preview: &str) -> Result<()> {
        unsafe {
            let text_service = self.borrow()?;

            let context = text_service.context::<ITfContext>()?;
            let parent_context = self.to_parent_context_if_exists(Some(context))?;

            let preceding_text = edit_session::<String>(
                text_service.tid,
                parent_context.clone(),
                Rc::new({
                    let preview_count = preview.chars().count() as i32;

                    move |cookie| {
                        // 2. Get the selection from the parent context.
                        let mut pselection: [TF_SELECTION; 1] = [TF_SELECTION::default()];
                        let mut pfetched = 0;
                        parent_context.GetSelection(
                            cookie,
                            TF_DEFAULT_SELECTION,
                            &mut pselection,
                            &mut pfetched,
                        )?;

                        let prange = &pselection[0].range;
                        let range = prange.as_ref().context("Range not found")?.Clone()?;

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
                            -30,
                            &mut preceding_range_shifted,
                            &halt_cond,
                        )?;

                        preceding_range.ShiftEnd(
                            cookie,
                            -preview_count,
                            &mut preceding_range_shifted,
                            &halt_cond,
                        )?;

                        let mut pchtext = [0u16; 64];
                        let mut pcch = 0;
                        preceding_range.GetText(
                            cookie,
                            TF_TF_MOVESTART,
                            &mut pchtext,
                            &mut pcch,
                        )?;

                        Ok(String::from_utf16_lossy(&pchtext[..pcch as usize]))
                    }
                }),
            )?;

            let mut ipc_service = IMEState::get()?
                .ipc_service
                .clone()
                .context("ipc_service is None")?;

            ipc_service.set_context(preceding_text.context("preceding_text is null")?)?;

            Ok(())
        }
    }
}
