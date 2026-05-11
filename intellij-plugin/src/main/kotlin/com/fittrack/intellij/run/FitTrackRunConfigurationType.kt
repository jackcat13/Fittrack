package com.fittrack.intellij.run

import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.ConfigurationType
import com.intellij.execution.configurations.RunConfiguration
import com.intellij.openapi.project.Project
import javax.swing.Icon

class FitTrackRunConfigurationType : ConfigurationType {
    private val factory = FitTrackRunConfigurationFactory(this)

    override fun getDisplayName(): String = "Fit & Track"
    override fun getConfigurationTypeDescription(): String = "Compile a Fit & Track training log"
    override fun getIcon(): Icon? = null
    override fun getId(): String = "FITTRACK_RUN_CONFIGURATION"
    override fun getConfigurationFactories(): Array<ConfigurationFactory> = arrayOf(factory)
}

class FitTrackRunConfigurationFactory(type: ConfigurationType) : ConfigurationFactory(type) {
    override fun getId(): String = "FitTrack"

    override fun createTemplateConfiguration(project: Project): RunConfiguration =
        FitTrackRunConfiguration(project, this, "Compile Fit & Track")
}

