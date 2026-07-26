use std::{cell::Cell, rc::Rc};

use windows::{
    core::GUID,
    Win32::{
        Foundation::{BOOL, LPARAM, WPARAM},
        UI::TextServices::{ITfComposition, ITfCompositionSink, ITfContext, ITfKeyEventSink_Impl},
    },
};

use crate::globals::GUID_DISPLAY_ATTRIBUTE;

use super::{edit_session::request_edit_session, text_service::TextService_Impl};

// sink (aka event listener) for key events
// 返り値はS_OKのみであることに注意
impl ITfKeyEventSink_Impl for TextService_Impl {
    #[macros::anyhow(ignore_with = false.into())]
    fn OnTestKeyDown(
        &self,
        _pic: Option<&ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(true.into())
    }

    #[macros::anyhow(ignore_with = false.into())]
    fn OnKeyDown(
        &self,
        pic: Option<&ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        let context = match pic {
            Some(ctx) => ctx,
            None => return Ok(false.into()),
        };

        let tid = self.tid.get();
        if tid == 0 {
            return Ok(false.into());
        }

        let context_manager = self.contexts.borrow();
        let context_state = match context_manager.find(&context) {
            Some(state) => state,
            None => return Ok(false.into()),
        };

        let composition: Rc<Cell<Option<ITfComposition>>> = Rc::new(Cell::new(None));
        let composition_ref = composition.clone();

        let composition_sink: ITfCompositionSink = match self.this() {
            Ok(sink) => sink,
            // TODO: tracingする
            Err(_) => return Ok(false.into()),
        };

        let atom = self.display_attribute_atom.take();
        let attr_atom = atom.get(&GUID_DISPLAY_ATTRIBUTE).copied();
        self.display_attribute_atom.set(atom);

        let edit_result = request_edit_session(context, tid, move |editor| {
            let range = editor.get_insertion_range()?;
            let composition = editor.start_composition(&range, &composition_sink)?;

            editor.set_composition_text(&composition, &crate::client::convert("ABC"))?;

            if let Some(atom) = attr_atom {
                editor.set_display_attribute(&range, atom)?;
            }

            composition_ref.set(Some(composition));
            Ok(())
        });

        if let Some(comp) = composition.take() {
            context_state.composition.set(Some(comp));
        }

        match edit_result {
            Ok(_) => Ok(true.into()),
            Err(e) => {
                // エラーを返す代わりに、falseを返す
                tracing::error!("request_edit_session failed (safe rollback): {:?}", e);
                Ok(false.into())
            }
        }
    }

    #[macros::anyhow(ignore_with = false.into())]
    fn OnTestKeyUp(
        &self,
        _pic: Option<&ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        // same as OnTestKeyDown
        Ok(false.into())
    }

    #[macros::anyhow(ignore_with = false.into())]
    fn OnKeyUp(&self, _pic: Option<&ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        // this function is called when a key is released
        // but we handle key events in OnKeyDown function
        // so just return S_OK
        Ok(false.into())
    }

    #[macros::anyhow(ignore_with = false.into())]
    fn OnPreservedKey(&self, _pic: Option<&ITfContext>, _rguid: *const GUID) -> Result<BOOL> {
        // this function is actually not used
        Ok(true.into())
    }

    #[macros::anyhow]
    fn OnSetFocus(&self, _fforeground: BOOL) -> Result<()> {
        Ok(())
    }
}
