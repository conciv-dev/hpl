const DIRECTIVE_RE =
  /^(?:@ts-(?:ignore|expect-error|nocheck|check)\b|eslint-(?:disable(?:-line|-next-line)?|enable|env)\b|oxlint-(?:disable(?:-line|-next-line)?|enable)\b|biome-ignore\b|@vitest-environment\b|prettier-ignore\b|oxfmt-ignore\b|@vite-ignore\b|webpack(?:ChunkName|Mode|Prefetch|Preload|Ignore|Include|Exclude)\b|(?:v8|c8|istanbul) ignore\b|@license\b|@preserve\b|[@#]__(?:PURE|NO_SIDE_EFFECTS)__|(?:[@#]\s*)?sourceMappingURL=|(?:global|globals|exported)\b)/

function isDirective(comment) {
  if (comment.type === 'Shebang') return true
  const value = comment.value.trim()
  if (comment.type === 'Block' && value.startsWith('!')) return true
  if (value.startsWith('/ <reference') || value.startsWith('/ <amd')) return true
  return DIRECTIVE_RE.test(value)
}

const EM_DASH_RE = /\u2014/g

const EMOJI_RE = /\p{Extended_Pictographic}/gu

const ALLOWED_EMOJI = new Set(['\u{1F9D1}', '\u{1F916}'])

function buildLineStarts(text) {
  const lineStarts = [0]
  for (let index = 0; index < text.length; index++) {
    if (text[index] === '\n') lineStarts.push(index + 1)
  }
  return lineStarts
}

function offsetToLocation(lineStarts, offset) {
  let low = 0
  let high = lineStarts.length - 1
  while (low < high) {
    const mid = (low + high + 1) >> 1
    if (lineStarts[mid] <= offset) low = mid
    else high = mid - 1
  }
  return {line: low + 1, column: offset - lineStarts[low]}
}

const noComments = {
  meta: {
    type: 'suggestion',
    fixable: 'code',
    messages: {noComment: 'Comments are not allowed. Delete it, or make the code self-explanatory.'},
    schema: [],
  },
  createOnce(context) {
    return {
      Program() {
        for (const comment of context.sourceCode.getAllComments()) {
          if (isDirective(comment)) continue
          context.report({
            node: comment,
            messageId: 'noComment',
            fix: (fixer) => fixer.replaceText(comment, ' '),
          })
        }
      },
    }
  },
}

const noEmDash = {
  meta: {
    type: 'problem',
    messages: {noEmDash: 'Em dashes are not allowed. Use a comma, colon, or period.'},
    schema: [],
  },
  createOnce(context) {
    return {
      Program() {
        const text = context.sourceCode.text
        const lineStarts = buildLineStarts(text)
        EM_DASH_RE.lastIndex = 0
        let match = EM_DASH_RE.exec(text)
        while (match !== null) {
          const start = offsetToLocation(lineStarts, match.index)
          const end = offsetToLocation(lineStarts, match.index + match[0].length)
          context.report({loc: {start, end}, messageId: 'noEmDash'})
          match = EM_DASH_RE.exec(text)
        }
      },
    }
  },
}

const noEmoji = {
  meta: {
    type: 'problem',
    messages: {
      noEmoji: 'Emoji are not allowed here. Only the file-extension aliases U+1F9D1 and U+1F916 are permitted.',
    },
    schema: [],
  },
  createOnce(context) {
    return {
      Program() {
        const text = context.sourceCode.text
        const lineStarts = buildLineStarts(text)
        EMOJI_RE.lastIndex = 0
        let match = EMOJI_RE.exec(text)
        while (match !== null) {
          if (!ALLOWED_EMOJI.has(match[0])) {
            const start = offsetToLocation(lineStarts, match.index)
            const end = offsetToLocation(lineStarts, match.index + match[0].length)
            context.report({loc: {start, end}, messageId: 'noEmoji'})
          }
          match = EMOJI_RE.exec(text)
        }
      },
    }
  },
}

export default {
  meta: {name: 'napl'},
  rules: {'no-comments': noComments, 'no-em-dash': noEmDash, 'no-emoji': noEmoji},
}
