// SPDX-License-Identifier: MPL-2.0

use crate::error::{unexpected_eof, unexpected_syntax, Error, ErrorKind};
use crate::read::{Read, StrRead};

use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::{convert::AsRef, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

fn next_symbol<'a, R: Read<'a>>(reader: &R) -> Result<&'a str, Error> {
    reader.next_symbol().ok_or_else(|| unexpected_eof(reader))
}

fn get_symbol<'a, R: Read<'a>>(reader: &R) -> Result<&'a str, Error> {
    reader.get_symbol().ok_or_else(|| unexpected_eof(reader))
}

/// TR 타입에 대한 열거형 객체입니다.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TrType {
    /// 조회 TR
    #[cfg_attr(feature = "serde", serde(rename = "tr"))]
    Tr,
    /// 실시간 TR
    #[cfg_attr(feature = "serde", serde(rename = "real"))]
    Real,
}

impl FromStr for TrType {
    type Err = ();
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            ".Func" | "tr" => Ok(TrType::Tr),
            ".Feed" | "real" => Ok(TrType::Real),
            _ => Err(()),
        }
    }
}

/// 헤더 타입에 대한 열거형 객체입니다.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum HeaderType {
    A,
    B,
    C,
    D,
}

impl FromStr for HeaderType {
    type Err = ();
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            _ => Err(()),
        }
    }
}

/// TR 레이아웃에 대한 추상화 객체입니다.
///
/// [`FromStr`](FromStr)이 구현되어 문자열로부터 파싱할 수 있습니다.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TrLayout {
    /// TR 타입
    pub tr_type: TrType,
    /// TR 설명
    pub desc: String,
    /// TR 코드
    pub code: String,
    /// 각 필드 끝에 attribute byte 존재 여부
    pub attr: bool,
    /// block mode 여부
    pub block: bool,
    pub key: Option<u8>,
    pub group: Option<u8>,
    /// 헤더 타입
    pub header_type: Option<HeaderType>,
    /// 요청 블록 목록
    pub in_blocks: Vec<BlockLayout>,
    /// 응답 블록 목록
    pub out_blocks: Vec<BlockLayout>,
}

impl TrLayout {
    fn from_reader<'a, R: Read<'a>>(reader: &R) -> Result<Self, Error> {
        let parse_delimiter = || -> Result<(), Error> {
            match next_symbol(reader)? {
                "," => Ok(()),
                ";" => Err(Error::new(&reader.position(), ErrorKind::TrParamCount).into()),
                _ => Err(Error::new(&reader.position(), ErrorKind::TrParam).into()),
            }
        };

        let unexpected_func_param =
            || -> Error { Error::new(&reader.position(), ErrorKind::TrParam).into() };

        if next_symbol(reader)? != "BEGIN_FUNCTION_MAP" {
            return Err(unexpected_syntax(reader));
        }

        let func_type =
            TrType::from_str(next_symbol(reader)?).map_err(|_| unexpected_func_param())?;
        parse_delimiter()?;

        let desc = next_symbol(reader)?.to_owned();
        parse_delimiter()?;

        let name = next_symbol(reader)?.to_owned();

        if func_type == TrType::Real && name.len() != 3 {
            return Err(unexpected_func_param());
        }

        let mut attr = false;
        let mut block = false;
        let mut key = None;
        let mut group = None;
        let mut header_type = None;

        lazy_static! {
            static ref KV_REGEX: Regex =
                Regex::new(r"(?P<key>[[:alpha:]]*)=(?P<value>.*)").unwrap();
        }

        loop {
            match next_symbol(reader)? {
                "," => {}
                ";" => break,
                _ => return Err(unexpected_func_param()),
            }

            let param = next_symbol(reader)?;
            if let Some(cap) = KV_REGEX.captures(param) {
                let param_key = cap.name("key").ok_or_else(|| unexpected_func_param())?.as_str();
                let param_val = cap.name("value").ok_or_else(|| unexpected_func_param())?.as_str();

                match param_key {
                    "headtype" => {
                        header_type = Some(
                            HeaderType::from_str(param_val).map_err(|_| unexpected_func_param())?,
                        )
                    }
                    "key" => {
                        key = Some(param_val.parse::<u8>().map_err(|_| unexpected_func_param())?)
                    }
                    "group" => {
                        group = Some(param_val.parse::<u8>().map_err(|_| unexpected_func_param())?)
                    }
                    "tuxcode" | "svr" | "SERVICE" | "CREATOR" | "CREDATE" => {}
                    _ => {
                        return Err(unexpected_func_param());
                    }
                }
            } else {
                match param {
                    "attr" => {
                        attr = true;
                    }
                    "block" => {
                        block = true;
                    }
                    "ENCRYPT" | "SIGNATURE" => {}
                    _ => {
                        return Err(unexpected_func_param());
                    }
                }
            }
        }

        if next_symbol(reader)? != "BEGIN_DATA_MAP" {
            return Err(unexpected_syntax(reader));
        }

        let mut in_blocks = Vec::new();
        let mut out_blocks = Vec::new();

        loop {
            if get_symbol(reader)? == "END_DATA_MAP" {
                break;
            }

            let block = BlockLayout::from_reader(reader, attr)?;
            match block.block_type {
                BlockType::Input => {
                    in_blocks.push(block);
                }
                BlockType::Output => {
                    out_blocks.push(block);
                }
            }
        }

        Ok(TrLayout {
            tr_type: func_type,
            desc,
            code: name,
            attr,
            block,
            key,
            group,
            header_type,
            in_blocks,
            out_blocks,
        })
    }
}

