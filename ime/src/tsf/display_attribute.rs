use std::cell::Cell;

use windows::core::{implement, BSTR, GUID};
use windows::Win32::{
    Foundation::FALSE,
    UI::TextServices::{
        IEnumTfDisplayAttributeInfo, IEnumTfDisplayAttributeInfo_Impl, ITfDisplayAttributeInfo,
        ITfDisplayAttributeInfo_Impl, TF_ATTR_CONVERTED, TF_ATTR_INPUT, TF_ATTR_TARGET_CONVERTED,
        TF_CT_NONE, TF_DA_COLOR, TF_DA_COLOR_0, TF_DISPLAYATTRIBUTE, TF_LS_SOLID, TF_LS_SQUIGGLE,
    },
};

use crate::utils::globals::{
    GUID_DISPLAY_ATTRIBUTE_CONVERTED, GUID_DISPLAY_ATTRIBUTE_FOCUSED, GUID_DISPLAY_ATTRIBUTE_INPUT,
};

// DisplayAttribute3点セット

// TF_DISPLAYATTRIBUTE構造体で表示を制御する属性を表す
pub const DISPLAY_ATTRIBUTE_FOCUSED: TF_DISPLAYATTRIBUTE = TF_DISPLAYATTRIBUTE {
    // 文字色
    crText: TF_DA_COLOR {
        r#type: TF_CT_NONE,
        Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
    },
    // 背景色
    crBk: TF_DA_COLOR {
        r#type: TF_CT_NONE,
        Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
    },
    // 下線のスタイル
    lsStyle: TF_LS_SOLID,
    // 下線の太さ
    fBoldLine: FALSE,
    // 下線の色
    crLine: TF_DA_COLOR {
        r#type: TF_CT_NONE,
        Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
    },
    // テキスト入力状態
    bAttr: TF_ATTR_TARGET_CONVERTED,
};

pub const DISPLAY_ATTRIBUTE_CONVERTED: TF_DISPLAYATTRIBUTE = TF_DISPLAYATTRIBUTE {
    crText: TF_DA_COLOR {
        r#type: TF_CT_NONE,
        Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
    },
    crBk: TF_DA_COLOR {
        r#type: TF_CT_NONE,
        Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
    },
    lsStyle: TF_LS_SOLID,
    fBoldLine: FALSE,
    crLine: TF_DA_COLOR {
        r#type: TF_CT_NONE,
        Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
    },
    bAttr: TF_ATTR_CONVERTED,
};

pub const DISPLAY_ATTRIBUTE_INPUT: TF_DISPLAYATTRIBUTE = TF_DISPLAYATTRIBUTE {
    crText: TF_DA_COLOR {
        r#type: TF_CT_NONE,
        Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
    },
    crBk: TF_DA_COLOR {
        r#type: TF_CT_NONE,
        Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
    },
    lsStyle: TF_LS_SQUIGGLE,
    fBoldLine: FALSE,
    crLine: TF_DA_COLOR {
        r#type: TF_CT_NONE,
        Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
    },
    bAttr: TF_ATTR_INPUT,
};

#[derive(Clone)]
#[implement(ITfDisplayAttributeInfo)]
pub struct DisplayAttributeInfo {
    description: String,
    pub guid: GUID,
    attribute: Cell<TF_DISPLAYATTRIBUTE>,
    attribute_backup: TF_DISPLAYATTRIBUTE,
}

impl DisplayAttributeInfo {
    pub fn new(description: String, guid: GUID, attribute: TF_DISPLAYATTRIBUTE) -> Self {
        DisplayAttributeInfo {
            description,
            guid,
            attribute: Cell::new(attribute),
            attribute_backup: attribute,
        }
    }
}

impl ITfDisplayAttributeInfo_Impl for DisplayAttributeInfo_Impl {
    fn GetAttributeInfo(&self, pda: *mut TF_DISPLAYATTRIBUTE) -> windows::core::Result<()> {
        unsafe {
            *pda = self.attribute.get();
        }
        Ok(())
    }

    fn GetGUID(&self) -> windows::core::Result<GUID> {
        Ok(self.guid)
    }

    fn Reset(&self) -> windows::core::Result<()> {
        self.attribute.set(self.attribute_backup);
        Ok(())
    }

    fn GetDescription(&self) -> windows::core::Result<BSTR> {
        Ok(BSTR::from(self.description.as_str()))
    }

    fn SetAttributeInfo(&self, pda: *const TF_DISPLAYATTRIBUTE) -> windows::core::Result<()> {
        unsafe {
            self.attribute.set(*pda);
        }
        Ok(())
    }
}

#[implement(IEnumTfDisplayAttributeInfo)]
pub struct EnumDisplayAttributeInfo {
    pub attributes: Vec<DisplayAttributeInfo>,
    index: Cell<usize>,
}

impl EnumDisplayAttributeInfo {
    pub fn new() -> Self {
        let attributes = vec![
            DisplayAttributeInfo::new(
                "Focused".to_string(),
                GUID_DISPLAY_ATTRIBUTE_FOCUSED,
                DISPLAY_ATTRIBUTE_FOCUSED,
            ),
            DisplayAttributeInfo::new(
                "Converted".to_string(),
                GUID_DISPLAY_ATTRIBUTE_CONVERTED,
                DISPLAY_ATTRIBUTE_CONVERTED,
            ),
            DisplayAttributeInfo::new(
                "Input".to_string(),
                GUID_DISPLAY_ATTRIBUTE_INPUT,
                DISPLAY_ATTRIBUTE_INPUT,
            ),
        ];

        EnumDisplayAttributeInfo {
            attributes,
            index: Cell::new(0),
        }
    }
}

impl IEnumTfDisplayAttributeInfo_Impl for EnumDisplayAttributeInfo_Impl {
    fn Clone(&self) -> windows::core::Result<IEnumTfDisplayAttributeInfo> {
        let clone = EnumDisplayAttributeInfo::new();
        clone.index.set(self.index.get());
        Ok(clone.into())
    }

    fn Next(
        &self,
        ulcount: u32,
        rginfo: *mut Option<ITfDisplayAttributeInfo>,
        pcfetched: *mut u32,
    ) -> windows::core::Result<()> {
        unsafe {
            if ulcount == 0 {
                return Ok(());
            }

            let mut fetched = 0;
            let mut index = self.index.get();

            while fetched < ulcount && index < self.attributes.len() {
                let attribute = match self.attributes.get(index).cloned() {
                    Some(attr) => attr,
                    None => return Err(windows::core::Error::from_win32()),
                };
                *rginfo = Some(attribute.into());
                fetched += 1;
                index += 1;
            }

            self.index.set(index);
            *pcfetched = fetched;
        }
        Ok(())
    }

    fn Reset(&self) -> windows::core::Result<()> {
        self.index.set(0);
        Ok(())
    }

    fn Skip(&self, ulcount: u32) -> windows::core::Result<()> {
        let index = self.index.get() + ulcount as usize;
        self.index.set(index);
        Ok(())
    }
}
