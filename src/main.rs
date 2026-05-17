// Copyright 2020-2026 Velithris
// SPDX-License-Identifier: MIT

mod utils;

use clap::{Arg, ArgMatches, Command};
use cli_mdt_parser as parser;
use std::{ffi::OsString, process::ExitCode};

#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

fn try_main() -> Result<(), Error> {
    let matches = Command::new("cli_mdt_parser")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Converts MDT-compatible strings to JSON and vice versa")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("decode")
                .about("Converts an MDT-compatible string to JSON")
                .disable_version_flag(true)
                .arg_required_else_help(true)
                .arg(
                    Arg::new("INPUT FILE")
                        .help("Sets the input file to use (- for stdin)")
                        .value_parser(clap::value_parser!(OsString))
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("OUTPUT FILE")
                        .help("Sets the output file to use")
                        .long("output")
                        .short('o')
                        .value_parser(clap::value_parser!(OsString)),
                ),
        )
        .subcommand(
            Command::new("encode")
                .about("Converts a JSON string to an MDT-compatible one")
                .disable_version_flag(true)
                .arg_required_else_help(true)
                .arg(
                    Arg::new("INPUT FILE")
                        .help("Sets the input file to use (- for stdin)")
                        .value_parser(clap::value_parser!(OsString))
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("OUTPUT FILE")
                        .help("Sets the output file to use")
                        .long("output")
                        .short('o')
                        .value_parser(clap::value_parser!(OsString)),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("encode", sub_m)) => encode(sub_m),
        Some(("decode", sub_m)) => decode(sub_m),
        _ => unreachable!(),
    }
}

fn encode(matches: &ArgMatches) -> Result<(), Error> {
    let input_file = matches.get_one::<OsString>("INPUT FILE").unwrap();
    let output_file = matches.get_one::<OsString>("OUTPUT FILE");

    let json = utils::read_from_file(input_file)?;
    let json = String::from_utf8(json)?;

    let lua_value = serde_json::from_str(&json)?;
    let wa_string = parser::encode(&lua_value)?;

    utils::write_to_file(output_file.map(|s| s.as_os_str()), wa_string.as_bytes())?;

    Ok(())
}

fn decode(matches: &ArgMatches) -> Result<(), Error> {
    let input_file = matches.get_one::<OsString>("INPUT FILE").unwrap();
    let output_file = matches.get_one::<OsString>("OUTPUT FILE");
    let wa_string = utils::read_from_file(input_file)?;

    let lua_value = parser::decode(wa_string.trim_ascii_end(), None)?;
    let json_string = serde_json::to_string_pretty(&lua_value)?;

    utils::write_to_file(output_file.map(|s| s.as_os_str()), json_string.as_bytes())?;

    Ok(())
}

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}
