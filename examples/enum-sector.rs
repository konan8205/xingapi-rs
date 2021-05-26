// SPDX-License-Identifier: MIT

// TR을 요청하는 예제입니다.

use clap::{App, Arg};
use xingapi::{
    data::{Block, Data, DataType},
    hashmap,
    response::Message,
    XingApi,
};

#[tokio::main]
async fn main() {
    let matches = App::new("enum-sector")
        .arg(Arg::new("id").short('i').long("id").required(true).takes_value(true))
        .arg(Arg::new("pw").short('p').long("pw").required(true).takes_value(true))
        .get_matches();

    let id = matches.value_of("id").unwrap();
    let pw = matches.value_of("pw").unwrap();

    let xingapi = XingApi::new().await.unwrap();

    xingapi.connect("demo.ebestsec.co.kr", 20001, None, None).await.unwrap();
    println!("server connected");

    let login = xingapi.login(&id, &pw, "", false).await.unwrap();
    println!("login: {:?}", login);
    assert!(login.is_ok());

    let data = Data {
        code: "t8425".into(),
        data_type: DataType::Input,
        blocks: hashmap! {
            "t8425InBlock" => Block::Block(hashmap! {
                "dummy" => "",
            }),
        },
    };

    let res = xingapi.request(&data, None, None).await.unwrap();

    for block in res.data().unwrap().blocks["t8425OutBlock"].as_array().unwrap() {
        println!("tmcode: {:0>4}, tmname: {}", block["tmcode"], block["tmname"]);
    }

    xingapi.disconnect().await;
    println!("server disconnected");

    assert_eq!(xingapi.is_connected().await, false);
}
