/** Valid TokenKind values from neco-syntax-textmate. */
const KNOWN_KINDS = new Set([
  'keyword',
  'type',
  'function',
  'string',
  'number',
  'comment',
  'operator',
  'punctuation',
  'variable',
  'constant',
  'tag',
  'attribute',
  'escape',
  'plain',
])

/**
 * Map a TokenKind string to its CSS class name.
 *
 * Unknown kinds fall back to `'neco-token-plain'`.
 */
export function tokenKindToClass(kind: string): string {
  if (KNOWN_KINDS.has(kind)) {
    return `neco-token-${kind}`
  }
  return 'neco-token-plain'
}

export interface ThemeData {
  name: string
  id: string
  vars: Record<string, string>
}

export interface ThemeStyleSource {
  vars: Record<string, string>
}

const appliedThemeVarNames = new WeakMap<HTMLElement, Set<string>>()

type KdlToken =
  | { type: 'word'; value: string }
  | { type: 'string'; value: string }
  | { type: 'equals' }
  | { type: 'lbrace' }
  | { type: 'rbrace' }
  | { type: 'newline' }

interface KdlNode {
  name: string
  args: string[]
  props: Record<string, string>
  children: KdlNode[]
}

export function collectThemeStyleEntries(theme: ThemeStyleSource): Array<[string, string]> {
  const entries: Array<[string, string]> = []
  for (const [name, value] of Object.entries(theme.vars)) {
    entries.push([name, value])
    if (name.startsWith('view-finder-')) {
      entries.push([`view-files-${name.slice('view-finder-'.length)}`, value])
    }
  }
  return entries
}

export function applyTheme(theme: ThemeData | null, target?: HTMLElement | null): void {
  if (typeof document === 'undefined' || !theme) return
  const element = target ?? document.documentElement
  const applied = appliedThemeVarNames.get(element) ?? new Set<string>()
  for (const name of applied) {
    element.style.removeProperty(`--${name}`)
  }
  applied.clear()
  for (const [name, value] of collectThemeStyleEntries(theme)) {
    element.style.setProperty(`--${name}`, value)
    applied.add(name)
  }
  appliedThemeVarNames.set(element, applied)
}

export function parseThemeKdl(text: string, fallbackId = ''): ThemeData {
  const document = parseKdlDocument(text)
  const theme = document.find((node) => node.name === 'theme')
  if (theme === undefined) {
    throw new Error('theme node is required')
  }

  const themeId = theme.args[0] ?? fallbackId
  if (themeId.length === 0) {
    throw new Error('theme id is required')
  }

  const gradDefs = new Map<string, string>()
  for (const node of theme.children) {
    if (node.name !== 'grad') continue
    const name = node.args[0]
    if (name === undefined) {
      throw new Error('grad name is required')
    }
    const value = parseGradNode(node)
    if (value === null) {
      throw new Error(`grad ${name} requires value or structured steps`)
    }
    gradDefs.set(name, value)
  }

  const vars: Record<string, string> = {}
  for (const node of theme.children) {
    switch (node.name) {
      case 'bg':
      case 'fg':
      case 'border':
      case 'shadow':
      case 'syntax':
      case 'state':
        collectPrefixedVars(vars, node.name, node, gradDefs)
        break
      case 'terminal':
        collectTerminalVars(vars, node, gradDefs)
        break
      case 'editor':
        collectPrefixedVars(vars, 'editor', node, gradDefs)
        break
      case 'override': {
        const scope = node.args[0]
        if (scope === undefined) {
          throw new Error('override name is required')
        }
        collectPrefixedVars(vars, `view-${scope}`, node, gradDefs)
        break
      }
      case 'var': {
        const name = node.props.name
        const value = node.props.value
        if (name !== undefined && value !== undefined) {
          vars[name] = value
        }
        break
      }
    }
  }

  return {
    name: themeId,
    id: themeId,
    vars,
  }
}

function collectPrefixedVars(
  vars: Record<string, string>,
  prefix: string,
  node: KdlNode,
  gradDefs: ReadonlyMap<string, string>,
): void {
  for (const child of node.children) {
    const rawValue = child.args[0]
    if (rawValue === undefined) continue
    vars[`${prefix}-${child.name}`] = gradDefs.get(rawValue) ?? rawValue
  }
}

function collectTerminalVars(
  vars: Record<string, string>,
  node: KdlNode,
  gradDefs: ReadonlyMap<string, string>,
): void {
  for (const child of node.children) {
    if (child.name === 'ansi') {
      collectPrefixedVars(vars, 'term', child, gradDefs)
      continue
    }
    const rawValue = child.args[0]
    if (rawValue === undefined) continue
    vars[`term-${child.name}`] = gradDefs.get(rawValue) ?? rawValue
  }
}

