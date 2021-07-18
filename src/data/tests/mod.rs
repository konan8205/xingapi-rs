// SPDX-License-Identifier: MPL-2.0

#![cfg(test)]

use super::{decode_array_block, decode_block, decode_non_block, encode, Block, Data, DataType};
use crate::hashmap;
use xingapi_res::{HeaderType, TrLayout};

use hex_literal::hex;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref LAYOUT_MAP: HashMap<String, TrLayout> = xingapi_res::load().unwrap();

    // date="2020-10-21", shcode="078020"
    static ref T1101_DATA: &'static [u8] = &include_bytes!("t1101-output.dat")[1..];

    // date="2020-10-21", shcode="078020"
    static ref T1764_DATA: &'static [u8] = &include_bytes!("t1764-output.dat")[1..];

    // date="2021-01-11"
    static ref T0424_DATA: &'static [u8] = &include_bytes!("t0424-output.dat")[1..];
}

#[test]
fn test_decode_block() {
    // t1101OutBlock
    {
        let tr_layout = LAYOUT_MAP.get("t1101").unwrap();
        assert!(tr_layout.attr);
        assert!(tr_layout.block);
        assert_eq!(tr_layout.header_type.unwrap(), HeaderType::A);

        let data = Data {
            code: "t1101".into(),
            data_type: DataType::Output,
            blocks: hashmap! {
                "t1101OutBlock" => {
                    decode_block(
                        tr_layout,
                        tr_layout
                            .out_blocks
                            .iter()
                            .find(|b| b.name == "t1101OutBlock")
                            .unwrap(),
                        &T1101_DATA,
                    )
                    .unwrap()
                }
            },
        };

        validate_t1101_data(&data);
    }
}

#[test]
fn test_decode_array_block() {
    // t1104OutBlock1
    {
        // date="2021-01-12", code="078020", gubn="1", dat="1", dat2="1"
        const T1104_DATA: &[u8] = &hex!("00 31 30 30 30 30 37 32 36 30");

        let tr_layout = LAYOUT_MAP.get("t1104").unwrap();
        assert!(!tr_layout.attr);
        assert!(tr_layout.block);
        assert_eq!(tr_layout.header_type.unwrap(), HeaderType::A);

        let block = decode_array_block(
            tr_layout,
            tr_layout.out_blocks.iter().find(|b| b.name == "t1104OutBlock1").unwrap(),
            T1104_DATA,
        )
        .unwrap();

        assert_eq!(
            block,
            Block::Array(vec![hashmap! {
                "indx" => "",
                "gubn" => "1",
                "vals" => "00007260",
            }])
        );
    }

    // t1764OutBlock
    {
        let tr_layout = LAYOUT_MAP.get("t1764").unwrap();
        assert!(tr_layout.attr);
        assert!(tr_layout.block);
        assert_eq!(tr_layout.header_type.unwrap(), HeaderType::A);

        let data = Data {
            code: "t1764".into(),
            data_type: DataType::Output,
            blocks: hashmap! {
                "t1764OutBlock" => {
                    decode_array_block(
                        tr_layout,
                        tr_layout
                            .out_blocks
                            .iter()
                            .find(|b| b.name == "t1764OutBlock")
                            .unwrap(),
                        &T1764_DATA,
                    )
                    .unwrap()
                }
            },
        };

        validate_t1764_data(&data);
    }
}

#[test]
fn test_decode_non_block() {
    let tr_layout = LAYOUT_MAP.get("t0424").unwrap();
    assert!(tr_layout.attr);
    assert!(!tr_layout.block);
    assert_eq!(tr_layout.header_type.unwrap(), HeaderType::D);

    let data = decode_non_block(tr_layout, &T0424_DATA).unwrap();
    assert_eq!(data.code, "t0424");
    assert_eq!(data.data_type, DataType::Output);

    validate_t0424_data(&data);
}

#[test]
fn test_encode() {
    let data = Data {
        code: "t1104".into(),
        data_type: DataType::Input,
        blocks: hashmap! {
            "t1104InBlock" => Block::Block(hashmap! {
                "code" => "096530",
                "nrec" => "1",
            }),
            "t1104InBlock1" => Block::Array(vec![hashmap! {
                "indx" => "0",
                "gubn" => "1",
                "dat1" => "1",
                "dat2" => "1",
            }]),
        },
    };

    assert_eq!(
        encode(&LAYOUT_MAP, &data).unwrap(),
        hex!("30 39 36 35 33 30 31 00 30 31 31 31 00 00 00 00 00 00 00")
    );
}

