package com.fittrack.intellij.ui

import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefBrowser
import java.awt.BorderLayout
import javax.swing.JButton
import javax.swing.JPanel

class FitTrackToolWindowFactory : ToolWindowFactory {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val browser = JBCefBrowser()
        val refresh = JButton("Refresh")
        refresh.addActionListener {
            FitTrackDashboardService.getInstance(project).refresh()
        }

        val panel = JPanel(BorderLayout())
        panel.add(refresh, BorderLayout.NORTH)
        panel.add(browser.component, BorderLayout.CENTER)

        FitTrackDashboardService.getInstance(project).attach(browser)

        val content = ContentFactory.getInstance().createContent(panel, "Dashboard", false)
        toolWindow.contentManager.addContent(content)
    }
}

