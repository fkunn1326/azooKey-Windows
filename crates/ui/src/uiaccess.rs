// obtain useraccess privileges from system applications
// cf: https://github.com/killtimer0/uiaccess/

use std::{ffi::c_void, ptr::addr_of_mut};

use anyhow::Result;
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{CloseHandle, BOOL, HANDLE, INVALID_HANDLE_VALUE},
        Security::{
            DuplicateTokenEx, GetTokenInformation, LookupPrivilegeValueW, PrivilegeCheck,
            SecurityAnonymous, SecurityImpersonation, SetTokenInformation, TokenImpersonation,
            TokenPrimary, TokenSessionId, TokenUIAccess, PRIVILEGE_SET, SE_TCB_NAME,
            TOKEN_ACCESS_MASK, TOKEN_ADJUST_DEFAULT, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE,
            TOKEN_IMPERSONATE, TOKEN_QUERY,
        },
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
                TH32CS_SNAPPROCESS,
            },
            Environment::GetCommandLineW,
            SystemServices::PRIVILEGE_SET_ALL_NECESSARY,
            Threading::{
                CreateProcessAsUserW, ExitProcess, GetCurrentProcess, GetStartupInfoW, OpenProcess,
                OpenProcessToken, SetThreadToken, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION,
                PROCESS_QUERY_LIMITED_INFORMATION, STARTUPINFOW,
            },
        },
    },
};

/// get token from current process
fn open_current_process_token() -> Result<HANDLE> {
    let mut h_token = HANDLE::default();
    unsafe {
        if let Ok(()) = OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY,
            &mut h_token,
        ) {
            Ok(h_token)
        } else {
            anyhow::bail!("OpenProcessToken failed");
        }
    }
}

/// check for ui access
pub fn check_for_ui_access() -> Result<bool> {
    let mut token_ui_access: BOOL = false.into();
    let mut token_len: u32 = 0;

    unsafe {
        let h_token = open_current_process_token()?;
        let success = GetTokenInformation(
            h_token,
            TokenUIAccess,
            Some(&mut token_ui_access as *mut _ as *mut _),
            std::mem::size_of::<BOOL>() as u32,
            &mut token_len,
        );
        let _ = CloseHandle(h_token);
        if let Ok(()) = success {
            Ok(token_ui_access.as_bool())
        } else {
            anyhow::bail!("GetTokenInformation failed {success:?}");
        }
    }
}

pub fn duplicate_winlogon_token(
    session_id: u32,
    desired_access: TOKEN_ACCESS_MASK,
    h_token: &mut HANDLE,
) -> Result<()> {
    let mut privilege_set = PRIVILEGE_SET {
        PrivilegeCount: 1,
        Control: PRIVILEGE_SET_ALL_NECESSARY,
        ..Default::default()
    };

    unsafe {
        LookupPrivilegeValueW(None, SE_TCB_NAME, &mut privilege_set.Privilege[0].Luid)?;

        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        anyhow::ensure!(
            snapshot != INVALID_HANDLE_VALUE,
            "CreateToolhelp32Snapshot failed"
        );

        let mut process_entry = PROCESSENTRY32 {
            dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };

        Process32First(snapshot, &mut process_entry)?;
        while let Ok(()) = Process32Next(snapshot, &mut process_entry) {
            // check if the process is winlogon
            let exe_string = process_entry
                .szExeFile
                .iter()
                .map(|&c| c as u8)
                .collect::<Vec<_>>();
            let exe_string = String::from_utf8(exe_string)?;

            if !exe_string.to_lowercase().contains("winlogon") {
                continue;
            }

            let process = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                process_entry.th32ProcessID,
            )?;

            let mut token = HANDLE::default();
            OpenProcessToken(process, TOKEN_QUERY | TOKEN_DUPLICATE, &mut token)?;

            let mut privilege_result = false.into();
            PrivilegeCheck(token, &mut privilege_set as *mut _, &mut privilege_result)?;

            let mut token_session_id: u32 = 0;
            let mut token_info_length: u32 = 0;

            GetTokenInformation(
                token,
                TokenSessionId,
                Some(addr_of_mut!(token_session_id) as *mut c_void),
                std::mem::size_of::<u32>() as u32,
                &mut token_info_length,
            )?;

            anyhow::ensure!(
                token_session_id == session_id,
                "TokenSessionId does not match the session_id"
            );

            DuplicateTokenEx(
                token,
                desired_access,
                None,
                SecurityImpersonation,
                TokenImpersonation,
                h_token,
            )?;
        }
    }

    Ok(())
}

pub fn create_uiaccess_token(token_handle: &mut HANDLE) -> Result<()> {
    let mut token_self = HANDLE::default();

    unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY | TOKEN_DUPLICATE,
            &mut token_self,
        )?;

        let mut session_id = 0;
        let mut token_info_length = 0;

        GetTokenInformation(
            token_self,
            TokenSessionId,
            Some(addr_of_mut!(session_id) as *mut c_void),
            std::mem::size_of::<u32>() as u32,
            &mut token_info_length,
        )?;

        let mut system_token_handle = HANDLE::default();
        duplicate_winlogon_token(session_id, TOKEN_IMPERSONATE, &mut system_token_handle)?;

        SetThreadToken(None, system_token_handle)?;
        DuplicateTokenEx(
            token_self,
            TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT,
            None,
            SecurityAnonymous,
            TokenPrimary,
            token_handle,
        )?;

        let ui_access: BOOL = true.into();

        SetTokenInformation(
            *token_handle,
            TokenUIAccess,
            &ui_access as *const _ as *mut _,
            std::mem::size_of::<BOOL>() as u32,
        )?;
    }

    Ok(())
}

pub fn prepare_uiaccess_token() -> Result<()> {
    let ui_access = check_for_ui_access()?;
    if ui_access {
        println!("UIAccess is already enabled");
        return Ok(());
    }

    let mut token_handle = HANDLE::default();
    create_uiaccess_token(&mut token_handle)?;

    let mut startup_info = STARTUPINFOW::default();
    let mut process_info = PROCESS_INFORMATION::default();

    unsafe {
        GetStartupInfoW(&mut startup_info);
        CreateProcessAsUserW(
            token_handle,
            None,
            PWSTR(GetCommandLineW().as_ptr() as *mut u16),
            None,
            None,
            false,
            PROCESS_CREATION_FLAGS::default(),
            None,
            None,
            &mut startup_info,
            &mut process_info,
        )?;

        CloseHandle(process_info.hProcess)?;
        CloseHandle(process_info.hThread)?;
        ExitProcess(0);
    }
}