impl FromStr for TrLayout {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::from_reader(&StrRead::new(text))
    }
}

/// 블록 타입에 대한 열거형 객체입니다.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BlockType {
    /// 요청 타입
    #[cfg_attr(feature = "serde", serde(rename = "input"))]
    Input,
    /// 응답 타입
    #[cfg_attr(feature = "serde", serde(rename = "output"))]
    Output,
}

impl FromStr for BlockType {
    type Err = ();
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.as_ref() {
            "input" => Ok(Self::Input),
            "output" => Ok(Self::Output),
            _ => Err(()),
        }
    }
}

/// 블록 레이아웃에 대한 추상화 객체입니다.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BlockLayout {
    /// 블록 이름
    pub name: String,
    /// 블록 설명
    pub desc: String,
    /// 블록 타입
    pub block_type: BlockType,
    /// 배열 여부
    pub occurs: bool,
    /// 각 필드에 attribute byte를 포함한 전체 길이
    pub len: usize,
    /// 필드 목록
    pub fields: Vec<FieldLayout>,
}

impl BlockLayout {
    fn from_reader<'a, R: Read<'a>>(reader: &R, attr: bool) -> Result<Self, Error> {
        let parse_delimiter = || -> Result<(), Error> {
            match next_symbol(reader)? {
                "," => Ok(()),
                ";" => Err(Error::new(&reader.position(), ErrorKind::BlockParamCount).into()),
                _ => Err(Error::new(&reader.position(), ErrorKind::BlockParam).into()),
            }
        };

        let unexpected_block_param =
            || -> Error { Error::new(&reader.position(), ErrorKind::BlockParam).into() };

        lazy_static! {
            static ref NAME_REGEX: Regex = Regex::new(r"\w*(In|Out)Block\d*").unwrap();
        }

        let name = next_symbol(reader)?.to_owned();
        if !NAME_REGEX.is_match(&name) {
            return Err(unexpected_block_param());
        }
        parse_delimiter()?;

        let desc = next_symbol(reader)?.to_owned();
        parse_delimiter()?;

        let block_type =
            BlockType::from_str(next_symbol(reader)?).map_err(|_| unexpected_block_param())?;

        let mut occurs = false;

        loop {
            match next_symbol(reader)? {
                "," => {}
                ";" => break,
                _ => return Err(unexpected_block_param()),
            }

            match next_symbol(reader)? {
                "occurs" => {
                    occurs = true;
                }
                _ => {
                    return Err(unexpected_block_param());
                }
            }
        }

        if next_symbol(reader)? != "begin" {
            return Err(unexpected_syntax(reader));
        }

        let mut fields = Vec::new();
        loop {
            if get_symbol(reader)? == "end" {
                reader.next_symbol().unwrap();
                break;
            }

            fields.push(FieldLayout::from_reader(reader)?);
        }

        let len = fields.iter().map(|f| f.len + if attr { 1 } else { 0 }).sum();

        Ok(BlockLayout { name, desc, block_type, occurs, len, fields })
    }
}

