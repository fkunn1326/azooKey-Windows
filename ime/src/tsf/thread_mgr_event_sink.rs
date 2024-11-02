use windows::core::implement;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfDocumentMgr, ITfThreadMgrEventSink, ITfThreadMgrEventSink_Impl,
};

use anyhow::{Context, Result};

use ipc::socket::SocketManager;

use crate::handle_result;

use super::composition_mgr::CompositionMgr;
use super::key_event_sink::KeyEvent;

// イベントを受け取るクラス、編集コンテキストを作成したり、破棄したりするときに呼ばれる
#[implement(ITfThreadMgrEventSink)]
pub struct ThreadMgrEventSink {
    composition_mgr: CompositionMgr,
    socket_mgr: SocketManager,
}

impl ThreadMgrEventSink {
    pub fn new(composition_mgr: CompositionMgr, socket_mgr: SocketManager) -> Self {
        ThreadMgrEventSink {
            composition_mgr,
            socket_mgr,
        }
    }
}

impl ITfThreadMgrEventSink_Impl for ThreadMgrEventSink_Impl {
    fn OnInitDocumentMgr(&self, _doc_mgr: Option<&ITfDocumentMgr>) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnUninitDocumentMgr(&self, _doc_mgr: Option<&ITfDocumentMgr>) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnSetFocus(
        &self,
        docmgr: Option<&ITfDocumentMgr>,
        _prev_doc_mgr: Option<&ITfDocumentMgr>,
    ) -> windows::core::Result<()> {
        let result: Result<()> = (|| {
            if docmgr.is_none() {
                return Ok(());
            }
            let context = unsafe { docmgr.context("failed to get docmgr")?.GetBase() }?;
            if self.composition_mgr.composition.borrow().is_none() {
                self.composition_mgr.start_composition(context)?;
            }
            let preceding_text = self.composition_mgr.get_preceding_text()?;

            let message = serde_json::to_string(&KeyEvent {
                r#type: "left".to_string(),
                message: preceding_text,
            })?;
            self.socket_mgr.send(message)?;

            Ok(())
        })();

        handle_result!(result)
    }

    fn OnPushContext(&self, _context: Option<&ITfContext>) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnPopContext(&self, _context: Option<&ITfContext>) -> windows::core::Result<()> {
        Ok(())
    }
}
