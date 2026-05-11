package com.fittrack.intellij.language

import com.intellij.lexer.Lexer
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.HighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.editor.colors.TextAttributesKey.createTextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighterBase
import com.intellij.psi.TokenType
import com.intellij.psi.tree.IElementType

class FitTrackSyntaxHighlighter : SyntaxHighlighterBase() {
    override fun getHighlightingLexer(): Lexer = FitTrackLexer()

    override fun getTokenHighlights(tokenType: IElementType): Array<TextAttributesKey> = when (tokenType) {
        FitTrackTokenTypes.KEYWORD -> KEYWORD_KEYS
        FitTrackTokenTypes.STRING -> STRING_KEYS
        FitTrackTokenTypes.COMMENT -> COMMENT_KEYS
        FitTrackTokenTypes.NUMBER -> NUMBER_KEYS
        FitTrackTokenTypes.UNIT -> UNIT_KEYS
        FitTrackTokenTypes.RPE -> RPE_KEYS
        TokenType.BAD_CHARACTER -> BAD_CHARACTER_KEYS
        else -> EMPTY_KEYS
    }

    companion object {
        val KEYWORD = createTextAttributesKey("FITTRACK_KEYWORD", DefaultLanguageHighlighterColors.KEYWORD)
        val STRING = createTextAttributesKey("FITTRACK_STRING", DefaultLanguageHighlighterColors.STRING)
        val COMMENT = createTextAttributesKey("FITTRACK_COMMENT", DefaultLanguageHighlighterColors.LINE_COMMENT)
        val NUMBER = createTextAttributesKey("FITTRACK_NUMBER", DefaultLanguageHighlighterColors.NUMBER)
        val UNIT = createTextAttributesKey("FITTRACK_UNIT", DefaultLanguageHighlighterColors.METADATA)
        val RPE = createTextAttributesKey("FITTRACK_RPE", DefaultLanguageHighlighterColors.CONSTANT)
        val BAD_CHARACTER = createTextAttributesKey("FITTRACK_BAD_CHARACTER", HighlighterColors.BAD_CHARACTER)

        private val KEYWORD_KEYS = arrayOf(KEYWORD)
        private val STRING_KEYS = arrayOf(STRING)
        private val COMMENT_KEYS = arrayOf(COMMENT)
        private val NUMBER_KEYS = arrayOf(NUMBER)
        private val UNIT_KEYS = arrayOf(UNIT)
        private val RPE_KEYS = arrayOf(RPE)
        private val BAD_CHARACTER_KEYS = arrayOf(BAD_CHARACTER)
        private val EMPTY_KEYS = emptyArray<TextAttributesKey>()
    }
}