fn validate_t1101_data(data: &Data) {
    assert_eq!(data.code, "t1101");
    assert_eq!(data.data_type, DataType::Output);
    assert_eq!(data.blocks.len(), 1);

    assert_eq!(
        data.blocks["t1101OutBlock"],
        Block::Block(hashmap! {
            "hname" => "이베스트투자증권",
            "price" => "00006000",
            "sign" => "2",
            "change" => "00000090",
            "diff" => "001.52",
            "volume" => "000000109214",
            "jnilclose" => "00005910",
            "offerho1" => "00006000",
            "bidho1" => "00005990",
            "offerrem1" => "000000004207",
            "bidrem1" => "000000002620",
            "preoffercha1" => "000000000000",
            "prebidcha1" => "000000000000",
            "offerho2" => "00006010",
            "bidho2" => "00005980",
            "offerrem2" => "000000001288",
            "bidrem2" => "000000000577",
            "preoffercha2" => "000000000000",
            "prebidcha2" => "000000000000",
            "offerho3" => "00006020",
            "bidho3" => "00005970",
            "offerrem3" => "000000001266",
            "bidrem3" => "000000022248",
            "preoffercha3" => "000000000000",
            "prebidcha3" => "000000000000",
            "offerho4" => "00006030",
            "bidho4" => "00005960",
            "offerrem4" => "000000000712",
            "bidrem4" => "000000001098",
            "preoffercha4" => "000000000000",
            "prebidcha4" => "000000000000",
            "offerho5" => "00006040",
            "bidho5" => "00005950",
            "offerrem5" => "000000000589",
            "bidrem5" => "000000001688",
            "preoffercha5" => "000000000000",
            "prebidcha5" => "000000000000",
            "offerho6" => "00006050",
            "bidho6" => "00005940",
            "offerrem6" => "000000000563",
            "bidrem6" => "000000001221",
            "preoffercha6" => "000000000000",
            "prebidcha6" => "000000000000",
            "offerho7" => "00006060",
            "bidho7" => "00005930",
            "offerrem7" => "000000000381",
            "bidrem7" => "000000002501",
            "preoffercha7" => "000000000000",
            "prebidcha7" => "000000000000",
            "offerho8" => "00006070",
            "bidho8" => "00005920",
            "offerrem8" => "000000000412",
            "bidrem8" => "000000001167",
            "preoffercha8" => "000000000000",
            "prebidcha8" => "000000000000",
            "offerho9" => "00006080",
            "bidho9" => "00005910",
            "offerrem9" => "000000000225",
            "bidrem9" => "000000000701",
            "preoffercha9" => "000000000000",
            "prebidcha10" => "000000000000",
            "offerho10" => "00006090",
            "bidho10" => "00005900",
            "offerrem10" => "000000000640",
            "bidrem10" => "000000000777",
            "preoffercha10" => "000000000000",
            "prebidcha9" => "000000000000",
            "offer" => "000000010283",
            "bid" => "000000034598",
            "preoffercha" => "-00000011585",
            "prebidcha" => "-00000035931",
            "hotime" => "16000112",
            "yeprice" => "00000000",
            "yevolume" => "000000000000",
            "yesign" => "3",
            "yechange" => "00000000",
            "yediff" => "000.00",
            "tmoffer" => "000000000000",
            "tmbid" => "000000000098",
            "ho_status" => "2",
            "shcode" => "078020",
            "uplmtprice" => "00007680",
            "dnlmtprice" => "00004140",
            "open" => "00005910",
            "high" => "00006000",
            "low" => "00005870",
        })
    );
}

