use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct ExerciseCatalog {
    values: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Training {
    pub date: String,
    pub title: String,
    pub exercises: Vec<Exercise>,
    pub cardio: Vec<Cardio>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exercise {
    pub name: String,
    pub sets: Vec<Set>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Set {
    pub count: u32,
    pub reps: u32,
    pub weight_kg: f32,
    pub rpe: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cardio {
    pub kind: String,
    pub distance_km: f32,
    pub duration_seconds: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledTraining {
    pub trainings: Vec<Training>,
    pub summary: Summary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub total_trainings: usize,
    pub total_sets: usize,
    pub total_volume_kg: f32,
    pub total_cardio_km: f32,
}

impl ExerciseCatalog {
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut values = BTreeSet::new();

        for (index, raw_line) in source.lines().enumerate() {
            let line_no = index + 1;
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.contains('"') {
                return Err(format!(
                    "Exercise catalog line {line_no}: write one exercise name per line without quotes"
                ));
            }

            values.insert(line.to_string());
        }

        if values.is_empty() {
            return Err("Exercise catalog cannot be empty".to_string());
        }

        Ok(Self { values })
    }

    pub fn contains(&self, name: &str) -> bool {
        self.values.contains(name)
    }

    pub fn values(&self) -> impl Iterator<Item = &str> {
        self.values.iter().map(String::as_str)
    }
}

pub fn compile_document(source: &str) -> Result<CompiledTraining, String> {
    compile_document_with_catalog(source, None)
}

pub fn compile_document_with_catalog(
    source: &str,
    catalog: Option<&ExerciseCatalog>,
) -> Result<CompiledTraining, String> {
    let trainings = parse_document_with_catalog(source, catalog)?;
    let mut total_sets = 0;
    let mut total_volume_kg = 0.0;
    let mut total_cardio_km = 0.0;

    for training in &trainings {
        for exercise in &training.exercises {
            for set in &exercise.sets {
                total_sets += set.count as usize;
                total_volume_kg += set_volume(set);
            }
        }
        for cardio in &training.cardio {
            total_cardio_km += cardio.distance_km;
        }
    }

    Ok(CompiledTraining {
        summary: Summary {
            total_trainings: trainings.len(),
            total_sets,
            total_volume_kg,
            total_cardio_km,
        },
        trainings,
    })
}

pub fn parse_document(source: &str) -> Result<Vec<Training>, String> {
    parse_document_with_catalog(source, None)
}

pub fn parse_document_with_catalog(
    source: &str,
    catalog: Option<&ExerciseCatalog>,
) -> Result<Vec<Training>, String> {
    let mut trainings = Vec::new();
    let mut current_training: Option<Training> = None;
    let mut current_exercise: Option<Exercise> = None;

    for (index, raw_line) in source.lines().enumerate() {
        let line_no = index + 1;
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("training ") {
            flush_exercise(&mut current_training, &mut current_exercise);
            if let Some(training) = current_training.take() {
                trainings.push(training);
            }
            current_training = Some(parse_training_header(line, line_no)?);
        } else if line.starts_with("exercise ") {
            let training = current_training.as_mut().ok_or_else(|| {
                format!("Line {line_no}: exercise must appear inside a training block")
            })?;
            if let Some(exercise) = current_exercise.take() {
                training.exercises.push(exercise);
            }
            let name = parse_exercise_name(line, line_no)?;
            validate_exercise_name(&name, catalog, line_no)?;
            current_exercise = Some(Exercise {
                name,
                sets: Vec::new(),
            });
        } else if line.starts_with("set ") {
            let exercise = current_exercise
                .as_mut()
                .ok_or_else(|| format!("Line {line_no}: set must appear inside an exercise"))?;
            exercise.sets.push(parse_set(line, line_no)?);
        } else if line.starts_with("cardio ") {
            flush_exercise(&mut current_training, &mut current_exercise);
            let training = current_training
                .as_mut()
                .ok_or_else(|| format!("Line {line_no}: cardio must appear inside a training"))?;
            training.cardio.push(parse_cardio(line, line_no)?);
        } else if line.starts_with("note ") {
            flush_exercise(&mut current_training, &mut current_exercise);
            let training = current_training
                .as_mut()
                .ok_or_else(|| format!("Line {line_no}: note must appear inside a training"))?;
            training
                .notes
                .push(parse_quoted_after_keyword(line, "note", line_no)?);
        } else {
            return Err(format!("Line {line_no}: unknown statement `{line}`"));
        }
    }

    flush_exercise(&mut current_training, &mut current_exercise);
    if let Some(training) = current_training {
        trainings.push(training);
    }

    if trainings.is_empty() {
        return Err("No training entries found".to_string());
    }

    Ok(trainings)
}

fn validate_exercise_name(
    name: &str,
    catalog: Option<&ExerciseCatalog>,
    line_no: usize,
) -> Result<(), String> {
    let Some(catalog) = catalog else {
        return Ok(());
    };

    if catalog.contains(name) {
        return Ok(());
    }

    let allowed = catalog.values().collect::<Vec<_>>().join(", ");
    Err(format!(
        "Line {line_no}: unknown exercise `{name}`. Allowed values: {allowed}"
    ))
}

fn set_volume(set: &Set) -> f32 {
    set.count as f32 * set.reps as f32 * set.weight_kg
}

pub fn render_json(compiled: &CompiledTraining) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"summary\": {\n");
    out.push_str(&format!(
        "    \"totalTrainings\": {},\n    \"totalSets\": {},\n    \"totalVolumeKg\": {},\n    \"totalCardioKm\": {}\n",
        compiled.summary.total_trainings,
        compiled.summary.total_sets,
        fmt_num(compiled.summary.total_volume_kg),
        fmt_num(compiled.summary.total_cardio_km)
    ));
    out.push_str("  },\n");
    out.push_str("  \"trainings\": [\n");

    for (training_index, training) in compiled.trainings.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"date\": \"{}\",\n      \"title\": \"{}\",\n",
            escape_json(&training.date),
            escape_json(&training.title)
        ));
        out.push_str("      \"exercises\": [\n");
        for (exercise_index, exercise) in training.exercises.iter().enumerate() {
            out.push_str("        {\n");
            out.push_str(&format!(
                "          \"name\": \"{}\",\n          \"sets\": [\n",
                escape_json(&exercise.name)
            ));
            for (set_index, set) in exercise.sets.iter().enumerate() {
                let rpe = set.rpe.map(fmt_num).unwrap_or_else(|| "null".to_string());
                out.push_str(&format!(
                    "            {{ \"count\": {}, \"reps\": {}, \"weightKg\": {}, \"rpe\": {} }}{}\n",
                    set.count,
                    set.reps,
                    fmt_num(set.weight_kg),
                    rpe,
                    comma(set_index, exercise.sets.len())
                ));
            }
            out.push_str("          ]\n");
            out.push_str(&format!(
                "        }}{}\n",
                comma(exercise_index, training.exercises.len())
            ));
        }
        out.push_str("      ],\n");
        out.push_str("      \"cardio\": [\n");
        for (cardio_index, cardio) in training.cardio.iter().enumerate() {
            out.push_str(&format!(
                "        {{ \"kind\": \"{}\", \"distanceKm\": {}, \"durationSeconds\": {} }}{}\n",
                escape_json(&cardio.kind),
                fmt_num(cardio.distance_km),
                cardio.duration_seconds,
                comma(cardio_index, training.cardio.len())
            ));
        }
        out.push_str("      ],\n");
        out.push_str("      \"notes\": [");
        for (note_index, note) in training.notes.iter().enumerate() {
            if note_index > 0 {
                out.push_str(", ");
            }
            out.push('"');
            out.push_str(&escape_json(note));
            out.push('"');
        }
        out.push_str("]\n");
        out.push_str(&format!(
            "    }}{}\n",
            comma(training_index, compiled.trainings.len())
        ));
    }

    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}

