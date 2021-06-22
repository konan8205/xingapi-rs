// SPDX-License-Identifier: MPL-2.0

// XingAPI DLL를 불러오고 함수를 직접 호출하는 모듈입니다.
//
// XingAPI는 non-thread safe이기 때문에 실제 함수 호출은 단일 스레드에서만 해야 합니다.
// 따라서 실제 함수 호출을 대신 수행하는 별도의 스레드 객체인 Caller의 구현에 사용합니다.

use crate::{
    error::{EntryError, Error},
    euckr,
    os::windows::raw::XM_OFFSET,
};

use lazy_static::lazy_static;
use libloading::os::windows::{Library, Symbol};
use std::{path::Path, sync::Mutex};

use winapi::ctypes::{c_int, c_void};
use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, TRUE};
use winapi::shared::windef::HWND;

// 서버 연결, 로그인
type Connect = unsafe extern "system" fn(HWND, *const u8, c_int, c_int, c_int, c_int) -> BOOL;
type IsConnected = unsafe extern "system" fn() -> BOOL;
type Disconnect = unsafe extern "system" fn() -> BOOL;
type Login = unsafe extern "system" fn(HWND, *const u8, *const u8, *const u8, c_int, BOOL) -> BOOL;
type Logout = unsafe extern "system" fn(HWND) -> BOOL;

// 조회TR
type Request = unsafe extern "system" fn(
    HWND,
    *const u8,
    *const c_void,
    c_int,
    BOOL,
    *const u8,
    c_int,
) -> c_int;
type ReleaseRequestData = unsafe extern "system" fn(c_int);
type ReleaseMessageData = unsafe extern "system" fn(LPARAM);

// 실시간TR
type AdviseRealData = unsafe extern "system" fn(HWND, *const u8, *const u8, c_int) -> BOOL;
type UnadviseRealData = unsafe extern "system" fn(HWND, *const u8, *const u8, c_int) -> BOOL;
type UnadviseWindow = unsafe extern "system" fn(HWND) -> BOOL;

// 계좌
type GetAccountListCount = unsafe extern "system" fn() -> c_int;
type GetAccountList = unsafe extern "system" fn(c_int, *mut u8, c_int) -> BOOL;
type GetAccountName = unsafe extern "system" fn(*const u8, *mut u8, c_int) -> BOOL;
type GetAccountDetailName = unsafe extern "system" fn(*const u8, *mut u8, c_int) -> BOOL;
type GetAccountNickname = unsafe extern "system" fn(*const u8, *mut u8, c_int) -> BOOL;

// 정보
type GetLastError = unsafe extern "system" fn() -> c_int;
type GetErrorMessage = unsafe extern "system" fn(c_int, *mut u8, c_int) -> c_int;

type GetClientIp = unsafe extern "system" fn(*mut u8);
type GetServerName = unsafe extern "system" fn(*mut u8);
type GetApiPath = unsafe extern "system" fn(*mut u8);

type GetTrCountPerSec = unsafe extern "system" fn(*const u8) -> c_int;
type GetTrCountBaseSec = unsafe extern "system" fn(*const u8) -> c_int;
type GetTrCountRequest = unsafe extern "system" fn(*const u8) -> c_int;
type GetTrCountLimit = unsafe extern "system" fn(*const u8) -> c_int;

// 부가기능
type RequestService = unsafe extern "system" fn(HWND, *const u8, *const u8) -> c_int;
type RemoveService = unsafe extern "system" fn(HWND, *const u8, *const u8) -> c_int;

type RequestLinkToHts = unsafe extern "system" fn(HWND, *const u8, *const u8, *const u8) -> c_int;
type AdviseLinkFromHts = unsafe extern "system" fn(HWND);
type UnadviseLinkFromHts = unsafe extern "system" fn(HWND);

type Decompress = unsafe extern "system" fn(*const u8, *const u8, c_int) -> c_int;

#[allow(dead_code)]
pub struct Entry {
    _disable_send_sync: *const (),
    lib: Library,

    // 서버 연결, 로그인
    connect: Symbol<Connect>,
    is_connected: Symbol<IsConnected>,
    disconnect: Symbol<Disconnect>,
    login: Symbol<Login>,
    logout: Symbol<Logout>,

    // 조회TR
    request: Symbol<Request>,
    release_request_data: Symbol<ReleaseRequestData>,
    release_message_data: Symbol<ReleaseMessageData>,

    // 실시간TR
    advise_real_data: Symbol<AdviseRealData>,
    unadvise_real_data: Symbol<UnadviseRealData>,
    unadvise_window: Symbol<UnadviseWindow>,

