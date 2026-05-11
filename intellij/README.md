# IntelliJ Support

This folder contains lightweight JetBrains IDE support for Fit & Track `.fit` files.

## Syntax Highlighting

Import `fileTypes/FitTrack.xml`:

1. Open IntelliJ IDEA settings.
2. Go to `Editor > File Types`.
3. Use the settings menu to import the file type XML.
4. Confirm that `*.fit` is mapped to `FitTrack`.

The file type highlights DSL keywords, line comments, quoted strings, numbers, and `kg`/`km` suffixes.

## Live Templates

Import `templates/FitTrack.xml` from `Editor > Live Templates`.

Included abbreviations:

- `training` creates a full training block.
- `exercise` creates an exercise with one set.
- `set` creates a strength set.
- `cardio` creates a cardio entry.

## Completion Roadmap

The external exercise catalog in `config/exercises.txt` is enforced by the Rust compiler. IntelliJ's custom file type import cannot dynamically read that file for completion. That will require a dedicated JetBrains plugin that reads the project catalog and contributes completion items after the `exercise` keyword.

