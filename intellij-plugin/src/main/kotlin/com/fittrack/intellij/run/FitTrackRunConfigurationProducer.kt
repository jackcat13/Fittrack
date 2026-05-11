package com.fittrack.intellij.run

import com.intellij.execution.actions.ConfigurationContext
import com.intellij.execution.actions.RunConfigurationProducer
import com.intellij.execution.configurations.ConfigurationTypeUtil
import com.intellij.openapi.util.Ref
import com.intellij.psi.PsiElement

class FitTrackRunConfigurationProducer : RunConfigurationProducer<FitTrackRunConfiguration>(
    ConfigurationTypeUtil.findConfigurationType(FitTrackRunConfigurationType::class.java),
) {
    override fun setupConfigurationFromContext(
        configuration: FitTrackRunConfiguration,
        context: ConfigurationContext,
        sourceElement: Ref<PsiElement>,
    ): Boolean {
        val file = context.psiLocation?.containingFile?.virtualFile ?: return false
        if (file.extension != "fit") return false

        configuration.inputPath = file.path
        configuration.name = "Compile ${file.name}"
        return true
    }

    override fun isConfigurationFromContext(
        configuration: FitTrackRunConfiguration,
        context: ConfigurationContext,
    ): Boolean {
        val file = context.psiLocation?.containingFile?.virtualFile ?: return false
        return file.extension == "fit" && configuration.inputPath == file.path
    }
}

