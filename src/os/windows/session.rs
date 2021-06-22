// SPDX-License-Identifier: MPL-2.0

use super::{
    caller::Caller,
    raw::{XM_DISCONNECT, XM_LOGIN, XM_LOGOUT},
    window::Window,
};
use crate::{
    error::{Error, Win32Error},
    euckr,
    response::LoginResponse,
};

use async_channel::Sender;
use async_lock::RwLock;
use lazy_static::lazy_static;
use std::{
    panic::{RefUnwindSafe, UnwindSafe},
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc,
    },
};

use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::winuser::{
    DefWindowProcA, GetWindowLongPtrA, RegisterClassExA, SetWindowLongPtrA, GWLP_USERDATA,
    WM_DESTROY, WNDCLASSEXA,
};

lazy_static! {
    static ref SESSION_WNDCLASS: Vec<i8> = {
        let class_name: Vec<i8> =
            b"xingapi::session::SESSION_WNDCLASS\0".iter().map(|&c| c as i8).collect();

        unsafe {
            RegisterClassExA(&WNDCLASSEXA {
                cbSize: std::mem::size_of::<WNDCLASSEXA>() as UINT,
                lpfnWndProc: Some(SessionWindow::wndproc),
                cbWndExtra: std::mem::size_of::<usize>() as _,
                hInstance: GetModuleHandleA(std::ptr::null()),
                lpszClassName: class_name.as_ptr(),
                ..std::mem::zeroed()
            });
        }

        class_name
    };
}

struct WindowData {
    tx_res: Option<Sender<Option<LoginResponse>>>,
}

impl WindowData {
    fn empty() -> Self {
        WindowData { tx_res: None }
    }

    fn new(window: &Window) -> AtomicPtr<RwLock<Self>> {
        let data = RwLock::new(WindowData::empty());
        let mut ptr = AtomicPtr::new(Box::into_raw(Box::new(data)));
        unsafe {
            SetWindowLongPtrA(**window as _, GWLP_USERDATA, *ptr.get_mut() as _);
        }

        ptr
    }
}

pub struct SessionWindow {
    caller: Arc<Caller>,
    window: Window,
    window_data: AtomicPtr<RwLock<WindowData>>,
}

impl SessionWindow {
    pub(crate) async fn new(caller: Arc<Caller>) -> Result<Self, Win32Error> {
        let window = Window::new(caller.clone(), &SESSION_WNDCLASS).await?;
        let window_data = WindowData::new(&window);

        Ok(Self { caller, window, window_data })
    }

    pub async fn connect(
        &self,
        addr: &str,
        port: u16,
        timeout: Option<i32>,
        max_packet_size: Option<i32>,
    ) -> Result<(), Error> {
        let handle = self.caller.handle().write().await;
        handle.connect(*self.window, addr, port, timeout, max_packet_size).await
    }

    pub async fn login(
        &self,
        id: &str,
        pw: &str,
        cert_pw: &str,
        cert_err_dialog: bool,
    ) -> Result<LoginResponse, Error> {
        let handle = self.caller.handle().write().await;
        let window_data = unsafe { &mut *self.window_data.load(Ordering::Relaxed) };

        let (tx_res, rx_res) = async_channel::unbounded();
        *window_data.write().await = WindowData { tx_res: Some(tx_res) };

        handle.login(*self.window, id, pw, cert_pw, cert_err_dialog).await?;
        let result = rx_res.recv().await.unwrap();

        *window_data.write().await = WindowData::empty();

        if let Some(res) = result {
            Ok(res)
        } else {
            Err(Error::TimedOut)
        }
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        debug_assert!(Caller::is_caller_thread());

        match msg {
            WM_DESTROY => {
                let window_data = GetWindowLongPtrA(hwnd, GWLP_USERDATA) as *mut RwLock<WindowData>;
                assert_ne!(window_data, std::ptr::null_mut());
                drop(Box::from_raw(window_data));

                0
            }
            XM_DISCONNECT | XM_LOGIN | XM_LOGOUT => {
                Self::on_recv_msg(hwnd, msg, wparam, lparam);
                0
            }
            _ => DefWindowProcA(hwnd, msg, wparam, lparam),
        }
    }

    #[inline(always)]
    fn on_recv_msg(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) {
        let window_data_lock = unsafe {
            let ptr = GetWindowLongPtrA(hwnd, GWLP_USERDATA) as *const RwLock<WindowData>;
            assert_ne!(ptr, std::ptr::null());
            &*ptr
        };

        let window_data = loop {
            if let Some(data) = window_data_lock.try_read() {
                break data;
            }
            std::hint::spin_loop();
        };

        match msg {
            XM_DISCONNECT => {
                if let Some(tx) = &window_data.tx_res {
                    let _ = tx.try_send(None);
                }
            }
            XM_LOGIN => {
                if let Some(tx) = &window_data.tx_res {
                    let code = unsafe { euckr::decode_ptr(wparam as *const u8) };
                    let message = unsafe { euckr::decode_ptr(lparam as *const u8) };
                    let _ = tx.try_send(Some(LoginResponse::new(&code, &message)));
                }
            }
            XM_LOGOUT => {}
            _ => unreachable!(),
        }
    }
}

impl UnwindSafe for SessionWindow {}
impl RefUnwindSafe for SessionWindow {}
