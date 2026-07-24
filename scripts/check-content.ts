import {readdirSync, readFileSync, type Dirent} from 'node:fs'
import {join, relative} from 'node:path'

const CONTENT_ROOT = join(process.cwd(), 'apps/site/content')

const MARKDOWN_RE = /\.mdx?$/

const ALLOWED_EMOJI = new Set(['\u{1F9D1}', '\u{1F916}'])

type Matcher = {pattern: RegExp; message: string; allow?: Set<string>}

const MATCHERS: Matcher[] = [
  {pattern: /\u2014/g, message: 'Em dash is not allowed. Use a comma, colon, or period.'},
  {
    pattern: /\p{Extended_Pictographic}/gu,
    message: 'Emoji is not allowed. Only NAPL file-extension aliases are permitted.',
    allow: ALLOWED_EMOJI,
  },
]

type Violation = {file: string; line: number; column: number; message: string}

function isMarkdownFile(entry: Dirent): boolean {
  return entry.isFile() && MARKDOWN_RE.test(entry.name)
}

function entryPath(entry: Dirent): string {
  return join(entry.parentPath, entry.name)
}

function collectMarkdownFiles(root: string): string[] {
  const entries = readdirSync(root, {withFileTypes: true, recursive: true})
  return entries.filter(isMarkdownFile).map(entryPath)
}

function isAllowed(character: string, allow: Set<string> | undefined): boolean {
  return allow !== undefined && allow.has(character)
}

function scanLineWithMatcher(file: string, lineNumber: number, line: string, matcher: Matcher): Violation[] {
  const violations: Violation[] = []
  matcher.pattern.lastIndex = 0
  let match = matcher.pattern.exec(line)
  while (match !== null) {
    if (!isAllowed(match[0], matcher.allow)) {
      violations.push({file, line: lineNumber, column: match.index + 1, message: matcher.message})
    }
    match = matcher.pattern.exec(line)
  }
  return violations
}

function scanFile(path: string): Violation[] {
  const file = relative(process.cwd(), path)
  const lines = readFileSync(path, 'utf8').split('\n')
  const violations: Violation[] = []
  for (let index = 0; index < lines.length; index++) {
    for (const matcher of MATCHERS) {
      violations.push(...scanLineWithMatcher(file, index + 1, lines[index] ?? '', matcher))
    }
  }
  return violations
}

function main(): void {
  const violations = collectMarkdownFiles(CONTENT_ROOT).flatMap(scanFile)
  for (const violation of violations) {
    process.stdout.write(`${violation.file}:${violation.line}:${violation.column} ${violation.message}\n`)
  }
  if (violations.length > 0) {
    process.stdout.write(`\n${violations.length} content violation(s) found.\n`)
    process.exit(1)
  }
}

main()
