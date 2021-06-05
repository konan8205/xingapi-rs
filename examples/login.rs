// SPDX-License-Identifier: MIT

// 서버 연결 및 로그인 하는 예제입니다.

use clap::Clap;
use xingapi::{response::Message, XingApi};

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

    println!("accounts:");
    xingapi.accounts().await.iter().for_each(|acc| println!("{:?}", acc));

    println!("client_ip: {:?}", xingapi.client_ip().await);
    println!("server_name: {:?}", xingapi.server_name().await,);
    println!("api_path: {:?}", xingapi.path().await);

    xingapi.disconnect().await;
    println!("server disconnected");

    assert_eq!(xingapi.is_connected().await, false);
}
