// SPDX-License-Identifier: MIT

// 주식 시장의 종목을 조회하는 예제입니다.

use clap::Clap;
use xingapi::{
    data::{Block, Data, DataType},
    hashmap,
    response::Message,
    XingApi,
};

#[derive(Clap)]
struct Opts {
    #[clap(short)]
    id: String,
    #[clap(short)]
    pw: String,
}

#[tokio::main]
async fn main() {
    let opts = Opts::parse();
    let xingapi = XingApi::new().await.unwrap();

    xingapi.connect("demo.ebestsec.co.kr", 20001, None, None).await.unwrap();
    println!("server connected");

    let login = xingapi.login(&opts.id, &opts.pw, "", false).await.unwrap();
    println!("login: {:?}", login);
    assert!(login.is_ok());

    let res = xingapi
        .request(
            &Data {
                code: "t8430".into(),
                data_type: DataType::Input,
                blocks: hashmap! {
                    "t8430InBlock" => Block::Block(hashmap! {
                        "gubun" => "0",
                    }),
                },
            },
            None,
            None,
        )
        .await
        .unwrap();

    let mut stocks: Vec<&str> = res.data().unwrap().blocks["t8430OutBlock"]
        .as_array()
        .unwrap()
        .iter()
        .map(|block| block["shcode"].as_str())
        .collect();

    stocks.sort();
    println!("{:?}", stocks);

    xingapi.disconnect().await;
    println!("server disconnected");

    assert_eq!(xingapi.is_connected().await, false);
}
