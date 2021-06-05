// SPDX-License-Identifier: MPL-2.0

//! 파싱 및 RES 파일 불러오기에 대한 에러 모듈입니다.

use crate::read::{Position, Read};
use std::{fmt, path::PathBuf};

/// 파싱에 실패했을 때 발생하는 에러입니다.
#[derive(Debug)]
pub struct Error {
    line: usize,
    column: usize,
    kind: ErrorKind,
}

impl Error {
    /// 에러가 발생한 행 위치를 반환합니다.
    pub fn line(&self) -> usize {
        self.line
    }
    /// 에러가 발생한 열 위치를 반환합니다.
    pub fn column(&self) -> usize {
        self.column
    }
    /// 에러 종류를 반환합니다.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub(crate) fn new(pos: &Position, kind: ErrorKind) -> Self {
        Self { line: pos.line(), column: pos.column(), kind }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)?;
        write!(f, " at line {} column {}", self.line, self.column)
    }
}

impl std::error::Error for Error {}

/// 에러 종류에 대한 열거형 객체입니다.
#[derive(PartialEq, Debug)]
pub enum ErrorKind {
    /// 구문 오류
    Syntax,
    /// 예상치 못한 파일 끝 오류
    Eof,
    /// TR 인수 개수 오류
    TrParamCount,
    /// TR 인수 오류
    TrParam,
    /// block 인수 개수 오류
    BlockParamCount,
    /// block 인수 오류
    BlockParam,
    /// field 인수 개수 오류
    FieldParamCount,
    /// field 인수 오류
    FieldParam,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax => write!(f, "unexpected syntax"),
            Self::Eof => write!(f, "unexpected EOF"),
            Self::TrParamCount => write!(f, "unexpected tr parameter count"),
            Self::TrParam => write!(f, "unexpected tr parameter"),
            Self::BlockParamCount => write!(f, "unexpected block parameter count"),
            Self::BlockParam => write!(f, "unexpected block parameter"),
            Self::FieldParamCount => write!(f, "unexpected field parameter count"),
            Self::FieldParam => write!(f, "unexpected field parameter"),
        }
    }
}

pub(crate) fn unexpected_syntax<'a, R>(reader: &R) -> Error
where
    R: Read<'a>,
{
    Error::new(&reader.position(), ErrorKind::Syntax).into()
}

pub(crate) fn unexpected_eof<'a, R>(reader: &R) -> Error
where
    R: Read<'a>,
{
    Error::new(&reader.position(), ErrorKind::Eof).into()
}

/// RES 파일 불러오기에 실패하면 발생하는 에러입니다.
#[derive(Debug)]
pub enum LoadError {
    /// 입출력 오류가 발생했습니다.
    Io(std::io::Error),
    /// 파일을 CP949에서 UTF-8로 디코딩하지 못했습니다.
    Decode(PathBuf),
    /// 파일 파싱에 실패했습니다.
    Parse(PathBuf, Error),
    /// 코드는 같지만 파싱 결과가 동일하지 않은 여러 파일이 있습니다.
    Confilict(String),
}

impl From<std::io::Error> for LoadError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => err.fmt(f),
            Self::Decode(path) => {
                write!(f, "unable to decode file from CP949: {}", path.to_string_lossy())
            }
            Self::Parse(path, err) => {
                write!(f, "unable to parse file: {}, path: {}", path.to_string_lossy(), err)
            }
            Self::Confilict(res) => {
                write!(f, "found two different layouts with a same name: {}", res)
            }
        }
    }
}

impl std::error::Error for LoadError {}
