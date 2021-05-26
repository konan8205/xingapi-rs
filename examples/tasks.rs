// SPDX-License-Identifier: MIT

// 시간당 요청 제한 횟수에 맞춰 여러 가지 TR을 동시에 요청하는 예제입니다.

use clap::{App, Arg};
use std::time::Duration;
use xingapi::{
    data::{Block, Data, DataType},
    error::ErrorKind,
    hashmap,
    response::Message,
    XingApi,
};

#[tokio::main]
async fn main() {
    let matches = App::new("tasks")
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

    let t1101_one_sec_limit = xingapi.limit_per_one_sec("t1101").await as u64;
    let t1101_ten_min_limit = xingapi.limit_per_ten_min("t1101").await as u64;
    println!("t1101 one_sec_limit: {}", t1101_one_sec_limit);
    println!("t1101 ten_min_limit: {}", t1101_ten_min_limit);

    let t1764_one_sec_limit = xingapi.limit_per_one_sec("t1764").await as u64;
    let t1764_ten_min_limit = xingapi.limit_per_ten_min("t1764").await as u64;
    println!("t1764 one_sec_limit: {}", t1764_one_sec_limit);
    println!("t1764 ten_min_limit: {}", t1764_ten_min_limit);

    let xingapi_clone = xingapi.clone();

    let t1101_loop = tokio::spawn(async move {
        let xingapi = xingapi_clone;

        let data = Data {
            code: "t1101".into(),
            data_type: DataType::Input,
            blocks: hashmap! {
                "t1101InBlock" => Block::Block(hashmap! {
                    "shcode" => "078020",
                }),
            },
        };

        for i in 0..20 * t1101_one_sec_limit {
            let res = loop {
                let result = xingapi.request(&data, None, None).await;
                match &result {
                    Ok(_) => {}
                    Err(err) => {
                        if err.kind() == ErrorKind::LimitReached {
                            println!("t1101: limit reached");
                            tokio::time::sleep(Duration::from_millis(5)).await;
                            continue;
                        }
                    }
                }

                break result;
            }
            .unwrap();

            let elapsed = res.elapsed();

            assert!(res.is_ok());
            println!("t1101: index: {}, elapsed: {} ms", i, elapsed.as_millis());

            let wait_duration = Duration::from_secs_f32(1.0 / t1101_one_sec_limit as f32);
            if wait_duration > elapsed {
                tokio::time::sleep(wait_duration - elapsed).await;
            }
        }
    });

    let xingapi_clone = xingapi.clone();

    let t1764_loop = tokio::spawn(async move {
        let xingapi = xingapi_clone;

        let data = Data {
            code: "t1764".into(),
            data_type: DataType::Input,
            blocks: hashmap! {
                "t1764InBlock" => Block::Block(hashmap! {
                    "shcode" => "096530",
                    "gubun1" => "0",
                }),
            },
        };

        for i in 0..20 * t1764_one_sec_limit + 1 {
            let res = loop {
                let result = xingapi.request(&data, None, None).await;
                match &result {
                    Ok(_) => {}
                    Err(err) => {
                        if err.kind() == ErrorKind::LimitReached {
                            println!("t1764: limit reached");
                            tokio::time::sleep(Duration::from_millis(5)).await;
                            continue;
                        }
                    }
                }

                break result;
            }
            .unwrap();

            assert!(res.is_ok());
            println!("t1764: index: {}, elapsed: {} ms", i, res.elapsed().as_millis());

            let wait_duration = Duration::from_secs_f32(1.0 / t1764_one_sec_limit as f32);
            tokio::time::sleep(wait_duration).await;
        }
    });

    t1101_loop.await.unwrap();
    t1764_loop.await.unwrap();

    xingapi.disconnect().await;
    println!("server disconnected");

    assert_eq!(xingapi.is_connected().await, false);
}
