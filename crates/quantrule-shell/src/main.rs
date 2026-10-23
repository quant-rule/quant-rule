//! The QuantRule shell binary.
//!
//! The shell is the product's only surface. It parses commands, performs source
//! access, calls the data engine, and renders the result. It holds no indicator
//! or model logic.

#![forbid(unsafe_code)]

mod chart;
mod command;
mod csv;
mod export;
mod fetch;
mod render;
mod source;

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;

use command::Command;

fn main() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let exit_code = if args.is_empty() {
        run_interactive()
    } else {
        run_non_interactive(&args)
    };

    std::process::exit(exit_code);
}

fn run_interactive() -> i32 {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    loop {
        if let Err(error) = write_prompt(&mut stderr) {
            eprintln!("error: cannot write prompt: {error}");
            return 1;
        }

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => return 0,
            Ok(_) => {}
            Err(error) => {
                eprintln!("error: cannot read input: {error}");
                return 1;
            }
        }

        let command = match command::parse(&line) {
            Ok(Some(command)) => command,
            Ok(None) => continue,
            Err(error) => {
                eprintln!("error: {error}");
                continue;
            }
        };

        match command {
            Command::Candles(path) => {
                if let Err(error) = command::run_candles(&path, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::CandlesTolerant(path) => {
                if let Err(error) = command::run_candles_tolerant(&path, &mut stdout, &mut stderr) {
                    eprintln!("error: {error}");
                }
            }
            Command::Fetch(args) => {
                if let Err(error) = command::run_fetch(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::Help => {
                if let Err(error) = command::run_help(&mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::Quit => return 0,
        }
    }
}

fn run_non_interactive(args: &[OsString]) -> i32 {
    let Some(name) = args[0].to_str() else {
        eprintln!("error: command is not valid UTF-8");
        return 1;
    };

    match name {
        "candles" => {
            if args.len() != 2 {
                eprintln!("error: usage: candles <csv-path>");
                return 1;
            }

            let path = PathBuf::from(&args[1]);
            let mut stdout = io::stdout();
            match command::run_candles(&path, &mut stdout) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "candles-tolerant" => {
            if args.len() != 2 {
                eprintln!("error: usage: candles-tolerant <csv-path>");
                return 1;
            }

            let path = PathBuf::from(&args[1]);
            let mut stdout = io::stdout();
            let mut stderr = io::stderr();
            match command::run_candles_tolerant(&path, &mut stdout, &mut stderr) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "fetch" => {
            let mut arguments = Vec::with_capacity(args.len().saturating_sub(1));
            for argument in &args[1..] {
                let Some(argument) = argument.to_str() else {
                    eprintln!("error: fetch argument is not valid UTF-8");
                    return 1;
                };
                arguments.push(argument.to_owned());
            }

            let fetch_args = match command::parse_fetch_arguments(&arguments) {
                Ok(fetch_args) => fetch_args,
                Err(error) => {
                    eprintln!("error: {error}");
                    return 1;
                }
            };

            let mut stdout = io::stdout();
            match command::run_fetch(&fetch_args, &mut stdout) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "help" => {
            if args.len() != 1 {
                eprintln!("error: help takes no arguments");
                return 1;
            }

            let mut stdout = io::stdout();
            match command::run_help(&mut stdout) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "quit" => {
            if args.len() != 1 {
                eprintln!("error: quit takes no arguments");
                return 1;
            }
            0
        }
        other => {
            eprintln!("error: unknown command '{other}'; type 'help'");
            1
        }
    }
}

fn write_prompt(stderr: &mut impl Write) -> io::Result<()> {
    stderr.write_all(b"quantrule> ")?;
    stderr.flush()
}
