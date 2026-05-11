package com.fittrack.intellij.language

import com.intellij.psi.tree.IElementType

class FitTrackTokenType(debugName: String) : IElementType(debugName, FitTrackLanguage)

object FitTrackTokenTypes {
    val KEYWORD = FitTrackTokenType("FITTRACK_KEYWORD")
    val STRING = FitTrackTokenType("FITTRACK_STRING")
    val COMMENT = FitTrackTokenType("FITTRACK_COMMENT")
    val NUMBER = FitTrackTokenType("FITTRACK_NUMBER")
    val UNIT = FitTrackTokenType("FITTRACK_UNIT")
    val RPE = FitTrackTokenType("FITTRACK_RPE")
    val IDENTIFIER = FitTrackTokenType("FITTRACK_IDENTIFIER")
}

