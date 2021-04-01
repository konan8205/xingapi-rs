// SPDX-License-Identifier: MPL-2.0

use super::caller::Caller;
use crate::error::Win32Error;

use std::{ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct Window {
    caller: Arc<Caller>,
    hwnd: usize,
}

impl Window {
    pub async fn new(caller: Arc<Caller>, class_name: &[i8]) -> Result<Self, Win32Error> {
        // 힙 영역 메모리 할당을 먼저 합니다.
        let mut window = Window { caller: caller.clone(), hwnd: 0 };
        let hwnd = caller.create_window(class_name).await?;

        // 윈도우 생성 이후 패닉이 발생하면 윈도우가 소멸되지 않기 때문입니다.
        if hwnd != 0 {
            window.hwnd = hwnd;
            Ok(window)
        } else {
            Err(Win32Error::from_last_error())
        }
    }
}

impl Deref for Window {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.hwnd
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.caller.destroy_window(self.hwnd);
    }
}