impl AsRef<BlockLayout> for BlockLayout {
    fn as_ref(&self) -> &BlockLayout {
        &self
    }
}

/// 필드 타입에 대한 열거형 객체입니다.
///
/// `long`과 `int`는 모두 `FieldType::Int`로 간주됩니다.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FieldType {
    /// 문자열
    #[cfg_attr(feature = "serde", serde(rename = "char"))]
    Char,
    /// 날짜
    #[cfg_attr(feature = "serde", serde(rename = "date"))]
    Date,
    /// 정수
    #[cfg_attr(feature = "serde", serde(rename = "int"))]
    Int,
    /// 32비트 실수
    #[cfg_attr(feature = "serde", serde(rename = "float"))]
    Float,
    /// 64비트 실수
    #[cfg_attr(feature = "serde", serde(rename = "double"))]
    Double,
}

impl FromStr for FieldType {
    type Err = ();
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.as_ref() {
            "char" => Ok(Self::Char),
            "date" => Ok(Self::Date),
            "long" | "int" => Ok(Self::Int),
            "float" => Ok(Self::Float),
            "double" => Ok(Self::Double),
            _ => Err(()),
        }
    }
}

/// 필드 레이아웃에 대한 추상화 객체입니다.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FieldLayout {
    /// 필드 설명
    pub desc: String,
    /// 필드의 첫 번째 이름
    pub name_old: String,
    /// 필드의 두 번째 이름
    pub name: String,
    /// 필드 타입
    pub field_type: FieldType,
    /// attribute byte를 제외한 길이
    pub len: usize,
    /// 소수점 위치
    pub point: Option<usize>,
}

impl FieldLayout {
    fn from_reader<'a, R: Read<'a>>(reader: &R) -> Result<Self, Error> {
        let parse_delimiter = || -> Result<(), Error> {
            match next_symbol(reader)? {
                "," => Ok(()),
                ";" => Err(Error::new(&reader.position(), ErrorKind::FieldParamCount).into()),
                _ => Err(Error::new(&reader.position(), ErrorKind::FieldParam).into()),
            }
        };

        let unexpected_field_param =
            || -> Error { Error::new(&reader.position(), ErrorKind::FieldParam).into() };

        let desc = next_symbol(reader)?.to_owned();
        parse_delimiter()?;

        let name_old = next_symbol(reader)?.to_owned();
        parse_delimiter()?;

        let name = next_symbol(reader)?.to_owned();
        parse_delimiter()?;

        let field_type =
            FieldType::from_str(next_symbol(reader)?).map_err(|_| unexpected_field_param())?;
        parse_delimiter()?;

        lazy_static! {
            static ref LENGTH_REGEX: Regex = Regex::new(r"(?P<len>\d+)(\.(?P<point>\d))?").unwrap();
        }

        let captures: Captures =
            LENGTH_REGEX.captures(next_symbol(reader)?).ok_or(unexpected_field_param())?;

        let len = captures
            .name("len")
            .ok_or(unexpected_field_param())?
            .as_str()
            .parse::<usize>()
            .map_err(|_| unexpected_field_param())?;

        let point = if let Some(cap) = captures.name("point") {
            let point = cap.as_str();
            if !point.is_empty() {
                Some(point.parse::<usize>().map_err(|_| unexpected_field_param())?)
            } else {
                None
            }
        } else {
            None
        };

        // 필드가 세미콜론으로 끝나지 않는 경우도 있음
        match get_symbol(reader)? {
            ";" => {
                reader.next_symbol().unwrap();
            }
            _ => {}
        }

        Ok(FieldLayout { desc, name_old, name, field_type, len, point })
    }
}

impl AsRef<FieldLayout> for FieldLayout {
    fn as_ref(&self) -> &FieldLayout {
        &self
    }
}
