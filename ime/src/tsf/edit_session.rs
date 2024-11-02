use windows::core::implement;
use windows::Win32::Foundation::E_FAIL;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, TF_ES_READWRITE, TF_ES_SYNC,
};

use std::rc::Rc;

// テキスト編集に必要なクッキーを受け取り、編集処理を行うクラス
#[implement(ITfEditSession)]
pub struct EditSession {
    callback: Rc<dyn Fn(u32) -> anyhow::Result<()>>,
}

impl EditSession {
    pub fn new(callback: Rc<dyn Fn(u32) -> anyhow::Result<()>>) -> Self {
        EditSession { callback }
    }

    pub fn handle(
        client_id: u32,
        context: ITfContext,
        callback: Rc<dyn Fn(u32) -> anyhow::Result<()>>,
    ) -> anyhow::Result<()> {
        let session: ITfEditSession = EditSession::new(callback).into();

        let result = unsafe {
            context.RequestEditSession(client_id, &session, TF_ES_SYNC | TF_ES_READWRITE)
        };

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, cookie: u32) -> windows::core::Result<()> {
        (self.callback)(cookie).map_err(|e| windows::core::Error::new(E_FAIL, e.to_string()))?;
        Ok(())
    }
}
