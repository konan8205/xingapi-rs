// SPDX-License-Identifier: MIT

// 실시간 TR을 요청하는 예제입니다.

use clap::{App, Arg};
use lazy_static::lazy_static;
use xingapi::{
    data::{Block, Data, DataType},
    hashmap,
    response::Message,
    Real, XingApi,
};

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

enum Market {
    Kospi,
    Kosdaq,
}

impl Market {
    async fn is_listed(&self, xingapi: &XingApi, code: &str) -> bool {
        let res = xingapi
            .request(
                &Data {
                    code: "t8430".into(),
                    data_type: DataType::Input,
                    blocks: hashmap! {
                        "t8430InBlock" => Block::Block(hashmap! {
                            "gubun" => match self {
                                Self::Kospi => "1",
                                Self::Kosdaq => "2",
                            },
                        }),
                    },
                },
                None,
                None,
            )
            .await
            .unwrap();

        res.data().unwrap().blocks["t8430OutBlock"]
            .as_array()
            .unwrap()
            .iter()
            .find(|block| block["shcode"] == code)
            .is_some()
    }
}

#[tokio::main]
async fn main() {
    lazy_static! {
        static ref QUIT: AtomicBool = AtomicBool::new(false);
    }

    ctrlc::set_handler(|| {
        QUIT.store(true, Ordering::Relaxed);
    })
    .unwrap();

    let matches = App::new("listen-volume")
        .arg(Arg::new("id").short('i').long("id").required(true).takes_value(true))
        .arg(Arg::new("pw").short('p').long("pw").required(true).takes_value(true))
        .arg(Arg::new("code").short('c').long("code").required(true).takes_value(true))
        .get_matches();

    let id = matches.value_of("id").unwrap();
    let pw = matches.value_of("pw").unwrap();
    let code = matches.value_of("code").unwrap();

    let xingapi = XingApi::new().await.unwrap();

    xingapi.connect("demo.ebestsec.co.kr", 20001, None, None).await.unwrap();
    println!("server connected");

    let login = xingapi.login(&id, &pw, "", false).await.unwrap();
    if login.is_ok() {
        println!("login succeed: {}, {}", login.code(), login.message());
    } else {
        eprintln!("login failed: {}, {}", login.code(), login.message());
        return;
    }

    // 종목 코드가 어느 시장에 상장되어 있는지 검색합니다.
    let (tr_code, market) = {
        if Market::Kospi.is_listed(&xingapi, &code).await {
            ("S3_", "KOSPI")
        } else if Market::Kosdaq.is_listed(&xingapi, &code).await {
            ("K3_", "KOSDAQ")
        } else {
            eprintln!("unknown ticker: {}", code);
            return;
        }
    };

    let real = Arc::new(Real::new(xingapi.clone()).await.unwrap());

    real.subscribe(tr_code, vec![code.to_owned()]).await.unwrap();
    println!("registered: tr_code: {}, market: {}, ticker: {}", tr_code, market, code);

    while !QUIT.load(Ordering::Relaxed) {
        let real = real.clone();
        let recv_timeout =
            tokio::time::timeout(Duration::from_millis(10), async move { real.recv().await });

        if let Ok(res) = recv_timeout.await {
            let data = res.data().unwrap();
            let cvolume = data.blocks["OutBlock"]["cvolume"].parse::<u32>().unwrap();
            println!("real response: {}", cvolume);
        }
    }

    println!("ctrl-c interrupt");

    xingapi.disconnect().await;
    println!("server disconnected");

    assert_eq!(xingapi.is_connected().await, false);
}