fn validate_t1764_data(data: &Data) {
    assert_eq!(data.code, "t1764");
    assert_eq!(data.data_type, DataType::Output);
    assert_eq!(data.blocks.len(), 1);

    assert_eq!(
        data.blocks["t1764OutBlock"],
        Block::Array(vec![
            hashmap! { "rank" => "0", "tradno" => "000", "tradname" => "외국계회원사전체" },
            hashmap! { "rank" => "1", "tradno" => "042", "tradname" => "CS증권" },
            hashmap! { "rank" => "2", "tradno" => "017", "tradname" => "KB증권" },
            hashmap! { "rank" => "3", "tradno" => "012", "tradname" => "NH투자증권" },
            hashmap! { "rank" => "4", "tradno" => "025", "tradname" => "SK증권" },
            hashmap! { "rank" => "5", "tradno" => "043", "tradname" => "UBS" },
            hashmap! { "rank" => "6", "tradno" => "063", "tradname" => "eBEST" },
            hashmap! { "rank" => "7",  "tradno" => "045", "tradname" => "골드만" },
            hashmap! { "rank" => "8", "tradno" => "004", "tradname" => "대신증권" },
            hashmap! { "rank" => "9", "tradno" => "010", "tradname" => "메리츠" },
            hashmap! { "rank" => "10", "tradno" => "044", "tradname" => "메릴린치" },
            hashmap! { "rank" => "11", "tradno" => "005", "tradname" => "미래대우" },
            hashmap! { "rank" => "12", "tradno" => "030", "tradname" => "삼성증권" },
            hashmap! { "rank" => "13", "tradno" => "006", "tradname" => "신영증권" },
            hashmap! { "rank" => "14", "tradno" => "002", "tradname" => "신한투자" },
            hashmap! { "rank" => "15", "tradno" => "008", "tradname" => "유진증권" },
            hashmap! { "rank" => "16", "tradno" => "050", "tradname" => "키움증권" },
            hashmap! { "rank" => "17", "tradno" => "046", "tradname" => "하이증권" },
            hashmap! { "rank" => "18", "tradno" => "003", "tradname" => "한국증권" },
        ])
    );
}

fn validate_t0424_data(data: &Data) {
    assert_eq!(data.code, "t0424");
    assert_eq!(data.data_type, DataType::Output);

    assert_eq!(
        data.blocks["t0424OutBlock"],
        Block::Block(hashmap! {
            "sunamt" => "000000000500007907",
            "dtsunik" => "000000000000000000",
            "mamt" => "000000000001957400",
            "sunamt1" => "500000000",
            "cts_expcode" => "",
            "tappamt" => "000000000001965600",
            "tdtsunik" => "000000000000008200",
        })
    );

    assert_eq!(
        data.blocks["t0424OutBlock1"],
        Block::Array(vec![
            hashmap! {
                "expcode" => "005930",
                "jangb" => "",
                "janqty" => "20",
                "mdposqt" => "20",
                "pamt" => "90700",
                "mamt" => "1814000",
                "sinamt" => "0",
                "lastdt" => "",
                "msat" => "20",
                "mpms" => "90700",
                "mdat" => "0",
                "mpmd" => "0",
                "jsat" => "0",
                "jpms" => "0",
                "jdat" => "0",
                "jpmd" => "0",
                "sysprocseq" => "48966",
                "loandt" => "",
                "hname" => "삼성전자",
                "marketgb" => "",
                "jonggb" => "3",
                "janrt" => "9267",
                "price" => "00091000",
                "appamt" => "000000000001820000",
                "dtsunik" => "",
                "sunikrt" => "0000000033",
                "fee" => "0000000545",
                "tax" => "0000004550",
                "sininter" => "0000000000",
            },
            hashmap! {
                "expcode" => "078020",
                "jangb" => "",
                "janqty" => "20",
                "mdposqt" => "20",
                "pamt" => "7170",
                "mamt" => "143400",
                "sinamt" => "0",
                "lastdt" => "",
                "msat" => "20",
                "mpms" => "7170",
                "mdat" => "0",
                "mpmd" => "0",
                "jsat" => "0",
                "jpms" => "0",
                "jdat" => "0",
                "jpmd" => "0",
                "sysprocseq" => "48934",
                "loandt" => "",
                "hname" => "이베스트투자증권",
                "marketgb" => "",
                "jonggb" => "2",
                "janrt" => "733",
                "price" => "00007280",
                "appamt" => "000000000000145600",
                "dtsunik" => "",
                "sunikrt" => "0000000153",
                "fee" => "0000000042",
                "tax" => "0000000364",
                "sininter" => "0000000000",
            }
        ])
    )
}
