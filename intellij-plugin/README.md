# Fit & Track IntelliJ Plugin

This is the real JetBrains plugin scaffold for Fit & Track.

## Features

- `.fit` file type registration.
- Lexer-backed syntax highlighting for keywords, comments, strings, numbers, units, and RPE values.
- Run configuration support so a `.fit` file can be compiled from IntelliJ's Run button.
- `Fit Track` tool window that renders `web/data/training.json` inside the IDE.

## Run In A Development IDE

From this folder:

```sh
gradle runIde
```

The first run downloads the IntelliJ Platform SDK and Gradle dependencies. A Gradle wrapper can be added later once the plugin build is pinned in CI.

## Expected Project Layout

The run configuration assumes it is opened at the Fit & Track repository root:

```txt
Cargo.toml
config/exercises.txt
examples/*.fit
web/data/training.json
```

When a `.fit` file is run, the plugin executes:

```sh
cargo run -p fittrack -- compile <file.fit> --exercises config/exercises.txt -o web/data/training.json
```

After a successful compile, the `Fit Track` tool window refreshes automatically.