fn flush_exercise(training: &mut Option<Training>, exercise: &mut Option<Exercise>) {
    if let (Some(training), Some(exercise)) = (training.as_mut(), exercise.take()) {
        training.exercises.push(exercise);
    }
}

fn parse_training_header(line: &str, line_no: usize) -> Result<Training, String> {
    let rest = line
        .strip_prefix("training ")
        .ok_or_else(|| format!("Line {line_no}: expected training header"))?;
    let (date, title_part) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Line {line_no}: expected `training YYYY-MM-DD \"Title\"`"))?;

    validate_date(date, line_no)?;

    Ok(Training {
        date: date.to_string(),
        title: parse_quoted(title_part.trim(), line_no)?,
        exercises: Vec::new(),
        cardio: Vec::new(),
        notes: Vec::new(),
    })
}

fn parse_quoted_after_keyword(line: &str, keyword: &str, line_no: usize) -> Result<String, String> {
    let rest = line
        .strip_prefix(keyword)
        .and_then(|value| value.strip_prefix(' '))
        .ok_or_else(|| format!("Line {line_no}: expected `{keyword} \"...\"`"))?;
    parse_quoted(rest.trim(), line_no)
}

fn parse_exercise_name(line: &str, line_no: usize) -> Result<String, String> {
    let rest = line
        .strip_prefix("exercise ")
        .ok_or_else(|| format!("Line {line_no}: expected exercise statement"))?
        .trim();

    if rest.is_empty() {
        return Err(format!("Line {line_no}: expected exercise name"));
    }

    if rest.starts_with('"') || rest.ends_with('"') {
        return parse_quoted(rest, line_no);
    }

    Ok(rest.to_string())
}

