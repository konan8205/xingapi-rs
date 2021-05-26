// SPDX-License-Identifier: MPL-2.0

//! XingAPI의 요청 및 응답 데이터에 대한 레이아웃인 RES 파일에 대한 파서입니다.

#![cfg_attr(doc_cfg, feature(doc_cfg))]

pub mod error;
mod layout;
mod read;

pub use error::{Error, ErrorKind, LoadError};
pub use layout::{BlockLayout, BlockType, FieldLayout, FieldType, HeaderType, TrLayout, TrType};

use encoding_rs::EUC_KR;
use std::{collections::HashMap, ffi::OsStr, fs, path::Path, thread};

/// 시스템에 설치된 XingAPI의 기본 경로로 TR 레이아웃을 모두 불러옵니다.
#[cfg(any(windows, doc))]
#[cfg_attr(doc_cfg, doc(cfg(windows)))]
pub fn load() -> Result<HashMap<String, TrLayout>, LoadError> {
    load_from_path(Path::new("C:\\eBEST\\xingAPI\\Res"))
}

/// 지정된 경로로 TR 레이아웃을 모두 불러옵니다.
pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<HashMap<String, TrLayout>, LoadError> {
    let mut tasks = Vec::new();

    for ent in fs::read_dir(&path)? {
        let file_path = ent?.path();
        if file_path.extension() != Some(OsStr::new("res")) {
            continue;
        }

        tasks.push(thread::spawn(move || -> Result<TrLayout, LoadError> {
            let raw_data = fs::read(&file_path)?;
            let (data, _, had_errors) = EUC_KR.decode(&raw_data);
            if had_errors {
                return Err(LoadError::Decode(file_path));
            }

            let res = data.parse::<TrLayout>().map_err(|err| LoadError::Parse(file_path, err))?;
            Ok(res)
        }));
    }

    let mut res_map: HashMap<String, TrLayout> = HashMap::new();
    for task in tasks {
        let res = task.join().unwrap()?;

        if let Some(other_res) = res_map.get(&res.code) {
            if res != *other_res {
                return Err(LoadError::Confilict(res.code.to_owned()));
            }
        } else {
            res_map.insert(res.code.to_owned(), res);
        }
    }

    Ok(res_map)
}
