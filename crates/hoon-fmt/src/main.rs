//! Hoon formatter CLI.
//!
//! Usage:
//!   hoonfmt [OPTIONS] <FILES>...
//!
//! Options:
//!   -w, --write       Write formatted output back to files
//!   --check           Exit non-zero if changes needed
//!   --max-width <N>   Maximum line width (default: 80)
//!   --diff            Show diff instead of formatted output

use clap::Parser;
use formatter::{format_source, FormatterConfig};
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "hoonfmt")]
#[command(about = "Format Hoon source code")]
#[command(version)]
struct Cli {
    /// Files to format (use - for stdin)
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Write formatted output back to files
    #[arg(short, long)]
    write: bool,

    /// Exit non-zero if any file would be reformatted
    #[arg(long)]
    check: bool,

    /// Maximum line width
    #[arg(long, default_value = "80")]
    max_width: usize,

    /// Preferred line width for breaking decisions
    #[arg(long, default_value = "56")]
    preferred_width: usize,

    /// Show diff instead of formatted output
    #[arg(long)]
    diff: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let config = FormatterConfig::default()
        .with_max_width(cli.max_width)
        .with_preferred_width(cli.preferred_width);

    let mut any_changes = false;
    let mut any_errors = false;

    for path in &cli.files {
        let result = if path.as_os_str() == "-" {
            format_stdin(&config)
        } else {
            format_file(path, &config, cli.write, cli.check, cli.diff)
        };

        match result {
            Ok(changed) => {
                if changed {
                    any_changes = true;
                }
            }
            Err(e) => {
                eprintln!("Error formatting {}: {}", path.display(), e);
                any_errors = true;
            }
        }
    }

    if any_errors {
        ExitCode::from(1)
    } else if cli.check && any_changes {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn format_stdin(config: &FormatterConfig) -> Result<bool, Box<dyn std::error::Error>> {
    let mut source = String::new();
    io::stdin().read_to_string(&mut source)?;

    let formatted = format_source(&source, config)?;
    print!("{}", formatted);

    Ok(source != formatted)
}

fn format_file(
    path: &PathBuf,
    config: &FormatterConfig,
    write: bool,
    check: bool,
    diff: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let formatted = format_source(&source, config)?;

    let changed = source != formatted;

    if changed {
        if check {
            println!("{}: would reformat", path.display());
        } else if write {
            fs::write(path, &formatted)?;
            println!("{}: formatted", path.display());
        } else if diff {
            print_diff(path, &source, &formatted);
        } else {
            print!("{}", formatted);
        }
    } else if !check && !write {
        print!("{}", formatted);
    }

    Ok(changed)
}

fn print_diff(path: &PathBuf, original: &str, formatted: &str) {
    println!("--- {}", path.display());
    println!("+++ {} (formatted)", path.display());

    let orig_lines: Vec<&str> = original.lines().collect();
    let fmt_lines: Vec<&str> = formatted.lines().collect();

    // Simple line-by-line diff
    let max_lines = orig_lines.len().max(fmt_lines.len());

    for i in 0..max_lines {
        let orig = orig_lines.get(i);
        let fmt = fmt_lines.get(i);

        match (orig, fmt) {
            (Some(o), Some(f)) if o != f => {
                println!("-{}", o);
                println!("+{}", f);
            }
            (Some(o), Some(f)) if o == f => {
                println!(" {}", o);
            }
            (Some(o), None) => {
                println!("-{}", o);
            }
            (None, Some(f)) => {
                println!("+{}", f);
            }
            _ => {}
        }
    }
}
