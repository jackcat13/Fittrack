const state = {
  data: null,
  selectedExercise: null,
  selectedYear: "",
  selectedMonth: "",
};

const els = {
  reload: document.querySelector("#reload"),
  yearFilter: document.querySelector("#year-filter"),
  monthFilter: document.querySelector("#month-filter"),
  totalTrainings: document.querySelector("#total-trainings"),
  totalSets: document.querySelector("#total-sets"),
  totalVolume: document.querySelector("#total-volume"),
  totalCardio: document.querySelector("#total-cardio"),
  volumeChart: document.querySelector("#volume-chart"),
  exerciseChart: document.querySelector("#exercise-chart"),
  exerciseSelect: document.querySelector("#exercise-select"),
  sessions: document.querySelector("#sessions"),
  sessionCount: document.querySelector("#session-count"),
};

els.reload.addEventListener("click", loadData);
els.yearFilter.addEventListener("input", (event) => {
  state.selectedYear = event.target.value;
  render();
});
els.monthFilter.addEventListener("change", (event) => {
  state.selectedMonth = event.target.value;
  render();
});
els.exerciseSelect.addEventListener("change", (event) => {
  state.selectedExercise = event.target.value;
  render();
});

loadData();

async function loadData() {
  try {
    const response = await fetch("./data/training.json", { cache: "no-store" });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    state.data = await response.json();
    renderPeriodOptions();
    const exercises = exerciseNames(state.data);
    state.selectedExercise = state.selectedExercise || exercises[0] || "";
    render();
  } catch (error) {
    state.data = { summary: {}, trainings: [] };
    renderEmpty(`Could not load data: ${error.message}`);
  }
}

function render() {
  const data = filteredData(state.data);
  const summary = summarize(data.trainings);
  const names = exerciseNames(data);
  if (!names.includes(state.selectedExercise)) {
    state.selectedExercise = names[0] || "";
  }

  els.totalTrainings.textContent = formatNumber(summary.totalTrainings || 0);
  els.totalSets.textContent = formatNumber(summary.totalSets || 0);
  els.totalVolume.textContent = `${formatNumber(summary.totalVolumeKg || 0)} kg`;
  els.totalCardio.textContent = `${formatNumber(summary.totalCardioKm || 0)} km`;
  els.sessionCount.textContent = `${data.trainings.length} logged`;

  renderExerciseOptions(data);
  renderVolumeChart(data);
  renderExerciseChart(data, state.selectedExercise);
  renderSessions(data);
}

function renderPeriodOptions() {
  const months = Array.from({ length: 12 }, (_, index) => String(index + 1).padStart(2, "0"));

  els.monthFilter.innerHTML = [
    `<option value="">All months</option>`,
    ...months.map((month) => `<option value="${escapeHtml(month)}">${escapeHtml(monthName(month))}</option>`),
  ].join("");

  els.yearFilter.value = state.selectedYear;
  els.monthFilter.value = state.selectedMonth;
}

function renderExerciseOptions(data) {
  const names = exerciseNames(data);
  els.exerciseSelect.innerHTML = names.length
    ? names.map((name) => `<option value="${escapeHtml(name)}">${escapeHtml(name)}</option>`).join("")
    : `<option value="">No exercises</option>`;
  els.exerciseSelect.value = state.selectedExercise || names[0] || "";
  els.exerciseSelect.disabled = names.length === 0;
}

function renderVolumeChart(data) {
  const points = data.trainings.map((training) => ({
    label: shortDate(training.date),
    value: trainingVolume(training),
  }));
  els.volumeChart.innerHTML = barChart(points, "kg");
}

function renderExerciseChart(data, exerciseName) {
  const points = data.trainings
    .map((training) => {
      const exercise = training.exercises.find((item) => item.name === exerciseName);
      if (!exercise) return null;
      return {
        label: shortDate(training.date),
        weight: bestEstimatedMax(exercise),
        volume: exerciseVolume(exercise),
      };
    })
    .filter(Boolean);

  els.exerciseChart.innerHTML = exerciseProgressionCharts(points);
}

function renderSessions(data) {
  if (data.trainings.length === 0) {
    els.sessions.innerHTML = `<div class="empty">No sessions match these filters</div>`;
    return;
  }

  els.sessions.innerHTML = data.trainings
    .slice()
    .reverse()
    .map((training) => {
      const exercises = training.exercises
        .map((exercise) => {
          const setText = exercise.sets
            .map((set) => `${set.reps} x ${formatNumber(set.weightKg)}kg`)
            .join(", ");
          return `<div class="exercise-row"><strong>${escapeHtml(exercise.name)}</strong><span class="set-list">${escapeHtml(setText)}</span></div>`;
        })
        .join("");
      const cardio = training.cardio
        .map((item) => `${item.kind} ${formatNumber(item.distanceKm)}km in ${formatDuration(item.durationSeconds)}`)
        .join(" · ");
      const meta = [formatDate(training.date), cardio].filter(Boolean).join(" · ");

      return `
        <article class="session">
          <div>
            <h3>${escapeHtml(training.title)}</h3>
            <p class="session-meta">${escapeHtml(meta)}</p>
          </div>
          <div class="exercise-list">${exercises}</div>
        </article>
      `;
    })
    .join("");
}

function renderEmpty(message) {
  els.volumeChart.innerHTML = `<div class="empty">${escapeHtml(message)}</div>`;
  els.exerciseChart.innerHTML = `<div class="empty">${escapeHtml(message)}</div>`;
  els.sessions.innerHTML = `<div class="empty">${escapeHtml(message)}</div>`;
}

