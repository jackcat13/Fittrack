package com.fittrack.intellij.ui

import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.ProjectActivity

class FitTrackStartupActivity : ProjectActivity {
    override suspend fun execute(project: Project) {
        FitTrackDashboardService.getInstance(project)
    }
}

