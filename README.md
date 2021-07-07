# xingapi-rs

[![][xingapi-crate-img]][xingapi-crate]
[![][xingapi-docs-rs-img]][xingapi-docs-rs]

[xingapi-crate]: https://crates.io/crates/xingapi
[xingapi-docs-rs]: https://docs.rs/xingapi/
[xingapi-crate-img]: https://img.shields.io/crates/v/xingapi.svg?style=flat-square
[xingapi-docs-rs-img]: https://img.shields.io/docsrs/xingapi?style=flat-square

안전성과 간편성, 최적화를 동시에 추구하는 XingAPI 추상화 구현 라이브러리입니다.

## 튜토리얼
### XingAPI 설치
이베스트투자증권에서 제공하는 XingAPI 32비트 버전 설치 파일을 받아 기본 위치인
`C:\eBEST\xingAPI`에 설치해주세요.

### Rust 설치
Rust toolchain을 설치하기 위해선 Visual Studio의 MSVC 컴파일러가 필요합니다.

[rust-lang.org][rust-lang-start]에서 `rustup-init.exe`를 다운로드 받고 실행하여
설치해주세요. 64비트 버전을 설치하신 경우 콘솔에 다음의 명령어를 입력하여 직접
32비트 타겟을 추가해주셔야 합니다.

```sh
rustup target add i686-pc-windows-msvc
```

### 프로젝트 생성
`cargo new` 명령어를 콘솔에서 사용하여 프로젝트를 생성해주세요.

```sh
cargo new login
```

`Cargo.toml` 프로젝트 구성 파일에 다음과 같이 의존성 패키지 목록에 `xingapi`,
`tokio`, `clap`을 추가해주세요.

```toml
[package]
name = "login"
version = "0.1.0"
edition = "2018"

[dependencies]
clap = "3.0.0-beta.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"]}
xingapi = "0.2"
```

프로젝트 디렉터리에 `.cargo/config.toml` 파일을 생성하고 아래와 같이 작성하여
빌드 타겟을 고정해주세요.
```toml
[build]
target = "i686-pc-windows-msvc"
```

### 소스코드 작성
`src/main.rs` 파일에 다음의 소스 코드로 재작성해주세요.
```rust
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

    xingapi.disconnect().await;
    println!("server disconnected");

    assert_eq!(xingapi.is_connected().await, false);
}
```

### 컴파일 및 실행
`cargo run` 명령어를 사용하여 프로젝트를 빌드하고 실행할 수 있습니다.

다음과 같이 콘솔에 명령어를 입력하여 정상적으로 빌드되고 실행되는지
확인해주세요. `$ID`와 `$PW`에는 각각 모의투자 아이디와 비밀번호를 입력하시면
됩니다.

```sh
cargo run --target i686-pc-windows-msvc -- -i $ID -p $PW
```

### 더 알아볼 것
예제 코드는 저장소의 `examples` 디렉터리에 존재합니다.

Rust 언어가 처음이시라면 [온라인 설명서][book]나 그 [번역본][book-ko]을
읽어보세요. 한국에 서적으로도 출간되어 있습니다.

## 라이선스
라이선스는 Mozilla Public License 2.0을 채택하고 있습니다. 라이브러리의 소스
코드를 수정하지 않는 한 출처를 밝히고 자유롭게 사용하셔도 됩니다.

[rust-lang-start]: https://www.rust-lang.org/learn/get-started
[book]: https://doc.rust-lang.org/book/
[book-ko]: https://rinthel.github.io/rust-lang-book-ko/
