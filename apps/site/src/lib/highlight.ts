import {getHighlighter} from 'fumadocs-core/highlight'
import {naplLanguage} from '@napl/grammar'

const naplLang = {...naplLanguage, name: 'napl'}

export async function highlightNapl(code: string): Promise<string> {
  const highlighter = await getHighlighter('js', {
    langs: [naplLang, 'yaml', 'markdown'],
    themes: ['github-light', 'github-dark'],
  })

  return highlighter.codeToHtml(code, {
    lang: 'napl',
    themes: {
      light: 'github-light',
      dark: 'github-dark',
    },
    defaultColor: false,
  })
}
