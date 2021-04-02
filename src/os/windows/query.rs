// SPDX-License-Identifier: MPL-2.0

use super::{
    bindings,
    caller::Caller,
    raw::{MSG_PACKET, RECV_PACKET, XM_RECEIVE_DATA, XM_TIMEOUT},
    window::Window,
};
use crate::{
    data::{self, Data},
    error::{Error, Win32Error},
    euckr,
    response::QueryResponse,
};
use xingapi_res::TrLayout;

use array_init::array_init;
use lazy_static::lazy_static;

use std::{
    collections::HashMap,
    ops::DerefMut,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc, Weak,
    },
};

use bindings::{
    DefWindowProcA, GetModuleHandleA, GetWindowLongPtrA, RegisterClassExA, SetWindowLongPtrA,
    GWLP_USERDATA, HWND, LPARAM, LRESULT, UINT, WM_DESTROY, WNDCLASSEXA, WPARAM,
};

lazy_static! {
    static ref QUERY_WNDCLASS: Vec<i8> = {
        let class_name: Vec<i8> =
            b"xingapi::query::QUERY_WNDCLASS\0".iter().map(|&c| c as i8).collect();

        unsafe {
            RegisterClassExA(&WNDCLASSEXA {
                cbSize: std::mem::size_of::<WNDCLASSEXA>() as UINT,
                lpfnWndProc: Some(QueryWindow::wndproc),
                cbWndExtra: std::mem::size_of::<usize>() as _,
                hInstance: GetModuleHandleA(std::ptr::null()),
                lpszClassName: class_name.as_ptr(),
                ..std::mem::zeroed()
            });
        }

        class_name
    };
}

type TxResponse = async_channel::Sender<Option<IncompleteResponse>>;

struct IncompleteResponse {
    code: String,
    message: String,
    elapsed_time: i32,
    continue_key: Option<String>,
    data_recv: bool,
    block_data: HashMap<String, Vec<u8>>,
    non_block_data: Option<Vec<u8>>,
}

impl IncompleteResponse {
    fn empty() -> Self {
        Self {
            code: String::new(),
            message: String::new(),
            elapsed_time: 0,
            continue_key: None,
            data_recv: false,
            block_data: HashMap::new(),
            non_block_data: None,
        }
    }
}

struct WindowData {
    caller: Weak<Caller>,
    tr_layouts: Weak<HashMap<String, TrLayout>>,
    res_map: [Option<IncompleteResponse>; 256],
    tx_res_map: [async_lock::Mutex<Option<TxResponse>>; 256],
}

impl WindowData {
    fn new(
        window: &Window,
        caller: &Arc<Caller>,
        tr_layouts: &Arc<HashMap<String, TrLayout>>,
    ) -> AtomicPtr<Self> {
        let mut data = AtomicPtr::new(Box::into_raw(Box::new(WindowData {
            caller: Arc::downgrade(&caller),
            tr_layouts: Arc::downgrade(&tr_layouts),
            res_map: array_init(|_| None),
            tx_res_map: array_init(|_| async_lock::Mutex::new(None)),
        })));

        unsafe {
            SetWindowLongPtrA(**window as _, GWLP_USERDATA, *data.get_mut() as _);
        }

        data
    }
}

pub struct QueryWindow {
    caller: Arc<Caller>,
    tr_layouts: Arc<HashMap<String, TrLayout>>,
    window: Window,
    window_data: AtomicPtr<WindowData>,
}

