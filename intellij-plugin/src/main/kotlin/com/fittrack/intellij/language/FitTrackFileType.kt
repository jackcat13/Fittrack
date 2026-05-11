package com.fittrack.intellij.language

import com.intellij.openapi.fileTypes.LanguageFileType
import javax.swing.Icon

class FitTrackFileType : LanguageFileType(FitTrackLanguage) {
    override fun getName(): String = "FitTrack"
    override fun getDescription(): String = "Fit & Track training log"
    override fun getDefaultExtension(): String = "fit"
    override fun getIcon(): Icon? = null

    companion object {
        @JvmField
        val INSTANCE = FitTrackFileType()
    }
}

