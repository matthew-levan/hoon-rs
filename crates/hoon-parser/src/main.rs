use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use clap::Parser as ClapParser;
use hoon_parser::native_parser;
use hoon_parser::utils::LineMap;

#[derive(ClapParser, Debug)]
struct Cli {
    /// input file or directory (required unless --test)
    #[arg(value_name = "PATH", required = false)]
    input: Option<PathBuf>,

    /// disable debug traces
    #[arg(long = "no-dbug", short = 'b')]
    no_dbug: bool,
    /// disable debug traces
    #[arg(long = "preserve-syntax", short = 'p')]
    preserve_syntax: bool,

    /// output file (defaults to stdout)
    #[arg(long = "out", short = 'o', value_name = "PATH")]
    out: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let input = cli.input.clone().unwrap_or_else(|| {
        eprintln!("Input file or directory is required unless --test");
        std::process::exit(2);
    });

    let inputs = collect_inputs(&input);

    let start = Instant::now();

    for source_path in inputs {
        run_parser(
            &source_path,
            !cli.no_dbug,
            cli.preserve_syntax,
            cli.out.clone(),
        );
    }

    println!("total running time {:?} ", start.elapsed());
}

pub fn collect_inputs(path: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_inputs_inner(path, &mut files);
    files.sort();
    files
}

fn collect_inputs_inner(path: &PathBuf, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("hoon") {
            out.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        let entries = std::fs::read_dir(path).unwrap_or_else(|e| {
            eprintln!("Failed to read directory '{}': {}", path.display(), e);
            std::process::exit(1);
        });

        for entry in entries {
            let entry = entry.unwrap_or_else(|e| {
                eprintln!(
                    "Failed to read directory entry in '{}': {}",
                    path.display(),
                    e
                );
                std::process::exit(1);
            });

            collect_inputs_inner(&entry.path(), out);
        }
    } else {
        eprintln!("Invalid input path: {}", path.display());
        std::process::exit(2);
    }
}

fn run_parser(source_path: &PathBuf, dbug: bool, preserve_syntax: bool, out: Option<PathBuf>) {
    let source = fs::read_to_string(source_path).unwrap_or_else(|err| {
        eprintln!("Error reading file '{}': {}", source_path.display(), err);
        std::process::exit(1);
    });

    let start = Instant::now();

    let wer: Vec<String> = source_path
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();

    let linemap = Arc::new(LineMap::new(&source));

    match native_parser(wer, dbug, linemap, preserve_syntax)
        .parse(source.as_str())
        .into_result()
    {
        Ok(res) => {
            let took = start.elapsed();

            let start2 = Instant::now();
            let took2 = start2.elapsed();

            let json = serde_json::to_string_pretty(&res).expect("AST JSON serialization failed");

            match &out {
                None => {
                    println!("{json}");
                }
                Some(out) if out.is_dir() => {
                    let mut out_file =
                        out.join(source_path.file_name().expect("input has no filename"));
                    out_file.set_extension("json");
                    fs::write(&out_file, json).unwrap_or_else(|e| {
                        eprintln!("Failed to write '{}': {}", out_file.display(), e);
                        std::process::exit(1);
                    });
                }
                Some(out) => {
                    fs::write(out, json).unwrap_or_else(|e| {
                        eprintln!("Failed to write '{}': {}", out.display(), e);
                        std::process::exit(1);
                    });
                }
            }

            println!(
                "parsed file {}, took {:?}, noun creation time {:?}",
                source_path.display(),
                took,
                took2
            );
        }

        Err(errs) => {
            for err in errs {
                let span = err.span().into_range();
                let file_id = source_path.to_string_lossy().to_string();

                Report::build(ReportKind::Error, (file_id.clone(), span.clone()))
                    .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                    .with_label(
                        Label::new((file_id.clone(), span))
                            .with_message(err.reason().to_string())
                            .with_color(Color::Red),
                    )
                    .finish()
                    .eprint((file_id.clone(), Source::from(source.clone())))
                    .unwrap();
            }
        }
    };
}
