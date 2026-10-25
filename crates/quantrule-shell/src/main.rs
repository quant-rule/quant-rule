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
mod labels;
mod render;
mod render_series;
mod render_state;
mod source;
mod store;

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
            Command::Timeframes(path) => {
                if let Err(error) = command::run_timeframes(&path, &mut stdout, &mut stderr) {
                    eprintln!("error: {error}");
                }
            }
            Command::MarketState(args) => {
                if let Err(error) = command::run_market_state(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::MarketChart(args) => {
                if let Err(error) = command::run_market_chart(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::Axis { args, axis } => {
                if let Err(error) = command::run_axis(&args, axis, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::Events(args) => {
                if let Err(error) = command::run_events(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::Legs(args) => {
                if let Err(error) = command::run_legs(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::ModelSeries(args) => {
                if let Err(error) = command::run_model_series(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::ResearchExport(args) => {
                if let Err(error) = command::run_research_export(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::ResearchSummary(path) => {
                if let Err(error) = command::run_research_summary(&path, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::StoreInit(path) => {
                if let Err(error) = command::run_store_init(&path, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::StoreIngest(args) => {
                if let Err(error) = command::run_store_ingest(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::StoreVerify(args) => {
                if let Err(error) = command::run_store_verify(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::StoreList(args) => {
                if let Err(error) = command::run_store_list(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::StoreShow(args) => {
                if let Err(error) = command::run_store_show(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::StoreRemove(args) => {
                if let Err(error) = command::run_store_remove(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::Db(args) => {
                if let Err(error) = command::run_db(&args, &mut stdout) {
                    eprintln!("error: {error}");
                }
            }
            Command::Fetch(args) => {
                if let Err(error) = command::run_fetch(&args, &mut stdout, &mut stderr) {
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

fn string_arguments(args: &[OsString]) -> Option<Vec<String>> {
    args.iter()
        .map(|argument| argument.to_str().map(str::to_owned))
        .collect()
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
        "timeframes" => {
            if args.len() != 2 {
                eprintln!("error: usage: timeframes <csv-path>");
                return 1;
            }

            let path = PathBuf::from(&args[1]);
            let mut stdout = io::stdout();
            let mut stderr = io::stderr();
            match command::run_timeframes(&path, &mut stdout, &mut stderr) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "market-state" => {
            let Some(arguments) = string_arguments(&args[1..]) else {
                eprintln!("error: market-state argument is not valid UTF-8");
                return 1;
            };
            let model_args = match command::parse_model_arguments(&arguments, "market-state") {
                Ok(model_args) => model_args,
                Err(error) => {
                    eprintln!("error: {error}");
                    return 1;
                }
            };
            let mut stdout = io::stdout();
            match command::run_market_state(&model_args, &mut stdout) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "market-chart" => {
            let mut arguments = Vec::with_capacity(args.len().saturating_sub(1));
            for argument in &args[1..] {
                let Some(argument) = argument.to_str() else {
                    eprintln!("error: market-chart argument is not valid UTF-8");
                    return 1;
                };
                arguments.push(argument.to_owned());
            }

            let chart_args = match command::parse_market_chart_arguments(&arguments) {
                Ok(chart_args) => chart_args,
                Err(error) => {
                    eprintln!("error: {error}");
                    return 1;
                }
            };

            let mut stdout = io::stdout();
            match command::run_market_chart(&chart_args, &mut stdout) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "axis" => {
            let Some(arguments) = string_arguments(&args[1..]) else {
                eprintln!("error: axis argument is not valid UTF-8");
                return 1;
            };
            let (model_args, axis) = match command::parse_axis_arguments(&arguments) {
                Ok(parsed) => parsed,
                Err(error) => {
                    eprintln!("error: {error}");
                    return 1;
                }
            };
            let mut stdout = io::stdout();
            match command::run_axis(&model_args, axis, &mut stdout) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "events" => {
            let Some(arguments) = string_arguments(&args[1..]) else {
                eprintln!("error: events argument is not valid UTF-8");
                return 1;
            };
            let model_args = match command::parse_model_arguments(&arguments, "events") {
                Ok(model_args) => model_args,
                Err(error) => {
                    eprintln!("error: {error}");
                    return 1;
                }
            };
            let mut stdout = io::stdout();
            match command::run_events(&model_args, &mut stdout) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "legs" => {
            let Some(arguments) = string_arguments(&args[1..]) else {
                eprintln!("error: legs argument is not valid UTF-8");
                return 1;
            };
            let model_args = match command::parse_model_arguments(&arguments, "legs") {
                Ok(model_args) => model_args,
                Err(error) => {
                    eprintln!("error: {error}");
                    return 1;
                }
            };
            let mut stdout = io::stdout();
            match command::run_legs(&model_args, &mut stdout) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "model-series" => {
            let Some(arguments) = string_arguments(&args[1..]) else {
                eprintln!("error: model-series argument is not valid UTF-8");
                return 1;
            };
            let series_args = match command::parse_model_series_arguments(&arguments) {
                Ok(series_args) => series_args,
                Err(error) => {
                    eprintln!("error: {error}");
                    return 1;
                }
            };
            let mut stdout = io::stdout();
            match command::run_model_series(&series_args, &mut stdout) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "research" => {
            let mut arguments = Vec::with_capacity(args.len().saturating_sub(1));
            for argument in &args[1..] {
                match argument.to_str() {
                    Some(value) => arguments.push(value.to_string()),
                    None => {
                        eprintln!("error: argument is not valid UTF-8");
                        return 1;
                    }
                }
            }
            match command::parse_research_arguments(&arguments) {
                Ok(Some(Command::ResearchExport(export))) => {
                    let mut stdout = io::stdout();
                    match command::run_research_export(&export, &mut stdout) {
                        Ok(()) => 0,
                        Err(error) => {
                            eprintln!("error: {error}");
                            1
                        }
                    }
                }
                Ok(Some(Command::ResearchSummary(path))) => {
                    let mut stdout = io::stdout();
                    match command::run_research_summary(&path, &mut stdout) {
                        Ok(()) => 0,
                        Err(error) => {
                            eprintln!("error: {error}");
                            1
                        }
                    }
                }
                Ok(_) => {
                    eprintln!(
                        "error: usage: research export <csv-path> <dataset> --data-version <id> [--root <dir>]"
                    );
                    1
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "store" => {
            let mut arguments = Vec::with_capacity(args.len().saturating_sub(1));
            for argument in &args[1..] {
                match argument.to_str() {
                    Some(value) => arguments.push(value.to_string()),
                    None => {
                        eprintln!("error: argument is not valid UTF-8");
                        return 1;
                    }
                }
            }
            match command::parse_store_arguments(&arguments) {
                Ok(Some(Command::StoreInit(path))) => {
                    let mut stdout = io::stdout();
                    match command::run_store_init(&path, &mut stdout) {
                        Ok(()) => 0,
                        Err(error) => {
                            eprintln!("error: {error}");
                            1
                        }
                    }
                }
                Ok(Some(Command::StoreIngest(args))) => {
                    let mut stdout = io::stdout();
                    match command::run_store_ingest(&args, &mut stdout) {
                        Ok(()) => 0,
                        Err(error) => {
                            eprintln!("error: {error}");
                            1
                        }
                    }
                }
                Ok(Some(Command::StoreVerify(args))) => {
                    let mut stdout = io::stdout();
                    match command::run_store_verify(&args, &mut stdout) {
                        Ok(()) => 0,
                        Err(error) => {
                            eprintln!("error: {error}");
                            1
                        }
                    }
                }
                Ok(Some(Command::StoreList(args))) => {
                    let mut stdout = io::stdout();
                    match command::run_store_list(&args, &mut stdout) {
                        Ok(()) => 0,
                        Err(error) => {
                            eprintln!("error: {error}");
                            1
                        }
                    }
                }
                Ok(Some(Command::StoreShow(args))) => {
                    let mut stdout = io::stdout();
                    match command::run_store_show(&args, &mut stdout) {
                        Ok(()) => 0,
                        Err(error) => {
                            eprintln!("error: {error}");
                            1
                        }
                    }
                }
                Ok(Some(Command::StoreRemove(args))) => {
                    let mut stdout = io::stdout();
                    match command::run_store_remove(&args, &mut stdout) {
                        Ok(()) => 0,
                        Err(error) => {
                            eprintln!("error: {error}");
                            1
                        }
                    }
                }
                Ok(_) => {
                    eprintln!(
                        "error: usage: store init [--db <path>] | store ingest <csv-path> --dataset <name> --data-version <id> [--db <path>] | store verify <csv-path> --dataset <name> --data-version <id> [--db <path>] | store list [--db <path>] | store show <dataset> <table> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>] | store remove <dataset> [--db <path>]"
                    );
                    1
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    1
                }
            }
        }
        "db" => {
            let mut arguments = Vec::with_capacity(args.len().saturating_sub(1));
            for argument in &args[1..] {
                let Some(argument) = argument.to_str() else {
                    eprintln!("error: db argument is not valid UTF-8");
                    return 1;
                };
                arguments.push(argument.to_owned());
            }
            match command::parse_db_arguments(&arguments) {
                Ok(Some(Command::Db(args))) => {
                    let mut stdout = io::stdout();
                    match command::run_db(&args, &mut stdout) {
                        Ok(()) => 0,
                        Err(error) => {
                            eprintln!("error: {error}");
                            1
                        }
                    }
                }
                Ok(_) => {
                    eprintln!(
                        "error: usage: db series <dataset> [--timeframe <tf>] [--limit <n>] [--db <path>] | db bars <dataset> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>] | db metrics <dataset> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>] | db layer3 <dataset> [--bar <n>] [--limit <n>] [--db <path>]"
                    );
                    1
                }
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
            let mut stderr = io::stderr();
            match command::run_fetch(&fetch_args, &mut stdout, &mut stderr) {
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
