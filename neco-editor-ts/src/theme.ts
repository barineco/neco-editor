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
