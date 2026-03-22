// eslint.config.js — AUTO-GENERATED. DO NOT EDIT directly.
// Re-sync: node claude-resources/tools/sync-thesis-eslint.js (from development/)
//
// Rules: const-only (no var/let), no classes — pure functions + readonly types.
import tsParser from '@typescript-eslint/parser'

export default [
  {
    files: ['**/*.ts'],
    ignores: [
      '**/node_modules/**',
      '**/dist/**',
      '**/*.d.ts',
      'scripts/**',
      'mcp-servers/**',
    ],
    languageOptions: { parser: tsParser },
    rules: {
      'prefer-const': 'error',
      'no-var': 'error',
      'no-restricted-syntax': [
        'error',
        {
          selector: 'ClassDeclaration',
          message: 'Use pure functions and readonly types instead of classes.',
        },
        {
          selector: 'ClassExpression',
          message: 'Use pure functions and readonly types instead of classes.',
        },
      ],
    },
  },
]