function parseGradNode(node: KdlNode): string | null {
  const value = node.children.find((child) => child.name === 'value')?.args[0] ?? node.props.value
  if (value !== undefined) return value

  const steps: string[] = []
  for (const child of node.children) {
    if (child.name === 'linear') {
      const direction = child.props.direction ?? '180deg'
      const from = child.props.from
      const to = child.props.to
      if (from !== undefined && to !== undefined) {
        steps.push(`linear-gradient(${direction}, ${from}, ${to})`)
      }
    }
    if (child.name === 'radial') {
      const shape = child.props.shape ?? 'circle'
      const at = child.props.at ?? 'center'
      const color = child.props.color
      const stop = child.props.stop ?? '100%'
      if (color !== undefined) {
        steps.push(`radial-gradient(${shape} at ${at}, ${color}, transparent ${stop})`)
      }
    }
  }
  return steps.length > 0 ? steps.join(', ') : null
}

function parseKdlDocument(text: string): KdlNode[] {
  const parser = new KdlParser(tokenizeKdl(text))
  return parser.parseNodes(false)
}

class KdlParser {
  private pos = 0
  private readonly tokens: KdlToken[]

  constructor(tokens: KdlToken[]) {
    this.tokens = tokens
  }

  parseNodes(untilRbrace: boolean): KdlNode[] {
    const nodes: KdlNode[] = []
    while (this.pos < this.tokens.length) {
      this.skipNewlines()
      if (this.peek()?.type === 'rbrace') {
        if (!untilRbrace) throw new Error('unexpected }')
        this.pos++
        break
      }
      const token = this.peek()
      if (token === undefined) break
      if (token.type !== 'word') {
        throw new Error(`expected node name, got ${token.type}`)
      }
      nodes.push(this.parseNode())
    }
    return nodes
  }

  private parseNode(): KdlNode {
    const name = this.expect('word').value
    const args: string[] = []
    const props: Record<string, string> = {}
    let children: KdlNode[] = []

    while (this.pos < this.tokens.length) {
      const token = this.peek()
      if (token === undefined || token.type === 'newline' || token.type === 'rbrace') break
      if (token.type === 'lbrace') {
        this.pos++
        children = this.parseNodes(true)
        break
      }
      if (token.type === 'word' && this.peek(1)?.type === 'equals') {
        const key = token.value
        this.pos += 2
        const valueToken = this.peek()
        if (valueToken?.type !== 'string' && valueToken?.type !== 'word') {
          throw new Error(`expected property value for ${key}`)
        }
        props[key] = valueToken.value
        this.pos++
        continue
      }
      if (token.type === 'string' || token.type === 'word') {
        args.push(token.value)
        this.pos++
        continue
      }
      throw new Error(`unexpected token ${token.type}`)
    }
    this.skipNewlines()
    return { name, args, props, children }
  }

  private skipNewlines(): void {
    while (this.peek()?.type === 'newline') this.pos++
  }

  private peek(offset = 0): KdlToken | undefined {
    return this.tokens[this.pos + offset]
  }

  private expect<T extends KdlToken['type']>(type: T): Extract<KdlToken, { type: T }> {
    const token = this.peek()
    if (token?.type !== type) {
      throw new Error(`expected ${type}`)
    }
    this.pos++
    return token as Extract<KdlToken, { type: T }>
  }
}

function tokenizeKdl(text: string): KdlToken[] {
  const tokens: KdlToken[] = []
  let i = 0
  while (i < text.length) {
    const ch = text[i]
    if (ch === ' ' || ch === '\t' || ch === '\r') {
      i++
      continue
    }
    if (ch === '\n' || ch === ';') {
      tokens.push({ type: 'newline' })
      i++
      continue
    }
    if (ch === '/' && text[i + 1] === '/') {
      i += 2
      while (i < text.length && text[i] !== '\n') i++
      continue
    }
    if (ch === '{') {
      tokens.push({ type: 'lbrace' })
      i++
      continue
    }
    if (ch === '}') {
      tokens.push({ type: 'rbrace' })
      i++
      continue
    }
    if (ch === '=') {
      tokens.push({ type: 'equals' })
      i++
      continue
    }
    if (ch === '"') {
      const [value, nextIndex] = readKdlString(text, i)
      tokens.push({ type: 'string', value })
      i = nextIndex
      continue
    }

    const start = i
    while (
      i < text.length &&
      !/\s/.test(text[i] ?? '') &&
      !['{', '}', '=', ';'].includes(text[i] ?? '')
    ) {
      i++
    }
    tokens.push({ type: 'word', value: text.slice(start, i) })
  }
  return tokens
}

function readKdlString(text: string, start: number): [string, number] {
  let value = ''
  let i = start + 1
  while (i < text.length) {
    const ch = text[i]
    if (ch === '"') return [value, i + 1]
    if (ch === '\\') {
      const next = text[i + 1]
      switch (next) {
        case 'n':
          value += '\n'
          break
        case 'r':
          value += '\r'
          break
        case 't':
          value += '\t'
          break
        case '"':
        case '\\':
          value += next
          break
        default:
          value += next ?? ''
          break
      }
      i += 2
      continue
    }
    value += ch
    i++
  }
  throw new Error('unterminated string')
}
