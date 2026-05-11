package com.fittrack.intellij.run

import com.fittrack.intellij.ui.FitTrackDashboardService
import com.intellij.execution.ExecutionException
import com.intellij.execution.Executor
import com.intellij.execution.configurations.CommandLineState
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.configurations.LocatableConfigurationBase
import com.intellij.execution.configurations.RunProfileState
import com.intellij.execution.process.OSProcessHandler
import com.intellij.execution.process.ProcessEvent
import com.intellij.execution.process.ProcessListener
import com.intellij.execution.runners.ExecutionEnvironment
import com.intellij.openapi.options.SettingsEditor
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.JDOMExternalizerUtil
import com.intellij.openapi.util.Key
import org.jdom.Element
import java.nio.file.Path
import javax.swing.JComponent
import javax.swing.JPanel

class FitTrackRunConfiguration(
    project: Project,
    factory: ConfigurationFactory,
    name: String,
) : LocatableConfigurationBase<RunProfileState>(project, factory, name) {
    var inputPath: String = ""
    var catalogPath: String = "config/exercises.txt"
    var outputPath: String = "web/data/training.json"

    override fun getState(executor: Executor, environment: ExecutionEnvironment): RunProfileState =
        FitTrackCommandLineState(environment, this)

    override fun getConfigurationEditor(): SettingsEditor<out FitTrackRunConfiguration> =
        object : SettingsEditor<FitTrackRunConfiguration>() {
            override fun resetEditorFrom(configuration: FitTrackRunConfiguration) = Unit
            override fun applyEditorTo(configuration: FitTrackRunConfiguration) = Unit
            override fun createEditor(): JComponent = JPanel()
        }

    override fun checkConfiguration() {
        if (inputPath.isBlank()) {
            throw ExecutionException("Choose a .fit training file to compile.")
        }
    }

    override fun writeExternal(element: Element) {
        super.writeExternal(element)
        JDOMExternalizerUtil.writeField(element, "inputPath", inputPath)
        JDOMExternalizerUtil.writeField(element, "catalogPath", catalogPath)
        JDOMExternalizerUtil.writeField(element, "outputPath", outputPath)
    }

    override fun readExternal(element: Element) {
        super.readExternal(element)
        inputPath = JDOMExternalizerUtil.readField(element, "inputPath") ?: ""
        catalogPath = JDOMExternalizerUtil.readField(element, "catalogPath") ?: "config/exercises.txt"
        outputPath = JDOMExternalizerUtil.readField(element, "outputPath") ?: "web/data/training.json"
    }
}

private class FitTrackCommandLineState(
    environment: ExecutionEnvironment,
    private val configuration: FitTrackRunConfiguration,
) : CommandLineState(environment) {
    override fun startProcess(): OSProcessHandler {
        val projectBase = configuration.project.basePath
            ?: throw ExecutionException("Fit & Track needs a project directory.")
        val commandLine = GeneralCommandLine()
            .withExePath("cargo")
            .withWorkDirectory(projectBase)
            .withParameters("run", "-p", "fittrack", "--", "compile", configuration.inputPath)

        val catalog = Path.of(projectBase, configuration.catalogPath).toFile()
        if (catalog.exists()) {
            commandLine.addParameters("--exercises", configuration.catalogPath)
        }

        commandLine.addParameters("-o", configuration.outputPath)

        val handler = OSProcessHandler(commandLine)
        handler.addProcessListener(object : ProcessListener {
            override fun processTerminated(event: ProcessEvent) {
                if (event.exitCode == 0) {
                    FitTrackDashboardService.getInstance(configuration.project).refresh()
                }
            }

            override fun startNotified(event: ProcessEvent) = Unit
            override fun processWillTerminate(event: ProcessEvent, willBeDestroyed: Boolean) = Unit
            override fun onTextAvailable(event: ProcessEvent, outputType: Key<*>) = Unit
        })
        return handler
    }
}

