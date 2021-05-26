// SPDX-License-Identifier: MIT

use clap::{App, Arg, ValueHint};
use std::{error::Error, fs::OpenOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("res2json")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .takes_value(true)
                .value_hint(ValueHint::DirPath),
        )
        .arg(Arg::new("output").required(true).takes_value(true))
        .arg(Arg::new("pretty").short('p').long("pretty"))
        .get_matches();

    let input = matches.value_of("input");
    let output = matches.value_of("output").unwrap();
    let pretty = matches.is_present("pretty");

    let tr_layouts = if let Some(path) = input {
        xingapi_res::load_from_path(path)
    } else {
        xingapi_res::load()
    }?;

    let mut res_files = tr_layouts.keys().collect::<Vec<_>>();
    res_files.sort();
    println!("loaded: {:?}", res_files);

    let file = OpenOptions::new().write(true).create_new(true).open(output)?;
    if pretty {
        serde_json::to_writer_pretty(&file, &tr_layouts)?;
    } else {
        serde_json::to_writer(&file, &tr_layouts)?;
    }
    println!("json dumped: {:?}", output);

    Ok(())
}
