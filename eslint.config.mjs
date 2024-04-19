import { antfu } from '@antfu/eslint-config'

export default antfu({
  ignores: [
    '**/generated',
    'packages/python',
    '.docs',
    'packages/csharp',
    'packages/cpp',
  ],
}).overrideRules({
  'antfu/consistent-list-newline': 'off',
  'ts/ban-ts-comment': 'off',
  'curly': 'off',
})
