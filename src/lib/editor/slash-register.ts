import type * as monaco from 'monaco-editor'
import { CODE_LANGUAGES, SLASH_COMMANDS, slashQuery, type SlashCommand } from './slash-commands'

const RUN = 'gitpad.slash.insert'

type Editor = monaco.editor.IStandaloneCodeEditor
type Arg = { kind: 'command'; id: string } | { kind: 'language'; value: string }

/// The `/` menu for Markdown notes. One provider, two stages: commands, then
/// the language of a code fence. Returns the disposer for the editor lifecycle.
export function registerSlashCommands(m: typeof monaco, editor: Editor): () => void {
  // The code command deletes its own text and waits for the language pick.
  let awaiting: { path: string; line: number } | null = null
  // Typing over a selection replaces it, so keep its text for the wrap commands.
  // The start position proves the slash was typed on top of it.
  let cached: { path: string; text: string; line: number; column: number } | null = null
  let lastLanguage = 'text'
  let applying = false

  function positionAfter(start: monaco.Position, text: string, offset: number) {
    const lines = text.slice(0, offset).split('\n')
    const line = start.lineNumber + lines.length - 1
    const column = lines.length === 1 ? start.column + offset : lines[lines.length - 1].length + 1
    return new m.Position(line, column)
  }

  /// The `/word` range on the current line, if the menu was opened from it.
  function slashRange() {
    const model = editor.getModel()
    const position = editor.getPosition()
    if (!model || !position) return null
    const line = model.getLineContent(position.lineNumber).slice(0, position.column - 1)
    const query = slashQuery(line)
    if (!query) return null
    return new m.Range(position.lineNumber, query.start + 1, position.lineNumber, position.column)
  }

  function runCommand(cmd: SlashCommand) {
    const model = editor.getModel()
    const range = slashRange()
    if (!model || !range) return
    const selection = editor.getSelection()
    const start = range.getStartPosition()
    const selected =
      cached &&
      cached.path === model.uri.path &&
      cached.line === start.lineNumber &&
      cached.column === start.column &&
      selection?.isEmpty()
        ? cached.text
        : ''
    cached = null

    if (cmd.id === 'code' && selected === '') {
      // Ask for the language first: the fence is built only once one is picked.
      applying = true
      editor.executeEdits('slash', [{ range, text: '' }])
      applying = false
      const start = range.getStartPosition()
      editor.setPosition(start)
      awaiting = { path: model.uri.path, line: start.lineNumber }
      editor.trigger('slash', 'editor.action.triggerSuggest', {})
      return
    }

    const built = cmd.build(selected, lastLanguage)
    editor.executeEdits('slash', [{ range, text: built.text }])
    editor.setSelection(
      m.Selection.fromPositions(
        positionAfter(start, built.text, built.select[0]),
        positionAfter(start, built.text, built.select[1]),
      ),
    )
    editor.focus()
  }

  function insertFence(language: string) {
    const position = editor.getPosition()
    if (!position) return
    lastLanguage = language
    awaiting = null
    const range = new m.Range(position.lineNumber, position.column, position.lineNumber, position.column)
    applying = true
    editor.executeEdits('slash', [{ range, text: '```' + language + '\n\n```\n' }])
    applying = false
    // The fence is open on the line above: put the caret on the empty line.
    editor.setPosition({ lineNumber: position.lineNumber + 1, column: 1 })
    editor.focus()
  }

  const command = m.editor.registerCommand(RUN, (_accessor, arg: Arg) => {
    if (arg.kind === 'language') return insertFence(arg.value)
    const cmd = SLASH_COMMANDS.find((candidate) => candidate.id === arg.id)
    if (cmd) runCommand(cmd)
  })

  const provider = m.languages.registerCompletionItemProvider('markdown', {
    triggerCharacters: ['/'],
    provideCompletionItems(model, position) {
      const stage = awaiting
      if (stage && stage.path === model.uri.path && stage.line === position.lineNumber) {
        const range = new m.Range(position.lineNumber, position.column, position.lineNumber, position.column)
        return {
          suggestions: CODE_LANGUAGES.map((language) => ({
            label: language,
            kind: m.languages.CompletionItemKind.Keyword,
            insertText: '',
            filterText: language,
            sortText: language === lastLanguage ? '0' : '1',
            range,
            command: { id: RUN, title: `Code block: ${language}`, arguments: [{ kind: 'language', value: language }] },
          })),
        }
      }

      // shortcut: the menu also opens inside an open code fence; scan the lines
      // above here if that ever gets in the way.
      const line = model.getLineContent(position.lineNumber).slice(0, position.column - 1)
      const query = slashQuery(line)
      if (!query) return { suggestions: [] }
      const range = new m.Range(position.lineNumber, query.start + 1, position.lineNumber, position.column)
      return {
        suggestions: SLASH_COMMANDS.map((cmd, index) => ({
          label: '/' + cmd.id,
          detail: cmd.detail,
          kind: m.languages.CompletionItemKind.Snippet,
          // The command deletes this text and inserts the real block, so the
          // caret lands exactly where the snippet wants it.
          insertText: '/' + cmd.id,
          filterText: '/' + cmd.id,
          sortText: String(index).padStart(2, '0'),
          range,
          command: { id: RUN, title: cmd.detail, arguments: [{ kind: 'command', id: cmd.id }] },
        })),
      }
    },
  })

  const selectionSub = editor.onDidChangeCursorSelection((e) => {
    const model = editor.getModel()
    if (!model || e.selection.isEmpty()) return
    cached = {
      path: model.uri.path,
      text: model.getValueInRange(e.selection),
      line: e.selection.startLineNumber,
      column: e.selection.startColumn,
    }
  })

  const contentSub = editor.onDidChangeModelContent(() => {
    if (applying) return
    awaiting = null
    // The wrap flow types '/bo' after the selection is gone: keep the text
    // while the line is still a bare slash query, forget it otherwise.
    const model = editor.getModel()
    const position = editor.getPosition()
    const line = model && position ? model.getLineContent(position.lineNumber).slice(0, position.column - 1) : ''
    if (!slashQuery(line)) cached = null
  })

  return () => {
    provider.dispose()
    command.dispose()
    selectionSub.dispose()
    contentSub.dispose()
  }
}
