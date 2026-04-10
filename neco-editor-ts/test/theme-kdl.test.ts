import test from 'node:test'
import assert from 'node:assert/strict'

import {
  collectThemeStyleEntries,
  parseThemeKdl,
} from '../src/theme.ts'

test('parseThemeKdl reads codigen-style editor theme vars', () => {
  const theme = parseThemeKdl(`
    theme "demo-theme" {
      bg {
        app "#101010"
      }
      syntax {
        keyword "#abcdef"
      }
      terminal {
        cursor "#ffeeaa"
        ansi {
          bright-black "#333333"
        }
      }
      editor {
        bg "#202020"
        fg "#f8f8f8"
        cursor "#ffcc00"
      }
      override "finder" {
        git-modified-fg "#88aaff"
      }
    }
  `)

  assert.equal(theme.id, 'demo-theme')
  assert.equal(theme.name, 'demo-theme')
  assert.equal(theme.vars['bg-app'], '#101010')
  assert.equal(theme.vars['syntax-keyword'], '#abcdef')
  assert.equal(theme.vars['term-cursor'], '#ffeeaa')
  assert.equal(theme.vars['term-bright-black'], '#333333')
  assert.equal(theme.vars['editor-bg'], '#202020')
  assert.equal(theme.vars['editor-fg'], '#f8f8f8')
  assert.equal(theme.vars['editor-cursor'], '#ffcc00')
  assert.equal(theme.vars['view-finder-git-modified-fg'], '#88aaff')
})

test('parseThemeKdl resolves grad values and explicit vars', () => {
  const theme = parseThemeKdl(`
    theme "demo-theme" {
      grad "accent-glow" {
        value "linear-gradient(180deg, #111111, #222222)"
      }
      editor {
        selection "accent-glow"
      }
      var name="editor-composition-bg" value="rgba(120, 160, 210, 0.18)"
    }
  `)

  assert.equal(theme.vars['editor-selection'], 'linear-gradient(180deg, #111111, #222222)')
  assert.equal(theme.vars['editor-composition-bg'], 'rgba(120, 160, 210, 0.18)')
})

test('collectThemeStyleEntries adds files alias for legacy finder vars', () => {
  assert.deepEqual(
    collectThemeStyleEntries({
      id: 'demo',
      name: 'Demo',
      vars: {
        'editor-bg': '#111111',
        'view-finder-item': '#222222',
      },
    }),
    [
      ['editor-bg', '#111111'],
      ['view-finder-item', '#222222'],
      ['view-files-item', '#222222'],
    ],
  )
})
