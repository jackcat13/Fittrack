package com.fittrack.intellij.ui

import com.intellij.openapi.components.Service
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindowManager
import com.intellij.ui.jcef.JBCefBrowser
import java.nio.file.Files
import java.nio.file.Path

@Service(Service.Level.PROJECT)
class FitTrackDashboardService(private val project: Project) {
    private var browser: JBCefBrowser? = null

    fun attach(browser: JBCefBrowser) {
        this.browser = browser
        refresh()
    }

    fun refresh() {
        val browser = browser ?: return
        browser.loadHTML(renderDashboardHtml())
        ToolWindowManager.getInstance(project).getToolWindow("Fit Track")?.show()
    }

    private fun renderDashboardHtml(): String {
        val basePath = project.basePath ?: return emptyState("Open a Fit & Track project to visualize data.")
        val dataPath = Path.of(basePath, "web", "data", "training.json")
        if (!Files.exists(dataPath)) {
            return emptyState("Compile a .fit file to generate web/data/training.json.")
        }

        val json = Files.readString(dataPath)
        return """
            <!doctype html>
            <html>
              <head>
                <meta charset="utf-8">
                <style>
                  body { margin: 0; padding: 18px; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #17201b; background: #f6f7f4; }
                  h1 { margin: 0 0 16px; font-size: 28px; }
                  h2 { margin: 0 0 10px; font-size: 16px; }
                  .stats { display: grid; grid-template-columns: repeat(4, minmax(120px, 1fr)); gap: 10px; margin-bottom: 14px; }
                  .card { background: white; border: 1px solid #d9ded8; border-radius: 8px; padding: 14px; }
                  .label { color: #65706a; font-size: 12px; }
                  .value { margin-top: 8px; font-size: 24px; font-weight: 800; }
                  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
                  svg { width: 100%; height: 220px; }
                  .bar { fill: #236b5c; }
                  .line { fill: none; stroke: #d05f3f; stroke-width: 3; }
                  .dot { fill: #d05f3f; }
                  .axis { fill: #65706a; font-size: 11px; }
                  .sessions { margin-top: 12px; display: grid; gap: 8px; }
                  .session-title { font-weight: 800; }
                  .sets { color: #65706a; font-size: 12px; margin-top: 4px; }
                </style>
              </head>
              <body>
                <h1>Fit & Track</h1>
                <div id="app"></div>
                <script>
                  const data = $json;
                  const app = document.querySelector("#app");
                  const fmt = new Intl.NumberFormat("en", { maximumFractionDigits: 1 });
                  const setCount = set => set.count || 1;
                  const formatSet = set => setCount(set) > 1
                    ? `${'$'}{setCount(set)} x ${'$'}{set.reps} x ${'$'}{fmt.format(set.weightKg)}kg`
                    : `${'$'}{set.reps} x ${'$'}{fmt.format(set.weightKg)}kg`;
                  const volume = training => training.exercises.reduce((total, exercise) =>
                    total + exercise.sets.reduce((sum, set) => sum + setCount(set) * set.reps * set.weightKg, 0), 0);
                  const maxVolume = Math.max(...data.trainings.map(volume), 1);
                  const bars = data.trainings.map((training, index) => {
                    const width = 100 / data.trainings.length;
                    const height = 160 * volume(training) / maxVolume;
                    const x = index * width + 2;
                    const y = 190 - height;
                    return `<rect class="bar" x="${'$'}{x}%" y="${'$'}{y}" width="${'$'}{Math.max(6, width - 4)}%" height="${'$'}{height}" rx="4"></rect>
                      <text class="axis" x="${'$'}{x}%" y="214">${'$'}{training.date.slice(5)}</text>`;
                  }).join("");
                  const sessions = data.trainings.slice().reverse().map(training => {
                    const rows = training.exercises.map(exercise =>
                      `<div><span class="session-title">${'$'}{exercise.name}</span><div class="sets">${'$'}{exercise.sets.map(formatSet).join(", ")}</div></div>`
                    ).join("");
                    return `<div class="card"><div class="session-title">${'$'}{training.title}</div><div class="sets">${'$'}{training.date}</div>${'$'}{rows}</div>`;
                  }).join("");
                  app.innerHTML = `
                    <div class="stats">
                      <div class="card"><div class="label">Sessions</div><div class="value">${'$'}{fmt.format(data.summary.totalTrainings)}</div></div>
                      <div class="card"><div class="label">Strength Sets</div><div class="value">${'$'}{fmt.format(data.summary.totalSets)}</div></div>
                      <div class="card"><div class="label">Total Volume</div><div class="value">${'$'}{fmt.format(data.summary.totalVolumeKg)} kg</div></div>
                      <div class="card"><div class="label">Cardio</div><div class="value">${'$'}{fmt.format(data.summary.totalCardioKm)} km</div></div>
                    </div>
                    <div class="grid">
                      <div class="card"><h2>Volume by session</h2><svg viewBox="0 0 760 220">${'$'}{bars}</svg></div>
                      <div class="card"><h2>Recent sessions</h2><div class="sessions">${'$'}{sessions}</div></div>
                    </div>`;
                </script>
              </body>
            </html>
        """.trimIndent()
    }

    private fun emptyState(message: String): String =
        "<html><body style=\"font-family: sans-serif; padding: 18px;\"><h2>Fit & Track</h2><p>$message</p></body></html>"

    companion object {
        fun getInstance(project: Project): FitTrackDashboardService = project.service()
    }
}
