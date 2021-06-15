// SPDX-License-Identifier: MPL-2.0

use encoding_rs::EUC_KR;
use std::borrow::Cow;

// EUC-KR(CP949)로 인코딩된 null-terminated 문자열을
// 포인터에서 참조하여 UTF-8 문자열로 인코딩합니다.
pub unsafe fn decode_ptr(data: *const u8) -> String {
    let len;
    let mut i = data;
    loop {
        if *i == b'\0' {
            len = i as usize - data as usize;
            break;
        }
        i = (i as usize + 1) as *const u8;
    }

    EUC_KR.decode(std::slice::from_raw_parts(data, len)).0.to_string()
}

// EUC-KR(CP949)로 인코딩된 null-terminated 문자열을 UTF-8 문자열로 디코딩합니다.
pub fn decode(data: &[u8]) -> Cow<str> {
    let mut len = data.len();
    for (i, &ch) in data.iter().enumerate() {
        if ch == b'\0' {
            len = i;
            break;
        }
    }

    EUC_KR.decode(&data[0..len]).0
}

// UTF-8 문자열을 EUC-KR(CP949)로 인코딩된 null-terminated 문자열로 인코딩합니다.
pub fn encode(text: &str) -> Vec<u8> {
    let mut text_encoded = EUC_KR.encode(text).0.to_vec();
    text_encoded.push(0);

    text_encoded
}
