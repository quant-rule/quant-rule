//! The QuantRule shell binary.
//!
//! The shell is one of the product's two first-class surfaces, primarily for
//! agents. It parses a command, calls the gateway over HTTP, and renders the
//! result. It holds no indicator, model, strategy, execution, or portfolio
//! logic, and it never opens a local database.

#![forbid(unsafe_code)]

mod client;
mod command;
mod error;
mod output;
mod profile;
mod session;

use std::ffi::OsString;
use std::io::{self, Write};

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

        let invocation = match command::parse_line(&line) {
            Ok(Some(invocation)) => invocation,
            Ok(None) => continue,
            Err(error) => {
                eprintln!("error: {error}");
                continue;
            }
        };

        if matches!(invocation.command, command::Command::Quit) {
            return 0;
        }
        let _ = command::execute(invocation, &mut stdout, &mut stderr);
    }
}

fn run_non_interactive(args: &[OsString]) -> i32 {
    let Some(arguments) = string_arguments(args) else {
        eprintln!("error: command is not valid UTF-8");
        return 1;
    };
    let invocation = match command::parse_argv(&arguments) {
        Ok(Some(invocation)) => invocation,
        Ok(None) => return 0,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };

    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    command::execute(invocation, &mut stdout, &mut stderr)
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
