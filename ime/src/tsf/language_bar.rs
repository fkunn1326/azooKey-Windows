use windows::core::{implement, IUnknown, Interface, BSTR, GUID, PCWSTR};
use windows::Win32::Foundation::E_INVALIDARG;
use windows::Win32::UI::TextServices::{
    ITfLangBarItemSink, GUID_LBI_INPUTMODE, TF_LBI_STYLE_BTN_BUTTON, TF_LBI_STYLE_TEXTCOLORICON,
};
use windows::Win32::{
    Foundation::{BOOL, POINT, RECT},
    System::Ole::CONNECT_E_CANNOTCONNECT,
    UI::{
        TextServices::{
            ITfLangBarItem, ITfLangBarItemButton, ITfLangBarItemButton_Impl, ITfLangBarItemMgr,
            ITfLangBarItem_Impl, ITfMenu, ITfSource, ITfSource_Impl, ITfThreadMgr, TfLBIClick,
            TF_LANGBARITEMINFO,
        },
        WindowsAndMessaging::{LoadImageW, HICON, IMAGE_ICON, LR_DEFAULTCOLOR},
    },
};

use crate::handle_result;
use crate::utils::globals::GUID_TEXT_SERVICE;
use crate::{dll::DllModule, utils::globals::TEXTSERVICE_LANGBARITEMSINK_COOKIE};

use anyhow::Result;

// https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/TSF/language-bar.md
// https://github.com/microsoft/Windows-classic-samples/blob/main/Samples/Win7Samples/winui/input/tsf/textservice/textservice-step04/LanguageBar.cpp

// 言語バー（"あ"とか"A"とかのやつ）を扱うクラス
#[implement(ITfSource, ITfLangBarItem, ITfLangBarItemButton)]
pub struct LanguageBar {
    thread_mgr: ITfThreadMgr,
}

// これを用意しないと言語バーは表示されない
static INFO: TF_LANGBARITEMINFO = TF_LANGBARITEMINFO {
    clsidService: GUID_TEXT_SERVICE,
    guidItem: GUID_LBI_INPUTMODE,
    dwStyle: TF_LBI_STYLE_BTN_BUTTON | TF_LBI_STYLE_TEXTCOLORICON,
    ulSort: 0,
    szDescription: [0; 32],
};

impl LanguageBar {
    pub fn new(thread_mgr: ITfThreadMgr) -> windows::core::Result<ITfLangBarItemButton> {
        let this = LanguageBar {
            thread_mgr: thread_mgr.clone(),
        };
        let item: ITfLangBarItemButton = this.into();
        LanguageBar::add_item(thread_mgr.clone(), item.clone())?;
        Ok(item)
    }

    pub fn deactivate(&self, item: ITfLangBarItemButton) -> windows::core::Result<()> {
        LanguageBar::remove_item(self, item)
    }

    fn add_item(thread_mgr: ITfThreadMgr, item: ITfLangBarItemButton) -> windows::core::Result<()> {
        let langbar_mgr: ITfLangBarItemMgr = thread_mgr.cast()?;
        unsafe { langbar_mgr.AddItem(&item)? }

        Ok(())
    }

    fn remove_item(&self, item: ITfLangBarItemButton) -> windows::core::Result<()> {
        let langbar_mgr: ITfLangBarItemMgr = self.thread_mgr.cast()?;
        unsafe { langbar_mgr.RemoveItem(&item)? }

        Ok(())
    }
}

impl ITfLangBarItem_Impl for LanguageBar_Impl {
    fn GetInfo(&self, p_info: *mut TF_LANGBARITEMINFO) -> windows::core::Result<()> {
        unsafe {
            *p_info = INFO;
        }
        Ok(())
    }

    fn GetStatus(&self) -> windows::core::Result<u32> {
        Ok(0)
    }

    fn Show(&self, _f_show: BOOL) -> windows::core::Result<()> {
        Ok(())
    }

    fn GetTooltipString(&self) -> windows::core::Result<BSTR> {
        Ok(BSTR::from("GetTooltipString"))
    }
}

impl ITfLangBarItemButton_Impl for LanguageBar_Impl {
    fn OnClick(
        &self,
        _click: TfLBIClick,
        _pt: &POINT,
        _prcarea: *const RECT,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn InitMenu(&self, _pmenu: Option<&ITfMenu>) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnMenuSelect(&self, _w_id: u32) -> windows::core::Result<()> {
        Ok(())
    }

    fn GetIcon(&self) -> windows::core::Result<HICON> {
        let result: Result<HICON> = (|| {
            let dll_module = DllModule::global()
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to get DllModule"))?;

            unsafe {
                let handle = LoadImageW(
                    dll_module.hinst,
                    PCWSTR(102 as *mut u16),
                    IMAGE_ICON,
                    0,
                    0,
                    LR_DEFAULTCOLOR,
                )?;

                Ok(HICON(handle.0))
            }
        })();

        handle_result!(result)
    }

    fn GetText(&self) -> windows::core::Result<BSTR> {
        Ok(BSTR::from("GetText"))
    }
}

impl ITfSource_Impl for LanguageBar_Impl {
    fn AdviseSink(&self, riid: *const GUID, punk: Option<&IUnknown>) -> windows::core::Result<u32> {
        let riid = unsafe { *riid };

        if riid != ITfLangBarItemSink::IID {
            return Err(E_INVALIDARG.into());
        }

        if punk.is_none() {
            return Err(E_INVALIDARG.into());
        }

        Ok(TEXTSERVICE_LANGBARITEMSINK_COOKIE)
    }

    fn UnadviseSink(&self, dw_cookie: u32) -> windows::core::Result<()> {
        if dw_cookie != TEXTSERVICE_LANGBARITEMSINK_COOKIE {
            return Err(CONNECT_E_CANNOTCONNECT.into());
        }

        Ok(())
    }
}