fn parse_quoted(input: &str, line_no: usize) -> Result<String, String> {
    if !input.starts_with('"') || !input.ends_with('"') || input.len() < 2 {
        return Err(format!("Line {line_no}: expected quoted text"));
    }
    Ok(input[1..input.len() - 1].replace("\\\"", "\""))
}

fn parse_set(line: &str, line_no: usize) -> Result<Set, String> {
    let rest = line
        .strip_prefix("set ")
        .ok_or_else(|| format!("Line {line_no}: expected set statement"))?;
    let parts: Vec<&str> = rest.split_whitespace().collect();

    let (count, reps_part, weight_part, rpe_part) = match parts.as_slice() {
        [reps, "x", weight] => (1, *reps, *weight, None),
        [reps, "x", weight, rpe] => (1, *reps, *weight, Some(*rpe)),
        [count, "x", reps, "x", weight] => (parse_count(count, line_no)?, *reps, *weight, None),
        [count, "x", reps, "x", weight, rpe] => {
            (parse_count(count, line_no)?, *reps, *weight, Some(*rpe))
        }
        _ => {
            return Err(format!(
                "Line {line_no}: expected `set [<count> x] <reps> x <weight>kg [@rpe]`"
            ));
        }
    };

    let reps = reps_part
        .parse::<u32>()
        .map_err(|_| format!("Line {line_no}: invalid reps `{reps_part}`"))?;
    let weight_kg = parse_kg(weight_part, line_no)?;
    let rpe = rpe_part
        .map(|value| parse_rpe(value, line_no))
        .transpose()?;

    Ok(Set {
        count,
        reps,
        weight_kg,
        rpe,
    })
}

fn parse_count(input: &str, line_no: usize) -> Result<u32, String> {
    let count = input
        .parse::<u32>()
        .map_err(|_| format!("Line {line_no}: invalid set count `{input}`"))?;
    if count == 0 {
        return Err(format!("Line {line_no}: set count must be greater than 0"));
    }
    Ok(count)
}

fn parse_cardio(line: &str, line_no: usize) -> Result<Cardio, String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 4 {
        return Err(format!(
            "Line {line_no}: expected `cardio <kind> <distance>km <mm:ss>`"
        ));
    }
    Ok(Cardio {
        kind: parts[1].to_string(),
        distance_km: parse_km(parts[2], line_no)?,
        duration_seconds: parse_duration(parts[3], line_no)?,
    })
}

fn parse_kg(input: &str, line_no: usize) -> Result<f32, String> {
    parse_number_with_suffix(input, "kg", line_no)
}

fn parse_km(input: &str, line_no: usize) -> Result<f32, String> {
    parse_number_with_suffix(input, "km", line_no)
}

fn parse_number_with_suffix(input: &str, suffix: &str, line_no: usize) -> Result<f32, String> {
    let number = input
        .strip_suffix(suffix)
        .ok_or_else(|| format!("Line {line_no}: expected `{input}` to end with {suffix}"))?;
    number
        .parse::<f32>()
        .map_err(|_| format!("Line {line_no}: invalid number `{number}`"))
}

