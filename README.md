# Fit & Track

Fit & Track is a tiny fitness-log DSL and dashboard starter. Write training sessions in a fast text format, compile them with Rust, then inspect progress through a browser dashboard.

## DSL

```fittrack
training 2026-05-01 "Push Strength"
  exercise "Bench Press"
    set 5 x 60kg @8
    set 5 x 62.5kg @8.5
  cardio run 5km 27:30
  note "Bench moved well."
```

Supported statements:

- `training YYYY-MM-DD "Title"` starts a session.
- `exercise "Name"` starts a strength movement. When compiled with `--exercises`, `Name` must be present in the external exercise catalog.
- `set reps x weightkg [@rpe]` records one strength set.
- `cardio kind distancekm mm:ss` records cardio work.
- `note "Text"` adds a session note.

## Exercise Catalog

Exercise names can be treated like an external enum. Put the allowed values in a plain text file, one exercise per line:

```txt
Back Squat
Bench Press
Deadlift
```

The starter catalog lives at `config/exercises.txt`. Personalize that file to control which values are accepted after `exercise`.

## Run

Compile the sample training log:

```sh
cargo run -p fittrack -- compile examples/may.fit --exercises config/exercises.txt -o web/data/training.json
```

Open the terminal app:

```sh
cargo run -p fittrack -- tui examples/may.fit --exercises config/exercises.txt -o web/data/training.json
```

Inside the TUI:

- `Ctrl-E` switches between the `.fit` editor and the exercise catalog editor.
- `Ctrl-N` / `Ctrl-P` moves through the filtered completion choices.
- `Tab` accepts the selected completion.
- `F5` compiles the current buffers to JSON and opens the dashboard view.
- `Ctrl-B` toggles between editor and dashboard.
- In the dashboard, `Left` / `Right` chooses the year, month, or exercise filter.
- In the dashboard, `Space` / `Enter` cycles the focused filter, and `Backspace` clears it.
- In the dashboard, `Up` / `Down` / `PageUp` / `PageDown` or the mouse wheel scrolls the page.
- `Ctrl-S` saves both edited files.
- `Ctrl-C` quits.

Open the dashboard:

```sh
python3 -m http.server 8080 --directory web
```

Then visit `http://localhost:8080`.

## Editor Support

The `vscode/` folder contains a TextMate grammar and language configuration that can be packaged into a VS Code extension later. It already captures the language shape for highlighting: keywords, strings, numbers, units, comments, and RPE values.

The `intellij/` folder contains lightweight JetBrains IDE support: an importable custom file type for `.fit` syntax highlighting and live templates for quickly writing training blocks, exercises, sets, and cardio entries.

The `intellij-plugin/` folder contains a real JetBrains plugin scaffold with `.fit` highlighting, Run-button compilation, and an in-IDE dashboard tool window.

## Next Milestones

- Add a real parser crate with richer diagnostics and recovery.
- Package VS Code support with snippets and completions.
- Add IntelliJ exercise completion backed by `config/exercises.txt`.
- Persist personal records and derived metrics in the compiler output.
- Add dashboard filters for date ranges, exercise groups, and cardio types.
