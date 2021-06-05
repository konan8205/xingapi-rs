// SPDX-License-Identifier: MIT

use std::{fs::OpenOptions, path::PathBuf};

use clap::{Clap, ValueHint};

#[derive(Clap)]
struct Opts {
    #[clap(short, parse(from_os_str), value_hint = ValueHint::DirPath)]
    input: Option<PathBuf>,
    #[clap(short, parse(from_os_str), value_hint = ValueHint::FilePath)]
    output: PathBuf,
    #[clap(short)]
    pretty: bool,
}

fn main() {
    let opts = Opts::parse();
    let tr_layouts = if let Some(path) = opts.input {
        xingapi_res::load_from_path(path)
    } else {
        xingapi_res::load()
    }
    .unwrap();

    println!("loaded: {}", tr_layouts.len());

    let file = OpenOptions::new().write(true).create_new(true).open(&opts.output).unwrap();
    if opts.pretty {
        serde_json::to_writer_pretty(&file, &tr_layouts).unwrap();
    } else {
        serde_json::to_writer(&file, &tr_layouts).unwrap();
    }

    println!("json dumped: {}", opts.output.display());
}