    // 계좌
    get_acc_list_count: Symbol<GetAccountListCount>,
    get_acc_list: Symbol<GetAccountList>,
    get_acc_name: Symbol<GetAccountName>,
    get_acc_detail_name: Symbol<GetAccountDetailName>,
    get_acc_nickname: Symbol<GetAccountNickname>,

    // 정보
    get_client_ip: Symbol<GetClientIp>,
    get_server_name: Symbol<GetServerName>,
    get_api_path: Symbol<GetApiPath>,

    get_last_error: Symbol<GetLastError>,
    get_error_message: Symbol<GetErrorMessage>,

    get_tr_count_per_sec: Symbol<GetTrCountPerSec>,
    get_tr_count_base_sec: Symbol<GetTrCountBaseSec>,
    get_tr_count_request: Symbol<GetTrCountRequest>,
    get_tr_count_limit: Symbol<GetTrCountLimit>,

    // 부가기능
    request_service: Symbol<RequestService>,
    remove_service: Symbol<RemoveService>,

    request_link_to_hts: Symbol<RequestLinkToHts>,
    advise_link_from_hts: Symbol<AdviseLinkFromHts>,
    unadvise_link_from_hts: Symbol<UnadviseLinkFromHts>,

    decompress: Symbol<Decompress>,
}

#[allow(dead_code)]
impl Entry {
    fn load_lib(path: &Path) -> Result<Library, EntryError> {
        lazy_static! {
            static ref LOAD_LIB_LOCK: Mutex<()> = Mutex::new(());
        }

        let _lock_guard = LOAD_LIB_LOCK.lock().unwrap();

        if Library::open_already_loaded(path).is_ok() {
            return Err(EntryError::LibraryInUse);
        }

        unsafe { Library::new(path) }.map_err(|error| {
            let path = path.to_string_lossy().into();
            EntryError::Library { path, error }
        })
    }

    fn load_entry(lib: Library, path: &Path) -> Result<Self, EntryError> {
        macro_rules! load_sym {
            ($sym_name:literal) => {
                unsafe { lib.get($sym_name) }.map_err(|error| EntryError::Symbol {
                    symbol: std::str::from_utf8($sym_name).unwrap().to_owned(),
                    path: path.to_string_lossy().into(),
                    error,
                })
            };
        }

        Ok(Self {
            _disable_send_sync: std::ptr::null(),
            connect: load_sym!(b"ETK_Connect")?,
            is_connected: load_sym!(b"ETK_IsConnected")?,
            disconnect: load_sym!(b"ETK_Disconnect")?,
            login: load_sym!(b"ETK_Login")?,
            logout: load_sym!(b"ETK_Logout")?,
            request: load_sym!(b"ETK_Request")?,
            release_request_data: load_sym!(b"ETK_ReleaseRequestData")?,
            release_message_data: load_sym!(b"ETK_ReleaseMessageData")?,
            advise_real_data: load_sym!(b"ETK_AdviseRealData")?,
            unadvise_real_data: load_sym!(b"ETK_UnadviseRealData")?,
            unadvise_window: load_sym!(b"ETK_UnadviseWindow")?,
            get_acc_list_count: load_sym!(b"ETK_GetAccountListCount")?,
            get_acc_list: load_sym!(b"ETK_GetAccountList")?,
            get_acc_name: load_sym!(b"ETK_GetAccountName")?,
            get_acc_detail_name: load_sym!(b"ETK_GetAcctDetailName")?,
            get_acc_nickname: load_sym!(b"ETK_GetAcctNickname")?,
            get_last_error: load_sym!(b"ETK_GetLastError")?,
            get_error_message: load_sym!(b"ETK_GetErrorMessage")?,
            get_client_ip: load_sym!(b"ETK_GetClientIP")?,
            get_server_name: load_sym!(b"ETK_GetServerName")?,
            get_api_path: load_sym!(b"ETK_GetAPIPath")?,
            get_tr_count_per_sec: load_sym!(b"ETK_GetTRCountPerSec")?,
            get_tr_count_base_sec: load_sym!(b"ETK_GetTRCountBaseSec")?,
            get_tr_count_request: load_sym!(b"ETK_GetTRCountRequest")?,
            get_tr_count_limit: load_sym!(b"ETK_GetTRCountLimit")?,
            request_service: load_sym!(b"ETK_RequestService")?,
            remove_service: load_sym!(b"ETK_RemoveService")?,
            request_link_to_hts: load_sym!(b"ETK_RequestLinkToHTS")?,
            advise_link_from_hts: load_sym!(b"ETK_AdviseLinkFromHTS")?,
            unadvise_link_from_hts: load_sym!(b"ETK_UnAdviseLinkFromHTS")?,
            decompress: load_sym!(b"ETK_Decompress")?,
            lib,
        })
    }

