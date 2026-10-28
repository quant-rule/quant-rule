//! The QuantRule shell binary.
//!
//! The shell is the product's only surface. It parses commands, performs source
//! access, calls the data engine, and renders the result. It holds no indicator
//! or model logic.

#![forbid(unsafe_code)]

mod chart;
mod command;
mod config;
mod csv;
mod export;
mod fetch;
mod labels;
mod render;
mod render_dashboard;
mod render_report;
mod render_series;
mod render_state;
mod source;
mod store;

use std::ffi::OsString;
use std::io::{self, Write};

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

        if matches!(command, Command::Quit) {
            return 0;
        }
        let _ = command::execute(command, &mut stdout, &mut stderr);
    }
}

fn run_non_interactive(args: &[OsString]) -> i32 {
    let Some(arguments) = string_arguments(args) else {
        eprintln!("error: command is not valid UTF-8");
        return 1;
    };
    let command = match command::parse_argv(&arguments) {
        Ok(Some(command)) => command,
        Ok(None) => return 0,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };

    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    command::execute(command, &mut stdout, &mut stderr)
}

fn string_arguments(args: &[OsString]) -> Option<Vec<String>> {
    args.iter()
        .map(|argument| argument.to_str().map(str::to_owned))
        .collect()
}

fn write_prompt(stderr: &mut impl Write) -> io::Result<()> {
    stderr.write_all(b"quantrule> ")?;
    stderr.flush()
}
