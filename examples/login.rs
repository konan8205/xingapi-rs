// SPDX-License-Identifier: MIT

// 서버 연결 및 로그인 하는 예제입니다.

use clap::{App, Arg};
use xingapi::{response::Message, XingApi};

#[tokio::main]
async fn main() {
    let matches = App::new("login")
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

    println!("accounts:");
    xingapi.accounts().await.iter().for_each(|acc| println!("{:?}", acc));

    println!("client_ip: {:?}", xingapi.client_ip().await);
    println!("server_name: {:?}", xingapi.server_name().await,);
    println!("api_path: {:?}", xingapi.path().await);

    xingapi.disconnect().await;
    println!("server disconnected");

    assert_eq!(xingapi.is_connected().await, false);
}
