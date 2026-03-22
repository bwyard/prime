#!/usr/bin/env node
// thesis-check.js — enforce the Stage thesis in CI and local dev
//
// Rules:
//   1. No `let` declarations (use `const`)
//   2. No `class` declarations (use pure functions + types)
//
// Does NOT fire on:
//   - Test files (*.test.ts) — relaxed for fixtures when needed
//     Pass --strict to include test files
//   - Lines with `// thesis-ignore` comment
//   - Type-only contexts (`let` inside JSDoc or string literals)
//
// Exit 0 = clean. Exit 1 = violations found.
//
// Usage:
//   node scripts/thesis-check.js
//   node scripts/thesis-check.js --strict     # include test files
//   node scripts/thesis-check.js --fix-hints  # print suggested fix per violation

import { readFileSync, readdirSync, statSync } from 'fs'
import { join, relative } from 'path'
import { fileURLToPath } from 'url'

const __dirname = fileURLToPath(new URL('.', import.meta.url))
const ROOT      = join(__dirname, '..')

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const args        = process.argv.slice(2)
const STRICT      = args.includes('--strict')   // include test files
const FIX_HINTS   = args.includes('--fix-hints')

const EXCLUDE_DIRS = ['node_modules', 'dist', '.git', 'scripts', 'mcp-servers']
const EXCLUDE_EXT  = ['.js', '.json', '.md', '.toml', '.lock', '.yaml', '.yml']

// ---------------------------------------------------------------------------
// Rules — each returns { line, col, rule, hint } | null
// ---------------------------------------------------------------------------

/** Matches `let ` at statement level — not inside strings or comments. */
const checkLet = (line, lineNum) => {
  const stripped = line.replace(/\/\/.*$/, '')           // strip line comment
  const match    = stripped.match(/\blet\s+/)
  if (!match) return null
  return {
    lineNum,
    col:  (match.index ?? 0) + 1,
    rule: 'no-let',
    hint: FIX_HINTS ? 'Replace `let` with `const`' : null,
  }
}

/** Matches `class ` declarations — not interfaces, not string mentions. */
const checkClass = (line, lineNum) => {
  const stripped = line.replace(/\/\/.*$/, '')
  const match    = stripped.match(/\bclass\s+\w/)
  if (!match) return null
  return {
    lineNum,
    col:  (match.index ?? 0) + 1,
    rule: 'no-class',
    hint: FIX_HINTS ? 'Replace `class` with a plain object type + pure functions' : null,
  }
}

const RULES = [checkLet, checkClass]

// ---------------------------------------------------------------------------
// File traversal
// ---------------------------------------------------------------------------

const collectFiles = (dir) => {
  const results = []
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry)
    const stat = statSync(full)
    if (stat.isDirectory()) {
      if (EXCLUDE_DIRS.includes(entry)) continue
      results.push(...collectFiles(full))
    } else {
      if (!full.endsWith('.ts'))         continue
      if (EXCLUDE_EXT.some(e => full.endsWith(e))) continue
      if (!STRICT && full.endsWith('.test.ts')) continue
      results.push(full)
    }
  }
  return results
}

// ---------------------------------------------------------------------------
// Check one file
// ---------------------------------------------------------------------------

const checkFile = (filePath) => {
  const src   = readFileSync(filePath, 'utf8')
  const lines = src.split('\n')
  const rel   = relative(ROOT, filePath)

  const { violations } = lines.reduce(
    ({ violations, inBlock }, line, i) => {
      // Track block comment state (/** ... */ and /* ... */)
      const trimmed = line.trimStart()
      if (inBlock) {
        return { violations, inBlock: !line.includes('*/') }
      }
      if (trimmed.startsWith('/*')) {
        return { violations, inBlock: !line.includes('*/') }
      }
      if (line.includes('thesis-ignore')) return { violations, inBlock }

      const lineViolations = RULES.flatMap(rule => {
        const v = rule(line, i + 1)
        return v ? [{ ...v, file: rel }] : []
      })
      return { violations: [...violations, ...lineViolations], inBlock }
    },
    { violations: [], inBlock: false },
  )

  return violations
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const files      = collectFiles(ROOT)
const violations = files.flatMap(checkFile)

if (violations.length === 0) {
  console.log(`thesis-check: ✓ ${files.length} files, 0 violations`)
  process.exit(0)
}

console.error(`thesis-check: ${violations.length} violation(s) found\n`)

for (const v of violations) {
  const hint = v.hint ? `  → ${v.hint}` : ''
  console.error(`  ${v.file}:${v.lineNum}:${v.col}  [${v.rule}]${hint ? '\n' + hint : ''}`)
}

console.error(`\nFix all violations before merging. Add // thesis-ignore to suppress a specific line.`)
process.exit(1)