fn parse_rpe(input: &str, line_no: usize) -> Result<f32, String> {
    let value = input
        .strip_prefix('@')
        .ok_or_else(|| format!("Line {line_no}: expected RPE like @8"))?;
    value
        .parse::<f32>()
        .map_err(|_| format!("Line {line_no}: invalid RPE `{value}`"))
}

fn parse_duration(input: &str, line_no: usize) -> Result<u32, String> {
    let (minutes, seconds) = input
        .split_once(':')
        .ok_or_else(|| format!("Line {line_no}: expected duration mm:ss"))?;
    let minutes = minutes
        .parse::<u32>()
        .map_err(|_| format!("Line {line_no}: invalid minutes `{minutes}`"))?;
    let seconds = seconds
        .parse::<u32>()
        .map_err(|_| format!("Line {line_no}: invalid seconds `{seconds}`"))?;
    if seconds >= 60 {
        return Err(format!("Line {line_no}: seconds must be below 60"));
    }
    Ok(minutes * 60 + seconds)
}

fn validate_date(input: &str, line_no: usize) -> Result<(), String> {
    let parts: Vec<&str> = input.split('-').collect();
    if parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts[2].len() != 2
        || parts
            .iter()
            .any(|part| !part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return Err(format!("Line {line_no}: expected date as YYYY-MM-DD"));
    }
    Ok(())
}

fn escape_json(input: &str) -> String {
    input
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<char>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            _ => vec![ch],
        })
        .collect()
}

fn fmt_num(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn comma(index: usize, len: usize) -> &'static str {
    if index + 1 == len { "" } else { "," }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_strength_and_cardio() {
        let source = r#"
training 2026-05-01 "Push"
  exercise "Bench Press"
    set 5 x 60kg @8
    set 5 x 62.5kg @8.5
  cardio run 5km 27:30
  note "smooth session"
"#;

        let compiled = compile_document(source).unwrap();

        assert_eq!(compiled.summary.total_trainings, 1);
        assert_eq!(compiled.summary.total_sets, 2);
        assert_eq!(compiled.summary.total_volume_kg, 612.5);
        assert_eq!(compiled.summary.total_cardio_km, 5.0);
        assert!(render_json(&compiled).contains("\"Bench Press\""));
    }

    #[test]
    fn validates_exercises_against_catalog() {
        let catalog = ExerciseCatalog::parse(
            r#"
Bench Press
Back Squat
"#,
        )
        .unwrap();
        let source = r#"
training 2026-05-01 "Push"
  exercise "Bench Press"
    set 5 x 60kg @8
"#;

        let compiled = compile_document_with_catalog(source, Some(&catalog)).unwrap();

        assert_eq!(compiled.summary.total_sets, 1);
    }

    #[test]
    fn accepts_unquoted_exercise_names() {
        let catalog = ExerciseCatalog::parse("Bench Press").unwrap();
        let source = r#"
training 2026-05-01 "Push"
  exercise Bench Press
    set 5 x 60kg @8
"#;

        let compiled = compile_document_with_catalog(source, Some(&catalog)).unwrap();

        assert_eq!(compiled.trainings[0].exercises[0].name, "Bench Press");
    }

    #[test]
    fn accepts_repeated_sets() {
        let source = r#"
training 2026-05-01 "Push"
  exercise Bench Press
    set 3 x 5 x 60kg @8
"#;

        let compiled = compile_document(source).unwrap();
        let set = compiled.trainings[0].exercises[0].sets[0];

        assert_eq!(set.count, 3);
        assert_eq!(set.reps, 5);
        assert_eq!(compiled.summary.total_sets, 3);
        assert_eq!(compiled.summary.total_volume_kg, 900.0);
        assert!(render_json(&compiled).contains("\"count\": 3"));
    }

    #[test]
    fn rejects_exercises_missing_from_catalog() {
        let catalog = ExerciseCatalog::parse("Bench Press").unwrap();
        let source = r#"
training 2026-05-01 "Pull"
  exercise "Deadlift"
    set 5 x 100kg @8
"#;

        let err = compile_document_with_catalog(source, Some(&catalog)).unwrap_err();

        assert!(err.contains("unknown exercise `Deadlift`"));
        assert!(err.contains("Bench Press"));
    }

    #[test]
    fn reports_line_numbers() {
        let err = compile_document("training 2026/05/01 \"Bad\"").unwrap_err();

        assert!(err.contains("Line 1"));
    }
}
