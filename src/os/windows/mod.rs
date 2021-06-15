// SPDX-License-Identifier: MPL-2.0

mod caller;
mod entry;
mod raw;
mod window;

mod query;
mod real;
mod session;

mod bindings {
    pub use winapi::{
        ctypes::{c_int, c_void},
        shared::{
            minwindef::{BOOL, DWORD, FALSE, LPARAM, LRESULT, TRUE, UINT, WPARAM},
            windef::HWND,
        },
        um::{
            errhandlingapi::GetLastError,
            libloaderapi::GetModuleHandleA,
            winuser::{
                CreateWindowExA, DefWindowProcA, DestroyWindow, DispatchMessageA,
                GetWindowLongPtrA, PeekMessageA, RegisterClassExA, SetWindowLongPtrA,
                TranslateMessage, GWLP_USERDATA, HWND_MESSAGE, MSG, PM_REMOVE, WM_DESTROY, WM_USER,
                WNDCLASSEXA,
            },
        },
    };
}

use self::{caller::Caller, query::QueryWindow, real::RealWindow, session::SessionWindow};
use crate::{
    data::Data,
    error::Error,
    response::{LoginResponse, QueryResponse, RealResponse},
    Account,
};

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use xingapi_res::TrLayout;

pub struct XingApiBuilder {
    path: Option<PathBuf>,
    tr_layouts: Option<HashMap<String, TrLayout>>,
}

impl XingApiBuilder {
    pub fn new() -> Self {
        Self { path: None, tr_layouts: None }
    }

    pub fn path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = Some(path.as_ref().to_owned());
        self
    }

    pub fn layouts<I>(mut self, layouts: I) -> Self
    where
        I: IntoIterator<Item = TrLayout>,
    {
        if self.tr_layouts.is_none() {
            self.tr_layouts = Some(HashMap::new());
        }

        let tr_layouts = self.tr_layouts.as_mut().unwrap();
        tr_layouts.extend(layouts.into_iter().map(|tr| (tr.code.to_owned(), tr)));
        self
    }

    pub async fn build(self) -> Result<Arc<XingApi>, Error> {
        let caller = Arc::new(Caller::new(self.path.as_deref())?);

        let tr_layouts =
            Arc::new(if let Some(val) = self.tr_layouts { val } else { xingapi_res::load()? });

        let session_window = SessionWindow::new(caller.clone()).await?;
        let query_window = QueryWindow::new(caller.clone(), tr_layouts.clone()).await?;

        Ok(Arc::new(XingApi { caller, tr_layouts, session_window, query_window }))
    }
}

pub struct XingApi {
    caller: Arc<Caller>,
    tr_layouts: Arc<HashMap<String, TrLayout>>,
    session_window: SessionWindow,
    query_window: QueryWindow,
}

impl XingApi {
    pub async fn new() -> Result<Arc<Self>, Error> {
        let caller = Arc::new(Caller::new(None)?);
        let tr_layouts = Arc::new(xingapi_res::load()?);

        let session_window = SessionWindow::new(caller.clone()).await?;
        let query_window = QueryWindow::new(caller.clone(), tr_layouts.clone()).await?;

        Ok(Arc::new(XingApi { caller, tr_layouts, session_window, query_window }))
    }

    pub async fn connect(
        &self,
        addr: &str,
        port: u16,
        timeout: Option<i32>,
        max_packet_size: Option<i32>,
    ) -> Result<(), Error> {
        self.session_window.connect(addr, port, timeout, max_packet_size).await
    }

    pub async fn is_connected(&self) -> bool {
        self.caller.handle().read().await.is_connected().await
    }

    pub async fn disconnect(&self) {
        self.caller.handle().write().await.disconnect().await
    }

    pub async fn login(
        &self,
        id: &str,
        pw: &str,
        cert_pw: &str,
        cert_err_dialog: bool,
    ) -> Result<LoginResponse, Error> {
        self.session_window.login(id, pw, cert_pw, cert_err_dialog).await
    }

    pub async fn request(
        &self,
        data: &Data,
        continue_key: Option<&str>,
        timeout: Option<i32>,
    ) -> Result<QueryResponse, Error> {
        self.query_window.request(data, continue_key, timeout).await
    }

    pub async fn accounts(&self) -> Vec<Account> {
        let handle = self.caller.handle().read().await;
        let codes = handle.get_account_list().await;

        let mut accounts = Vec::with_capacity(codes.len());
        for code in codes {
            let name = handle.get_account_name(&code).await;
            let detail_name = handle.get_account_detail_name(&code).await;
            let nickname = handle.get_account_nickname(&code).await;

            accounts.push(Account { code, name, detail_name, nickname });
        }

        accounts
    }

    pub async fn client_ip(&self) -> String {
        self.caller.handle().read().await.get_client_ip().await
    }

    pub async fn server_name(&self) -> String {
        self.caller.handle().read().await.get_server_name().await
    }

    pub async fn path(&self) -> String {
        self.caller.handle().read().await.get_api_path().await
    }

    pub async fn limit_per_one_sec(&self, tr_code: &str) -> i32 {
        self.caller.handle().read().await.get_tr_count_per_sec(tr_code).await
    }

    pub async fn limit_sec_per_once(&self, tr_code: &str) -> i32 {
        self.caller.handle().read().await.get_tr_count_base_sec(tr_code).await
    }

    pub async fn count_in_ten_min(&self, tr_code: &str) -> i32 {
        self.caller.handle().read().await.get_tr_count_request(tr_code).await
    }

    pub async fn limit_per_ten_min(&self, tr_code: &str) -> i32 {
        self.caller.handle().read().await.get_tr_count_limit(tr_code).await
    }
}

pub struct Real {
    window: RealWindow,
}

impl Real {
    pub async fn new(xingapi: Arc<XingApi>) -> Result<Self, Error> {
        Ok(Self {
            window: RealWindow::new(xingapi.caller.clone(), xingapi.tr_layouts.clone()).await?,
        })
    }

    pub async fn subscribe(&self, tr_code: &str, data: Vec<String>) -> Result<(), ()> {
        self.window.subscribe(tr_code, data).await
    }

    pub async fn unsubscribe(&self, tr_code: &str, data: Vec<String>) -> Result<(), ()> {
        self.window.unsubscribe(tr_code, data).await
    }

    pub async fn unsubscribe_all(&self) -> Result<(), ()> {
        self.window.unsubscribe_all().await
    }

    pub async fn recv(&self) -> RealResponse {
        self.window.recv().await
    }

    pub fn try_recv(&self) -> Option<RealResponse> {
        self.window.try_recv()
    }
}
