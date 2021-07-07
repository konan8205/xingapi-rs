// SPDX-License-Identifier: MPL-2.0

use super::{
    caller::Caller,
    raw::{RECV_REAL_PACKET, XM_RECEIVE_REAL_DATA},
    window::Window,
};
use crate::{
    data::{self, error::DecodeError},
    error::Win32Error,
    euckr,
    response::RealResponse,
};

use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicPtr, Arc},
};
use xingapi_res::TrLayout;

use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::winuser::{
    DefWindowProcA, GetWindowLongPtrA, RegisterClassExA, SetWindowLongPtrA, GWLP_USERDATA,
    WM_DESTROY, WNDCLASSEXA,
};

lazy_static! {
    static ref REAL_WNDCLASS: Vec<i8> = {
        let class_name: Vec<i8> =
            b"xingapi::real::REAL_WNDCLASS\0".iter().map(|&c| c as i8).collect();

        unsafe {
            RegisterClassExA(&WNDCLASSEXA {
                cbSize: std::mem::size_of::<WNDCLASSEXA>() as UINT,
                lpfnWndProc: Some(RealWindow::wndproc),
                cbWndExtra: std::mem::size_of::<usize>() as _,
                hInstance: GetModuleHandleA(std::ptr::null()),
                lpszClassName: class_name.as_ptr(),
                ..std::mem::zeroed()
            });
        }

        class_name
    };
}

struct IncompleteResponse {
    tr_code: String,
    key: String,
    reg_key: String,
    data: Vec<u8>,
}

struct WindowData {
    tx_res: async_channel::Sender<IncompleteResponse>,
}

impl WindowData {
    fn new(window: &Window, tx_res: async_channel::Sender<IncompleteResponse>) -> AtomicPtr<Self> {
        let mut data = AtomicPtr::new(Box::into_raw(Box::new(WindowData { tx_res })));
        unsafe {
            SetWindowLongPtrA(**window as _, GWLP_USERDATA, *data.get_mut() as _);
        }

        data
    }
}

pub struct RealWindow {
    caller: Arc<Caller>,
    tr_layouts: Arc<HashMap<String, TrLayout>>,
    window: Window,
    _window_data: AtomicPtr<WindowData>,
    rx_res: async_channel::Receiver<IncompleteResponse>,
}

impl RealWindow {
    pub async fn new(
        caller: Arc<Caller>,
        tr_layouts: Arc<HashMap<String, TrLayout>>,
    ) -> Result<Self, Win32Error> {
        let window = Window::new(caller.clone(), &REAL_WNDCLASS).await?;

        let (tx_res, rx_res) = async_channel::unbounded();
        let _window_data = WindowData::new(&window, tx_res);

        Ok(Self { caller, tr_layouts, window, _window_data, rx_res })
    }

    pub async fn subscribe(&self, tr_code: &str, data: Vec<String>) -> Result<(), ()> {
        let handle = self.caller.handle().read().await;
        handle.advise_real_data(*self.window, tr_code, data).await
    }

    pub async fn unsubscribe(&self, tr_code: &str, data: Vec<String>) -> Result<(), ()> {
        let handle = self.caller.handle().read().await;
        handle.unadvise_real_data(*self.window, tr_code, data).await
    }

    pub async fn unsubscribe_all(&self) -> Result<(), ()> {
        self.caller.unadvise_window(*self.window).await
    }

    pub async fn recv(&self) -> RealResponse {
        let res = self.rx_res.recv().await.unwrap();
        RealResponse::new(
            res.key,
            res.reg_key,
            if let Some(layout) = self.tr_layouts.get(&res.tr_code) {
                data::decode_non_block(layout, &res.data)
            } else {
                Err(DecodeError::UnknownLayout)
            },
        )
    }

    pub fn try_recv(&self) -> Option<RealResponse> {
        if let Ok(res) = self.rx_res.try_recv() {
            Some(RealResponse::new(
                res.key,
                res.reg_key,
                if let Some(layout) = self.tr_layouts.get(&res.tr_code) {
                    data::decode_non_block(layout, &res.data)
                } else {
                    Err(DecodeError::UnknownLayout)
                },
            ))
        } else {
            None
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
                let window_data = GetWindowLongPtrA(hwnd, GWLP_USERDATA) as *mut WindowData;
                assert_ne!(window_data, std::ptr::null_mut());
                drop(Box::from_raw(window_data));

                0
            }
            XM_RECEIVE_REAL_DATA => {
                let window_data = {
                    let ptr = GetWindowLongPtrA(hwnd, GWLP_USERDATA) as *mut WindowData;
                    assert_ne!(ptr, std::ptr::null_mut());
                    &mut *ptr
                };

                let packet = &*(lparam as *const RECV_REAL_PACKET);
                let _ = window_data.tx_res.try_send(IncompleteResponse {
                    tr_code: euckr::decode(&packet.tr_code).to_string(),
                    key: euckr::decode(&packet.key).to_string(),
                    reg_key: euckr::decode(&packet.reg_key).to_string(),
                    data: std::slice::from_raw_parts(packet.data, packet.data_len as _).into(),
                });

                0
            }
            _ => DefWindowProcA(hwnd, msg, wparam, lparam),
        }
    }
}

impl Drop for RealWindow {
    fn drop(&mut self) {
        self.caller.unadvise_window(*self.window);
    }
}