    pub fn new() -> Result<Self, EntryError> {
        let sdk_dl = Path::new("C:/eBEST/xingAPI/xingAPI.dll");
        let dl = Path::new("xingAPI.dll");

        match Self::load_lib(sdk_dl) {
            Ok(lib) => Self::load_entry(lib, sdk_dl),
            Err(err) => {
                if let Ok(lib) = Self::load_lib(dl) {
                    Self::load_entry(lib, dl)
                } else {
                    Err(err)
                }
            }
        }
    }

    pub fn new_with_path<P: AsRef<Path>>(path: P) -> Result<Self, EntryError> {
        let path = path.as_ref();
        Self::load_entry(Self::load_lib(path)?, path)
    }

    pub fn connect(
        &self,
        hwnd: usize,
        addr: &str,
        port: u16,
        timeout: Option<i32>,
        max_packet_size: Option<i32>,
    ) -> Result<(), Error> {
        unsafe {
            if (self.connect)(
                hwnd as _,
                euckr::encode(addr).as_ptr(),
                port as _,
                XM_OFFSET as _,
                if let Some(t) = timeout {
                    assert!(t > 0);
                    t
                } else {
                    -1
                },
                if let Some(s) = max_packet_size {
                    assert!(s > 0);
                    s
                } else {
                    -1
                },
            ) == TRUE
            {
                Ok(())
            } else {
                Err(self.get_last_error())
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        unsafe { (self.is_connected)() == TRUE }
    }

    pub fn disconnect(&self) {
        unsafe {
            (self.disconnect)();
        }
    }

    pub fn login(
        &self,
        hwnd: usize,
        id: &str,
        pw: &str,
        cert_pw: &str,
        cert_err_dialog: bool,
    ) -> Result<(), Error> {
        unsafe {
            if (self.login)(
                hwnd as _,
                euckr::encode(id).as_ptr(),
                euckr::encode(pw).as_ptr(),
                euckr::encode(cert_pw).as_ptr(),
                0,
                if cert_err_dialog { TRUE } else { FALSE },
            ) == TRUE
            {
                Ok(())
            } else {
                Err(self.get_last_error())
            }
        }
    }

    pub fn request(
        &self,
        hwnd: usize,
        tr_code: &str,
        data: &[u8],
        continue_key: Option<&str>,
        timeout: Option<i32>,
    ) -> Result<i32, Error> {
        let req_id = unsafe {
            (self.request)(
                hwnd as _,
                euckr::encode(tr_code).as_ptr(),
                data.as_ptr() as *const _,
                data.len() as _,
                if continue_key.is_some() { TRUE } else { FALSE },
                match continue_key {
                    Some(key) => euckr::encode(key).as_ptr(),
                    None => b"".as_ptr(),
                },
                if let Some(t) = timeout {
                    assert!(t > 0);
                    t
                } else {
                    30
                },
            )
        };

        if req_id >= 0 {
            Ok(req_id)
        } else {
            Err(Error::XingApi { code: req_id, message: self.get_error_message(req_id) })
        }
    }

    pub fn release_request_data(&self, req_id: i32) {
        unsafe { (self.release_request_data)(req_id) }
    }

    pub fn release_message_data(&self, lparam: LPARAM) {
        unsafe { (self.release_message_data)(lparam) }
    }

    pub fn advise_real_data(&self, hwnd: usize, tr_code: &str, data: &[String]) -> Result<(), ()> {
        if data.iter().any(|s| !s.is_ascii()) {
            return Err(());
        }

        let max_len = data.iter().map(|s| s.len()).max().unwrap_or(0);
        let enc_data: String = data.iter().map(|s| format!("{:0>1$}", s, max_len)).collect();

        unsafe {
            if (self.advise_real_data)(
                hwnd as _,
                euckr::encode(tr_code).as_ptr(),
                euckr::encode(&enc_data).as_ptr(),
                max_len as _,
            ) == TRUE
            {
                Ok(())
            } else {
                Err(())
            }
        }
    }

    pub fn unadvise_real_data(
        &self,
        hwnd: usize,
        tr_code: &str,
        data: &[String],
    ) -> Result<(), ()> {
        if data.iter().any(|s| !s.is_ascii()) {
            return Err(());
        }

        let max_len = data.iter().map(|s| s.len()).max().unwrap_or(0);
        let enc_data: String = data.iter().map(|s| format!("{:0>1$}", s, max_len)).collect();

        unsafe {
            if (self.unadvise_real_data)(
                hwnd as _,
                euckr::encode(tr_code).as_ptr(),
                euckr::encode(&enc_data).as_ptr(),
                max_len as _,
            ) == TRUE
            {
                Ok(())
            } else {
                Err(())
            }
        }
    }

    pub fn unadvise_window(&self, hwnd: usize) -> Result<(), ()> {
        unsafe {
            // 반환형은 BOOL이지만 에러 코드를 반환하기도 합니다.
            if (self.unadvise_window)(hwnd as _) > 0 {
                Ok(())
            } else {
                Err(())
            }
        }
    }

    pub fn get_account_list(&self) -> Vec<String> {
        let len = unsafe { (self.get_acc_list_count)() };

        let mut accounts = Vec::with_capacity(len as _);
        let mut buffer = [0; 32];
        for i in 0..len {
            unsafe {
                assert_eq!((self.get_acc_list)(i, buffer.as_mut_ptr(), buffer.len() as _), TRUE);
            }
            accounts.push(euckr::decode(&buffer).trim_end().to_owned());
        }
        accounts
    }

    pub fn get_account_name(&self, account: &str) -> String {
        let mut buffer = [0; 64];
        unsafe {
            (self.get_acc_name)(
                euckr::encode(account).as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len() as _,
            );
        }
        euckr::decode(&buffer).trim_end().to_owned()
    }

    pub fn get_account_detail_name(&self, account: &str) -> String {
        let mut buffer = [0; 64];
        unsafe {
            (self.get_acc_detail_name)(
                euckr::encode(account).as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len() as _,
            );
        }
        euckr::decode(&buffer).trim_end().to_owned()
    }

    pub fn get_account_nickname(&self, account: &str) -> String {
        let mut buffer = [0; 64];
        unsafe {
            (self.get_acc_nickname)(
                euckr::encode(account).as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len() as _,
            );
        }
        euckr::decode(&buffer).trim_end().to_owned()
    }

    pub fn get_client_ip(&self) -> String {
        let mut buffer = [0; 64];
        unsafe {
            (self.get_client_ip)(buffer.as_mut_ptr());
        }
        euckr::decode(&buffer).trim_end().to_owned()
    }

    pub fn get_server_name(&self) -> String {
        let mut buffer = [0; 64];
        unsafe {
            (self.get_server_name)(buffer.as_mut_ptr());
        }
        euckr::decode(&buffer).trim_end().to_owned()
    }

    pub fn get_api_path(&self) -> String {
        let mut buffer = [0; 261];
        unsafe {
            (self.get_api_path)(buffer.as_mut_ptr());
        }
        euckr::decode(&buffer).trim_end().to_owned()
    }

    pub fn get_last_error(&self) -> Error {
        let code = unsafe { (self.get_last_error)() };
        Error::XingApi { code, message: self.get_error_message(code) }
    }

    pub fn get_error_message(&self, code: i32) -> String {
        unsafe {
            let mut buffer = [0; 1024];
            let len = (self.get_error_message)(code, buffer.as_mut_ptr(), buffer.len() as _);
            assert!(len >= 0);

            let len = len as usize;
            assert!(len <= buffer.len());
            euckr::decode(&buffer[0..len]).to_string()
        }
    }

    pub fn get_tr_count_per_sec(&self, tr_code: &str) -> i32 {
        unsafe { (self.get_tr_count_per_sec)(euckr::encode(tr_code).as_ptr()) }
    }
    pub fn get_tr_count_base_sec(&self, tr_code: &str) -> i32 {
        unsafe { (self.get_tr_count_base_sec)(euckr::encode(tr_code).as_ptr()) }
    }
    pub fn get_tr_count_request(&self, tr_code: &str) -> i32 {
        unsafe { (self.get_tr_count_request)(euckr::encode(tr_code).as_ptr()) }
    }
    pub fn get_tr_count_limit(&self, tr_code: &str) -> i32 {
        unsafe { (self.get_tr_count_limit)(euckr::encode(tr_code).as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::{Entry, EntryError};

    #[test]
    fn test_load_entry() -> Result<(), Box<dyn std::error::Error>> {
        let entry = Entry::new()?;
        println!("api_path: {:?}", entry.get_api_path());
        assert!(!entry.is_connected());

        Ok(())
    }

    #[test]
    fn test_load_entry_twice() {
        let _entry = Entry::new().unwrap();
        match Entry::new() {
            Err(EntryError::LibraryInUse) => {}
            _ => panic!("unexpected result"),
        }
    }
}