impl QueryWindow {
    fn tx_res<'a>(&'a self, req_id: i32) -> &'a async_lock::Mutex<Option<TxResponse>> {
        &unsafe { &*self.window_data.load(Ordering::Relaxed) }.tx_res_map[req_id as usize]
    }

    pub(crate) async fn new(
        caller: Arc<Caller>,
        tr_layouts: Arc<HashMap<String, TrLayout>>,
    ) -> Result<Self, Win32Error> {
        let window = Window::new(caller.clone(), &QUERY_WNDCLASS).await?;
        let window_data = WindowData::new(&window, &caller, &tr_layouts);

        Ok(Self { caller, tr_layouts, window, window_data })
    }

    pub async fn request(
        &self,
        data: &Data,
        continue_key: Option<&str>,
        timeout: Option<i32>,
    ) -> Result<QueryResponse, Error> {
        let handle = self.caller.handle().read().await;
        let tr_code = &data.code;
        let req_id = handle
            .request(
                *self.window,
                tr_code,
                data::encode(&self.tr_layouts, &data)?,
                continue_key,
                timeout,
            )
            .await?;

        let (tx_res, rx_res) = async_channel::bounded(1);

        let mut tx_res_ref = self.tx_res(req_id).lock().await;
        assert!(tx_res_ref.is_none());
        *tx_res_ref.deref_mut() = Some(tx_res);
        drop(tx_res_ref);

        if let Some(res) = rx_res.recv().await.unwrap() {
            let data = if res.data_recv {
                Some(data::decode(&self.tr_layouts, tr_code, res.block_data, res.non_block_data))
            } else {
                None
            };

            Ok(QueryResponse::new(
                &res.code,
                &res.message,
                res.elapsed_time,
                res.continue_key,
                data,
            ))
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
                let window_data = GetWindowLongPtrA(hwnd, GWLP_USERDATA) as *mut WindowData;
                assert_ne!(window_data, std::ptr::null_mut());
                drop(Box::from_raw(window_data));

                0
            }
            XM_RECEIVE_DATA | XM_TIMEOUT => {
                Self::handle_xingapi_msg(hwnd, msg, wparam, lparam);
                0
            }
            _ => DefWindowProcA(hwnd, msg, wparam, lparam),
        }
    }

    #[inline(always)]
    fn handle_xingapi_msg(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) {
        let window_data = unsafe {
            let ptr = GetWindowLongPtrA(hwnd, GWLP_USERDATA) as *mut WindowData;
            assert_ne!(ptr, std::ptr::null_mut());
            &mut *ptr
        };

        macro_rules! acquire_tx_res {
            ($window_data:ident, $req_id:ident) => {
                loop {
                    if let Some(tx) = $window_data.tx_res_map[$req_id as usize].try_lock() {
                        if tx.is_some() {
                            break tx;
                        }
                    }
                    std::hint::spin_loop();
                }
            };
        }

        // req = request, res = response
        match msg {
            XM_RECEIVE_DATA => {
                let caller = unsafe { &*window_data.caller.as_ptr() };
                let layout_map = unsafe { &*window_data.tr_layouts.as_ptr() };

                let req_id = match wparam {
                    1 => unsafe { &*(lparam as *const RECV_PACKET) }.req_id,
                    2 | 3 => unsafe { &*(lparam as *const MSG_PACKET) }.req_id,
                    4 => lparam as _,
                    _ => unreachable!(),
                };

                if window_data.res_map[req_id as usize].is_none() {
                    window_data.res_map[req_id as usize] = Some(IncompleteResponse::empty());
                }

                // RECV_PACKET보다 MSG_PACKET이 먼저 오는 경우도 있습니다.
                match wparam {
                    1 => {
                        let recv_packet = unsafe { &*(lparam as *const RECV_PACKET) };
                        let tr_code = euckr::decode(&recv_packet.tr_code);
                        let continue_key = euckr::decode(&recv_packet.continue_key);

                        let res = window_data.res_map[req_id as usize].as_mut().unwrap();

                        if res.elapsed_time < recv_packet.elapsed_time {
                            res.elapsed_time = recv_packet.elapsed_time;
                        }

                        if !continue_key.is_empty() && res.continue_key.is_none() {
                            res.continue_key = Some(continue_key.to_string());
                        }

                        let raw_data = unsafe {
                            std::slice::from_raw_parts(recv_packet.data, recv_packet.data_len as _)
                                .to_owned()
                        };

                        if let Some(tr_layout) = layout_map.get(tr_code.as_ref()) {
                            res.data_recv = true;

                            if tr_layout.block {
                                let block_name = euckr::decode(&recv_packet.block_name).to_string();
                                res.block_data.insert(block_name, raw_data);
                            } else {
                                if res.non_block_data.is_none() {
                                    res.non_block_data = Some(raw_data)
                                }
                            }
                        }
                    }
                    2 => {
                        let msg_packet = unsafe { &*(lparam as *const MSG_PACKET) };

                        let res = window_data.res_map[req_id as usize].as_mut().unwrap();
                        res.code = euckr::decode(&msg_packet.msg_code).to_string();
                        res.message = euckr::decode(unsafe {
                            std::slice::from_raw_parts(
                                msg_packet.msg_data,
                                msg_packet.msg_data_len as usize,
                            )
                        })
                        .to_string();

                        caller.entry().release_message_data(lparam);
                    }
                    3 => {
                        caller.entry().release_message_data(lparam);
                    }
                    4 => {
                        let res = window_data.res_map[req_id as usize].take().unwrap();
                        let tx_res = acquire_tx_res!(window_data, req_id).take().unwrap();
                        let _ = tx_res.try_send(Some(res));

                        caller.entry().release_request_data(req_id);
                    }
                    _ => unreachable!(),
                }
            }
            XM_TIMEOUT => {
                let req_id = lparam as i32;
                let tx_res = acquire_tx_res!(window_data, req_id).take().unwrap();

                window_data.res_map[req_id as usize] = None;
                let _ = tx_res.try_send(None);
            }
            _ => unreachable!(),
        }
    }
}

impl UnwindSafe for QueryWindow {}
impl RefUnwindSafe for QueryWindow {}