function barChart(points, unit) {
  if (points.length === 0) return `<div class="empty">No data compiled yet</div>`;

  const width = 760;
  const height = 260;
  const pad = 34;
  const max = Math.max(...points.map((point) => point.value), 1);
  const barWidth = (width - pad * 2) / points.length;

  const bars = points
    .map((point, index) => {
      const x = pad + index * barWidth + 8;
      const h = ((height - pad * 2) * point.value) / max;
      const y = height - pad - h;
      return `
        <rect class="bar" x="${x}" y="${y}" width="${Math.max(12, barWidth - 16)}" height="${h}" rx="5"></rect>
        <text class="axis" x="${x}" y="${height - 10}">${escapeHtml(point.label)}</text>
        <text class="axis" x="${x}" y="${Math.max(16, y - 8)}">${formatNumber(point.value)} ${unit}</text>
      `;
    })
    .join("");

  return `<svg viewBox="0 0 ${width} ${height}" aria-hidden="true">
    <line x1="${pad}" y1="${height - pad}" x2="${width - pad}" y2="${height - pad}" stroke="#d9ded8"></line>
    ${bars}
  </svg>`;
}

function exerciseProgressionCharts(points) {
  if (points.length === 0) return `<div class="empty">No matching exercise data</div>`;

  return `
    <div class="metric-chart">
      <div class="metric-heading">
        <span class="chart-legend-swatch"></span>
        <strong>Estimated max</strong>
      </div>
      ${lineChart(points, "weight", "kg", "line", "dot")}
    </div>
    <div class="metric-chart">
      <div class="metric-heading">
        <span class="chart-legend-swatch volume-swatch"></span>
        <strong>Volume</strong>
      </div>
      ${lineChart(points, "volume", "kg", "line volume-line", "dot volume-dot")}
    </div>
  `;
}

function lineChart(points, valueKey, unit, lineClass, dotClass) {
  const width = 760;
  const height = 260;
  const pad = 36;
  const max = Math.max(...points.map((point) => point[valueKey]), 1);
  const step = points.length === 1 ? 0 : (width - pad * 2) / (points.length - 1);
  const coords = points.map((point, index) => ({
    ...point,
    x: points.length === 1 ? width / 2 : pad + index * step,
    y: height - pad - ((height - pad * 2) * point[valueKey]) / max,
  }));
  const path = coords.map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`).join(" ");
  const dots = coords
    .map(
      (point) => `
        <circle class="${dotClass}" cx="${point.x}" cy="${point.y}" r="5"></circle>
        <text class="axis" x="${point.x - 18}" y="${Math.max(16, point.y - 12)}">${formatNumber(point[valueKey])} ${unit}</text>
      `,
    )
    .join("");
  const labels = coords
    .map((point) => `<text class="axis" x="${point.x - 16}" y="${height - 10}">${escapeHtml(point.label)}</text>`)
    .join("");

  return `<svg viewBox="0 0 ${width} ${height}" aria-hidden="true">
    <line x1="${pad}" y1="${height - pad}" x2="${width - pad}" y2="${height - pad}" stroke="#d9ded8"></line>
    <path class="${lineClass}" d="${path}"></path>
    ${dots}
    ${labels}
  </svg>`;
}

function filteredData(data) {
  const selectedYear = els.yearFilter.value.trim();
  const selectedMonth = els.monthFilter.value;

  return {
    ...data,
    trainings: data.trainings.filter((training) => {
      const [year, month] = training.date.split("-");
      return (!selectedYear || year === selectedYear) && (!selectedMonth || month === selectedMonth);
    }),
  };
}

function summarize(trainings) {
  return trainings.reduce(
    (summary, training) => {
      summary.totalTrainings += 1;
      summary.totalSets += training.exercises.reduce((total, exercise) => total + exercise.sets.length, 0);
      summary.totalVolumeKg += trainingVolume(training);
      summary.totalCardioKm += training.cardio.reduce((total, item) => total + item.distanceKm, 0);
      return summary;
    },
    { totalTrainings: 0, totalSets: 0, totalVolumeKg: 0, totalCardioKm: 0 },
  );
}

function exerciseNames(data) {
  return [...new Set(data.trainings.flatMap((training) => training.exercises.map((exercise) => exercise.name)))].sort();
}

function trainingVolume(training) {
  return training.exercises.reduce((total, exercise) => total + exerciseVolume(exercise), 0);
}

function exerciseVolume(exercise) {
  return exercise.sets.reduce((sum, set) => sum + set.reps * set.weightKg, 0);
}

function bestEstimatedMax(exercise) {
  return Math.max(...exercise.sets.map((set) => set.weightKg * (1 + set.reps / 30)), 0);
}

function shortDate(value) {
  return value.slice(5);
}

function formatDate(value) {
  return new Intl.DateTimeFormat("en", { month: "short", day: "numeric", year: "numeric" }).format(new Date(`${value}T00:00:00`));
}

function formatDuration(seconds) {
  const minutes = Math.floor(seconds / 60);
  const rest = String(seconds % 60).padStart(2, "0");
  return `${minutes}:${rest}`;
}

function monthName(value) {
  return new Intl.DateTimeFormat("en", { month: "long" }).format(new Date(`2026-${value}-01T00:00:00`));
}

function formatNumber(value) {
  return new Intl.NumberFormat("en", { maximumFractionDigits: 1 }).format(value);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
