import KanaKanjiConverterModule
import Foundation
import ffi

@MainActor let converter = KanaKanjiConverter()
@MainActor var composingText = ComposingText()

@MainActor var execURL = URL(filePath: "")
@MainActor var useZenzai = false
@MainActor var leftSideContext = ""

@MainActor var options = ConvertRequestOptions(
    requireJapanesePrediction: true,
    requireEnglishPrediction: false,
    keyboardLanguage: .ja_JP,
    learningType: .nothing,
    dictionaryResourceURL: execURL.appendingPathComponent("Dictionary"),
    memoryDirectoryURL: URL(filePath: "./test"),
    sharedContainerURL: URL(filePath: "./test"),
    textReplacer: .init {
        return execURL.appendingPathComponent("EmojiDictionary").appendingPathComponent("emoji_all_E15.1.txt")
    },
    // zenzai
    zenzaiMode: useZenzai ? .on(
        weight: execURL.appendingPathComponent("zenz.gguf"),
        inferenceLimit: 1,
        requestRichCandidates: true,
        personalizationMode: nil,
        versionDependentMode: .v3(
            .init(
                profile: "",
                leftSideContext: leftSideContext
            )
        )
    ) : .off,
    metadata: .init(versionString: "Azookey for Windows")
)

class SimpleComposingText {
    init(text: String, cursor: Int) {
        self.text = UnsafeMutablePointer<CChar>(mutating: text.utf8String)!
        self.cursor = cursor
    }

    var text: UnsafeMutablePointer<CChar>
    var cursor: Int
}

struct SComposingText {
    var text: UnsafeMutablePointer<CChar>
    var cursor: Int
}

func constructCandidateString(candidate: Candidate, hiragana: String) -> String {
    var remainingHiragana = hiragana
    var result = ""
    
    for data in candidate.data {
        if remainingHiragana.count < data.ruby.count {
            result += remainingHiragana
            break
        }
        remainingHiragana.removeFirst(data.ruby.count)
        result += data.word
    }
    
    return result
}

@_silgen_name("Initialize")
@MainActor public func initialize(
    path: UnsafePointer<CChar>,
    use_zenzai: Bool
) {
    let path = String(cString: path)
    execURL = URL(filePath: path)
    useZenzai = use_zenzai

    composingText.insertAtCursorPosition("a", inputStyle: .roman2kana)
    converter.requestCandidates(composingText, options: options)
    composingText = ComposingText()
}

@_silgen_name("SetLeftSideContext")
@MainActor public func set_left_side_context(
    context: UnsafePointer<CChar>
) {
    leftSideContext = String(cString: context)
}

@_silgen_name("AppendText")
@MainActor public func append_text(
    input: UnsafePointer<CChar>,
    cursorPtr: UnsafeMutablePointer<Int>
) -> UnsafeMutablePointer<CChar> {
    let inputString = String(cString: input)
    composingText.insertAtCursorPosition(inputString, inputStyle: .roman2kana)

    cursorPtr.pointee = composingText.convertTargetCursorPosition    
    return _strdup(composingText.convertTarget)!
}

@_silgen_name("RemoveText")
@MainActor public func remove_text(
    cursorPtr: UnsafeMutablePointer<Int>
) -> UnsafeMutablePointer<CChar> {
    composingText.deleteBackwardFromCursorPosition(count: 1)

    cursorPtr.pointee = composingText.convertTargetCursorPosition
    return _strdup(composingText.convertTarget)!
}

@_silgen_name("MoveCursor")
@MainActor public func move_cursor(
    offset: Int32,
    cursorPtr: UnsafeMutablePointer<Int>
) -> UnsafeMutablePointer<CChar> {
    let previousCursor = composingText.convertTargetCursorPosition
    let cursor = composingText.moveCursorFromCursorPosition(count: Int(offset))
    print("offset: \(offset), cursor: \(cursor)")

    cursorPtr.pointee = cursor
    return _strdup(composingText.convertTarget)!
}

@_silgen_name("ClearText")
@MainActor public func clear_text() {
    composingText = ComposingText()
}

func to_list_pointer(_ list: [FFICandidate]) -> UnsafeMutablePointer<UnsafeMutablePointer<FFICandidate>?> {
    let pointer = UnsafeMutablePointer<UnsafeMutablePointer<FFICandidate>?>.allocate(capacity: list.count)
    for (i, item) in list.enumerated() {
        pointer[i] = UnsafeMutablePointer<FFICandidate>.allocate(capacity: 1)
        pointer[i]?.pointee = item
    }
    return pointer
}

@_silgen_name("GetComposedText")
@MainActor public func get_composed_text(lengthPtr: UnsafeMutablePointer<Int>) -> UnsafeMutablePointer<UnsafeMutablePointer<FFICandidate>?> {
    let hiragana = composingText.convertTarget
    let converted = converter.requestCandidates(composingText, options: options)
    var result: [FFICandidate] = []

    for i in 0..<converted.mainResults.count {
        let candidate = converted.mainResults[i]

        let text = strdup(constructCandidateString(candidate: candidate, hiragana: hiragana))
        let hiragana = strdup(hiragana)
        let correspondingCount = candidate.correspondingCount

        var afterComposingText = composingText
        afterComposingText.prefixComplete(correspondingCount: correspondingCount)
        let subtext = strdup(afterComposingText.convertTarget)

        result.append(FFICandidate(text: text, subtext: subtext, hiragana: hiragana, correspondingCount: Int32(correspondingCount)))        
    }

    lengthPtr.pointee = result.count

    return to_list_pointer(result)
}

@_silgen_name("ShrinkText")
@MainActor public func shrink_text(
    offset: Int32
) -> UnsafeMutablePointer<CChar>  {
    var afterComposingText = composingText
    afterComposingText.prefixComplete(correspondingCount: Int(offset))
    composingText = afterComposingText

    return _strdup(composingText.convertTarget)!
}