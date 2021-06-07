// SPDX-License-Identifier: MPL-2.0

//! [`Data`](crate::data::Data)를 인코딩하거나 디코딩할 때 발생하는 에러에 대한 모듈입니다.
//!
//! 이러한 에러들은 다운로드 받은 RES 파일이 오래되어 서버와 달라 발생할 수도 있습니다.

/// 응답 데이터를 디코딩할 때 발생하는 에러입니다.
#[derive(Clone, Debug)]
pub enum DecodeError {
    /// TR 코드를 불러오지 않았거나 알 수 없습니다.
    UnknownTrCode,
    /// 블록이 누락되었습니다.
    UnknownBlockName { block_name: String },
    /// 데이터 크기가 일치하지 않습니다.
    MismatchBufferLength,
    /// 배열 크기를 디코딩할 수 없습니다.
    DecodeOccursLength,
    /// CP949 데이터를 디코딩할 수 없습니다.
    DecodeString,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTrCode => write!(f, "unknown TR code"),
            Self::UnknownBlockName { block_name } => {
                write!(f, "unknown block name: {}", block_name)
            }
            Self::MismatchBufferLength => write!(f, "mismatch buffer length"),

            Self::DecodeOccursLength => write!(f, "invalid array length"),
            Self::DecodeString => write!(f, "invalid EUC-KR string"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// 요청 데이터를 인코딩할 때 발생하는 에러입니다.
#[derive(Clone, Debug)]
pub enum EncodeError {
    /// TR 코드를 불러오지 않았거나 알 수 없습니다.
    UnknownTrCode,
    /// 블록이 누락되었습니다.
    MissingBlock { block_name: String },
    /// 블록 타입이 일치하지 않습니다.
    MismatchBlockType { block_name: String },
    /// 배열의 최대 크기에 도달했습니다. 배열의 크기는 5자리 이하여야 합니다.
    ExceedMaxBlockCount { block_name: String },
    /// 필드가 누락되었습니다.
    MissingField { block_name: String, field_name: String },
    /// 필드의 최대 크기에 도달했습니다.
    ExceedFieldLength { block_name: String, field_name: String },
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTrCode => write!(f, "unknown tr code"),
            Self::MissingBlock { block_name } => write!(f, "missing block: {}", block_name),
            Self::MismatchBlockType { block_name } => {
                write!(f, "mismatch block type: {}", block_name)
            }
            Self::MissingField { block_name, field_name } => {
                write!(f, "missing field: {} {}", block_name, field_name)
            }
            Self::ExceedMaxBlockCount { block_name } => {
                write!(f, "the max length of array reached: {}", block_name)
            }
            Self::ExceedFieldLength { block_name, field_name } => {
                write!(f, "the max length of field reached: {} {}", block_name, field_name)
            }
        }
    }
}

impl std::error::Error for EncodeError {}
