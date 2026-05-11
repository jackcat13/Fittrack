package com.fittrack.intellij.language

import com.intellij.lexer.LexerBase
import com.intellij.psi.TokenType
import com.intellij.psi.tree.IElementType

class FitTrackLexer : LexerBase() {
    private var buffer: CharSequence = ""
    private var startOffset: Int = 0
    private var endOffset: Int = 0
    private var state: Int = 0
    private var tokenStart: Int = 0
    private var tokenEnd: Int = 0
    private var tokenType: IElementType? = null

    override fun start(buffer: CharSequence, startOffset: Int, endOffset: Int, initialState: Int) {
        this.buffer = buffer
        this.startOffset = startOffset
        this.endOffset = endOffset
        this.state = initialState
        this.tokenStart = startOffset
        locateToken()
    }

    override fun getState(): Int = state
    override fun getTokenType(): IElementType? = tokenType
    override fun getTokenStart(): Int = tokenStart
    override fun getTokenEnd(): Int = tokenEnd
    override fun getBufferSequence(): CharSequence = buffer
    override fun getBufferEnd(): Int = endOffset

    override fun advance() {
        tokenStart = tokenEnd
        locateToken()
    }

    private fun locateToken() {
        if (tokenStart >= endOffset) {
            tokenType = null
            tokenEnd = endOffset
            return
        }

        val current = buffer[tokenStart]
        when {
            current.isWhitespace() -> readWhile(TokenType.WHITE_SPACE) { it.isWhitespace() }
            current == '#' -> readUntilLineEnd(FitTrackTokenTypes.COMMENT)
            current == '"' -> readString()
            current == '@' -> readRpe()
            current.isDigit() -> readNumber()
            current.isLetter() -> readWord()
            else -> {
                tokenType = TokenType.BAD_CHARACTER
                tokenEnd = tokenStart + 1
            }
        }
    }

    private fun readString() {
        var index = tokenStart + 1
        var escaped = false
        while (index < endOffset) {
            val ch = buffer[index]
            if (ch == '"' && !escaped) {
                index += 1
                break
            }
            escaped = ch == '\\' && !escaped
            if (ch != '\\') escaped = false
            index += 1
        }
        tokenType = FitTrackTokenTypes.STRING
        tokenEnd = index
    }

    private fun readRpe() {
        var index = tokenStart + 1
        while (index < endOffset && (buffer[index].isDigit() || buffer[index] == '.')) {
            index += 1
        }
        tokenType = FitTrackTokenTypes.RPE
        tokenEnd = index
    }

    private fun readNumber() {
        var index = tokenStart
        while (index < endOffset && (buffer[index].isDigit() || buffer[index] == '.')) {
            index += 1
        }
        tokenType = FitTrackTokenTypes.NUMBER
        tokenEnd = index
    }

    private fun readWord() {
        var index = tokenStart
        while (index < endOffset && buffer[index].isLetter()) {
            index += 1
        }
        val word = buffer.subSequence(tokenStart, index).toString()
        tokenType = when (word) {
            "training", "exercise", "set", "cardio", "note", "x" -> FitTrackTokenTypes.KEYWORD
            "kg", "km" -> FitTrackTokenTypes.UNIT
            else -> FitTrackTokenTypes.IDENTIFIER
        }
        tokenEnd = index
    }

    private fun readUntilLineEnd(type: IElementType) {
        var index = tokenStart
        while (index < endOffset && buffer[index] != '\n') {
            index += 1
        }
        tokenType = type
        tokenEnd = index
    }

    private fun readWhile(type: IElementType, predicate: (Char) -> Boolean) {
        var index = tokenStart
        while (index < endOffset && predicate(buffer[index])) {
            index += 1
        }
        tokenType = type
        tokenEnd = index
    }
}

