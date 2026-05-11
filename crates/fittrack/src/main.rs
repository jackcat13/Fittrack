use std::env;
use std::fs;
use std::path::PathBuf;

use fittrack::{ExerciseCatalog, compile_document_with_catalog, render_json};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(help());
    };

    match command.as_str() {
        "compile" => {
            let input = args.next().ok_or_else(help)?;
            let mut catalog_path = None;
            let mut output = None;
            while let Some(arg) = args.next() {
                if arg == "-o" || arg == "--output" {
                    output = args.next().map(PathBuf::from);
                } else if arg == "--exercises" {
                    catalog_path = args.next().map(PathBuf::from);
                } else {
                    return Err(format!("Unknown argument: {arg}\n\n{}", help()));
                }
            }

            let source = fs::read_to_string(&input)
                .map_err(|err| format!("Could not read {input}: {err}"))?;
            let catalog = catalog_path
                .as_ref()
                .map(|path| load_catalog(path))
                .transpose()?;
            let compiled = compile_document_with_catalog(&source, catalog.as_ref())?;
            let json = render_json(&compiled);

            if let Some(path) = output {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|err| format!("Could not create {}: {err}", parent.display()))?;
                }
                fs::write(&path, json)
                    .map_err(|err| format!("Could not write {}: {err}", path.display()))?;
            } else {
                println!("{json}");
            }
            Ok(())
        }
        _ => Err(help()),
    }
}

fn load_catalog(path: &PathBuf) -> Result<ExerciseCatalog, String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("Could not read {}: {err}", path.display()))?;
    ExerciseCatalog::parse(&source)
        .map_err(|err| format!("Could not parse {}: {err}", path.display()))
}

fn help() -> String {
    "Usage: fittrack compile <input.fit> [--exercises exercises.txt] [-o output.json]".to_string()
}
