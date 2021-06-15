// SPDX-License-Identifier: MPL-2.0

// XingAPI 및 Win32 API 함수 호출을 별도의 스레드에서 비동기적으로 대신 수행하고
// 윈도우 메시지 루프를 반복하는 스레드에 대한 모듈입니다.

use super::{bindings, entry::Entry};
use crate::error::{EntryError, Error, Win32Error};

use std::{
    future::Future,
    mem::MaybeUninit,
    panic::{RefUnwindSafe, UnwindSafe},
    path::Path,
    pin::Pin,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
    thread::{self, JoinHandle},
    time::Duration,
};

use bindings::{
    CreateWindowExA, DestroyWindow, DispatchMessageA, GetModuleHandleA, PeekMessageA,
    TranslateMessage, HWND_MESSAGE, MSG, PM_REMOVE, TRUE,
};

macro_rules! define_fn {
    ($($func_camel_case:ident($($arg:ty$(,)?)*) -> $ret:ty)*) => {
        #[allow(dead_code)]
        enum CallerFn {
            $($func_camel_case {
                args: ($($arg,)*),
                tx_ret: crossbeam_channel::Sender<$ret>,
                waker: Arc<Mutex<Option<Waker>>>,
            },)*
        }
    };
}

define_fn! {
    // Win32 API
    CreateWindow(Vec<i8>) -> Result<usize, Win32Error>
    DestroyWindow(usize) -> Result<(), Win32Error>

    // XingAPI
    Connect(usize, String, u16, Option<i32>, Option<i32>) -> Result<(), Error>
    IsConnected() -> bool
    Disconnect() -> ()
    Login(usize, String, String, String, bool) -> Result<(), Error>
    Request(
        usize,
        String,
        Vec<u8>,
        Option<String>,
        Option<i32>
    ) -> Result<i32, Error>
    AdviseRealData(usize, String, Vec<String>) -> Result<(), ()>
    UnadviseRealData(usize, String, Vec<String>) -> Result<(), ()>
    UnadviseWindow(usize) -> Result<(), ()>
    GetAccountList() -> Vec<String>
    GetAccountName(String) -> String
    GetAccountDetailName(String) -> String
    GetAccountNickname(String) -> String
    GetClientIp() -> String
    GetServerName() -> String
    GetApiPath() -> String
    GetTrCountPerSec(String) -> i32
    GetTrCountBaseSec(String) -> i32
    GetTrCountRequest(String) -> i32
    GetTrCountLimit(String) -> i32
}

