use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use log::{info, debug};
use winapi::um::{
    userenv::{
        CreateEnvironmentBlock,
        DestroyEnvironmentBlock
    },
    winbase::{
        WTSGetActiveConsoleSessionId,
        CREATE_UNICODE_ENVIRONMENT,
        CREATE_NEW_CONSOLE,
        CREATE_NO_WINDOW
    },
    winnt::HANDLE,
    wtsapi32::WTSQueryUserToken,
    processthreadsapi::{
        CreateProcessAsUserW,
        STARTUPINFOW,
        PROCESS_INFORMATION
    },
    handleapi::CloseHandle
};
use winapi::ctypes::c_void;

fn get_session_user_token() -> Option<HANDLE> {
    let mut user_token: HANDLE = null_mut();
    let session_id = unsafe { WTSGetActiveConsoleSessionId() };
    if unsafe { WTSQueryUserToken(session_id, &mut user_token) } != 0 {
        Some(user_token)
    } else {
        None
    }
}

pub fn start_process_as_current_user(app_path: &str, cmd_line: Option<&str>, work_dir: Option<&str>, visible: bool) -> Result<u32, i32> {
    let h_user_token = match get_session_user_token() {
        Some(token) => token,
        None => return Err(-1)
    };

    let mut startup_info: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    startup_info.lpDesktop = null_mut();
    startup_info.wShowWindow = if visible { 5 } else { 0 }; // 5 = SW_SHOW

    let mut proc_info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let mut env: *mut c_void = null_mut();
    let creation_flags = CREATE_UNICODE_ENVIRONMENT | if visible { CREATE_NEW_CONSOLE } else { CREATE_NO_WINDOW };
    let app_path_wide: Vec<u16> = OsStr::new(app_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let cmd_line_wide: Option<Vec<u16>> = cmd_line
        .map(|s| OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0)).collect());
    let work_dir_wide: Option<Vec<u16>> = work_dir
        .map(|s| OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0)).collect());

    debug!("App path : {:?}",app_path);
    debug!("App path wide: {:?}",app_path_wide);
    debug!("CMD line: {:?}",cmd_line);
    debug!("CMD line wide: {:?}",cmd_line_wide);
    debug!("Work dir : {:?}",work_dir);
    debug!("Work dir wide: {:?}",work_dir_wide);
    debug!("User token {:?}", h_user_token);

    let result = unsafe {
        let env_block = CreateEnvironmentBlock(&mut env, h_user_token, 0);
        let user_process = CreateProcessAsUserW(
            h_user_token,
            app_path_wide.as_ptr(),
            cmd_line_wide.as_ref().map_or(null_mut(), |s| s.as_ptr() as *mut _),
            null_mut(),
            null_mut(),
            0,
            creation_flags,
            env,
            work_dir_wide.as_ref().map_or(null_mut(), |s| s.as_ptr() as *mut _),
            &mut startup_info,
            &mut proc_info
        );
        env_block != 0 && user_process !=0
    };
    info!("lpDesktop : {:?}", startup_info.lpDesktop);
    info!("Process ID: {:?}", proc_info.dwProcessId);
    info!("Tread ID: {:?}", proc_info.dwThreadId);

    unsafe {
        if !env.is_null() {
            DestroyEnvironmentBlock(env);
        }
        CloseHandle(proc_info.hThread);
        CloseHandle(proc_info.hProcess);
        CloseHandle(h_user_token);
    }
    if result {
        Ok(proc_info.dwProcessId)
    } else {
        let e = unsafe { winapi::um::errhandlingapi::GetLastError() as i32 };
        Err(e)
    }
}