// caller 스레드에서 호출될 함수의 반환을 기다리는 Future입니다.
pub struct RetFuture<T> {
    rx_ret: crossbeam_channel::Receiver<T>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl<T> Future for RetFuture<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Ok(ret) = self.rx_ret.try_recv() {
            Poll::Ready(ret)
        } else {
            *self.waker.lock().unwrap() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T> Drop for RetFuture<T> {
    fn drop(&mut self) {
        *self.waker.lock().unwrap() = None;
    }
}

macro_rules! call {
    ($self:ident, $func_camel_case:ident($($arg:expr$(,)?)*)) => {{
        let (tx_ret, rx_ret) = crossbeam_channel::bounded(1);
        let waker = Arc::new(Mutex::new(None));

        $self.tx_func.send(CallerFn::$func_camel_case {
            args: ($($arg.into(),)*),
            tx_ret,
            waker: waker.clone(),
        }).unwrap();

        RetFuture {
            rx_ret,
            waker
        }
    }};
}

// Caller의 핸들로, 함수 호출을 요청할 수 있는 핸들입니다.
//
// XingAPI 서버 연결 및 로그인 이벤트의 동기적 처리를 위해 RwLock에 묶여 있습니다.
pub struct CallerHandle {
    tx_func: crossbeam_channel::Sender<CallerFn>,
}

impl CallerHandle {
    pub fn connect(
        &self,
        hwnd: usize,
        addr: &str,
        port: u16,
        timeout: Option<i32>,
        max_packet_size: Option<i32>,
    ) -> RetFuture<Result<(), Error>> {
        call!(self, Connect(hwnd, addr, port, timeout, max_packet_size))
    }

    pub fn is_connected(&self) -> RetFuture<bool> {
        call!(self, IsConnected())
    }
    pub fn disconnect(&self) -> RetFuture<()> {
        call!(self, Disconnect())
    }

    pub fn login(
        &self,
        hwnd: usize,
        id: &str,
        pw: &str,
        cert_pw: &str,
        cert_err_dialog: bool,
    ) -> RetFuture<Result<(), Error>> {
        call!(self, Login(hwnd, id, pw, cert_pw, cert_err_dialog))
    }

    pub fn request(
        &self,
        hwnd: usize,
        tr_code: &str,
        data: Vec<u8>,
        continue_key: Option<&str>,
        timeout: Option<i32>,
    ) -> RetFuture<Result<i32, Error>> {
        let continue_key = continue_key.map(|s| s.to_owned());
        call!(self, Request(hwnd, tr_code, data, continue_key, timeout))
    }

    pub fn advise_real_data(
        &self,
        hwnd: usize,
        tr_code: &str,
        data: Vec<String>,
    ) -> RetFuture<Result<(), ()>> {
        call!(self, AdviseRealData(hwnd, tr_code, data))
    }

    pub fn unadvise_real_data(
        &self,
        hwnd: usize,
        tr_code: &str,
        data: Vec<String>,
    ) -> RetFuture<Result<(), ()>> {
        call!(self, UnadviseRealData(hwnd, tr_code, data))
    }

    pub fn get_account_list(&self) -> RetFuture<Vec<String>> {
        call!(self, GetAccountList())
    }
    pub fn get_account_name(&self, account: &str) -> RetFuture<String> {
        call!(self, GetAccountName(account))
    }
    pub fn get_account_detail_name(&self, account: &str) -> RetFuture<String> {
        call!(self, GetAccountDetailName(account))
    }
    pub fn get_account_nickname(&self, account: &str) -> RetFuture<String> {
        call!(self, GetAccountNickname(account))
    }
    pub fn get_client_ip(&self) -> RetFuture<String> {
        call!(self, GetClientIp())
    }
    pub fn get_server_name(&self) -> RetFuture<String> {
        call!(self, GetServerName())
    }
    pub fn get_api_path(&self) -> RetFuture<String> {
        call!(self, GetApiPath())
    }
    pub fn get_tr_count_per_sec(&self, tr_code: &str) -> RetFuture<i32> {
        call!(self, GetTrCountPerSec(tr_code))
    }
    pub fn get_tr_count_base_sec(&self, tr_code: &str) -> RetFuture<i32> {
        call!(self, GetTrCountBaseSec(tr_code))
    }
    pub fn get_tr_count_request(&self, tr_code: &str) -> RetFuture<i32> {
        call!(self, GetTrCountRequest(tr_code))
    }
    pub fn get_tr_count_limit(&self, tr_code: &str) -> RetFuture<i32> {
        call!(self, GetTrCountLimit(tr_code))
    }
}

// 함수 호출 및 윈도우 메시지 루프를 대신 수행하는 스레드 객체입니다.
pub struct Caller {
    join_handle: Option<JoinHandle<()>>,
    tx_func: crossbeam_channel::Sender<CallerFn>,
    tx_quit: crossbeam_channel::Sender<()>,
    handle: async_lock::RwLock<CallerHandle>,
    entry: AtomicPtr<Entry>,
}

impl Caller {
    pub fn create_window(&self, class_name: &[i8]) -> RetFuture<Result<usize, Win32Error>> {
        call!(self, CreateWindow(class_name.to_owned()))
    }

    pub fn destroy_window(&self, hwnd: usize) -> RetFuture<Result<(), Win32Error>> {
        call!(self, DestroyWindow(hwnd))
    }

    pub fn unadvise_window(&self, hwnd: usize) -> RetFuture<Result<(), ()>> {
        call!(self, UnadviseWindow(hwnd))
    }

    pub fn is_caller_thread() -> bool {
        std::thread::current().name() == Some("xingapi_caller_thread")
    }

    pub fn handle(&self) -> &async_lock::RwLock<CallerHandle> {
        &self.handle
    }

    pub fn entry(&self) -> &Entry {
        debug_assert!(Caller::is_caller_thread());
        unsafe { &*self.entry.load(Ordering::Relaxed) }
    }

    pub fn new(path: Option<&Path>) -> Result<Self, EntryError> {
        let path = path.map(|s| s.to_owned());
        let (tx_result, rx_result) = crossbeam_channel::bounded(1);

        let (tx_func, rx_func) = crossbeam_channel::unbounded::<CallerFn>();
        let (tx_quit, rx_quit) = crossbeam_channel::bounded::<()>(1);

        let thread_main = move || {
            let load_entry = || -> Result<Pin<Box<Entry>>, EntryError> {
                Ok(Pin::new(Box::new(if let Some(path) = path.as_deref() {
                    Entry::new_with_path(path)?
                } else {
                    Entry::new()?
                })))
            };

            let entry = match load_entry() {
                Ok(mut entry) => {
                    let entry_ptr = AtomicPtr::new(entry.as_mut().get_mut());
                    tx_result.send(Ok(entry_ptr)).unwrap();
                    entry
                }
                Err(err) => {
                    tx_result.send(Err(err)).unwrap();
                    return;
                }
            };

            let mut quit = false;
            loop {
                unsafe {
                    #[allow(clippy::uninit_assumed_init)]
                    let mut msg = MaybeUninit::<MSG>::uninit().assume_init();
                    while PeekMessageA(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) == TRUE {
                        TranslateMessage(&msg);
                        DispatchMessageA(&msg);
                    }
                }

                // 일정 시간 동안 thread가 parking 됩니다.
                if let Ok(func) = rx_func.recv_timeout(Duration::from_micros(100)) {
                    Self::dispatch_func(&entry, func);

                    for _ in 1..100 {
                        if let Ok(func) = rx_func.try_recv() {
                            Self::dispatch_func(&entry, func);
                        } else {
                            break;
                        }
                    }
                } else if quit {
                    break;
                }

                if let Ok(()) = rx_quit.try_recv() {
                    quit = true;
                }
            }
        };

        let join_handle = Some(
            thread::Builder::new().name("xingapi_caller_thread".into()).spawn(thread_main).unwrap(),
        );

        let entry = rx_result.recv().unwrap()?;

        Ok(Self {
            join_handle,
            tx_func: tx_func.clone(),
            tx_quit,
            handle: async_lock::RwLock::new(CallerHandle { tx_func }),
            entry,
        })
    }

    fn dispatch_func(entry: &Entry, func: CallerFn) {
        macro_rules! ret {
            ($tx_ret:ident, $waker:ident, $ret:expr) => {{
                let _ = $tx_ret.try_send($ret);
                if let Some(waker) = $waker.lock().unwrap().take() {
                    waker.wake();
                }
            }};
        }

        match func {
            // Win32 API
            CallerFn::CreateWindow { args: (class_name,), tx_ret, waker } => {
                #[rustfmt::skip]
                let hwnd = unsafe {
                    CreateWindowExA(
                        0,
                        class_name.as_ptr(),
                        std::ptr::null_mut(),
                        0, 0, 0, 0, 0,
                        HWND_MESSAGE,
                        std::ptr::null_mut(),
                        GetModuleHandleA(std::ptr::null()),
                        std::ptr::null_mut(),
                    )
                };

                ret!(
                    tx_ret,
                    waker,
                    if hwnd != std::ptr::null_mut() {
                        Ok(hwnd as usize)
                    } else {
                        Err(Win32Error::from_last_error())
                    }
                )
            }
            CallerFn::DestroyWindow { args: (hwnd,), tx_ret, waker } => {
                ret!(
                    tx_ret,
                    waker,
                    if unsafe { DestroyWindow(hwnd as _) } == TRUE {
                        Ok(())
                    } else {
                        Err(Win32Error::from_last_error())
                    }
                )
            }

            // XingAPI
            CallerFn::Connect {
                args: (hwnd, addr, port, timeout, max_packet_size),
                tx_ret,
                waker,
            } => {
                ret!(tx_ret, waker, entry.connect(hwnd, &addr, port, timeout, max_packet_size))
            }
            CallerFn::IsConnected { args: (), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.is_connected())
            }
            CallerFn::Disconnect { args: (), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.disconnect())
            }
            CallerFn::Login { args: (hwnd, id, pw, cert_pw, cert_err_dialog), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.login(hwnd, &id, &pw, &cert_pw, cert_err_dialog))
            }
            CallerFn::Request {
                args: (hwnd, tr_code, data, continue_key, timeout),
                tx_ret,
                waker,
            } => {
                let ret = entry.request(hwnd, &tr_code, &data, continue_key.as_deref(), timeout);
                ret!(tx_ret, waker, ret)
            }
            CallerFn::AdviseRealData { args: (hwnd, tr_code, data), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.advise_real_data(hwnd, &tr_code, &data))
            }
            CallerFn::UnadviseRealData { args: (hwnd, tr_code, data), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.unadvise_real_data(hwnd, &tr_code, &data))
            }
            CallerFn::UnadviseWindow { args: (hwnd,), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.unadvise_window(hwnd))
            }
            CallerFn::GetAccountList { args: (), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_account_list())
            }
            CallerFn::GetAccountName { args: (account,), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_account_name(&account))
            }
            CallerFn::GetAccountDetailName { args: (account,), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_account_detail_name(&account))
            }
            CallerFn::GetAccountNickname { args: (account,), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_account_nickname(&account))
            }
            CallerFn::GetClientIp { args: (), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_client_ip())
            }
            CallerFn::GetServerName { args: (), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_server_name())
            }
            CallerFn::GetApiPath { args: (), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_api_path())
            }
            CallerFn::GetTrCountPerSec { args: (tr_code,), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_tr_count_per_sec(&tr_code))
            }
            CallerFn::GetTrCountBaseSec { args: (tr_code,), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_tr_count_base_sec(&tr_code))
            }
            CallerFn::GetTrCountRequest { args: (tr_code,), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_tr_count_request(&tr_code))
            }
            CallerFn::GetTrCountLimit { args: (tr_code,), tx_ret, waker } => {
                ret!(tx_ret, waker, entry.get_tr_count_limit(&tr_code))
            }
        }
    }
}

impl Drop for Caller {
    fn drop(&mut self) {
        self.tx_quit.try_send(()).unwrap();
        self.join_handle.take().unwrap().join().unwrap();
    }
}

impl UnwindSafe for Caller {}
impl RefUnwindSafe for Caller {}

#[cfg(test)]
mod tests {
    use super::Caller;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_load_caller() -> Result<(), Box<dyn std::error::Error>> {
        let caller = Arc::new(Caller::new(None)?);
        println!("api_path: {:?}", caller.handle().read().await.get_api_path().await);
        assert!(!caller.handle().read().await.is_connected().await);

        Ok(())
    }
}
