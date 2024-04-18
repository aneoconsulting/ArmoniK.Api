import antfu from '@antfu/eslint-config'

export default antfu({
  rules: {
    'yml/no-empty-mapping-value': 'off',
    'antfu/consistent-list-newline': 'off',
    'style/comma-dangle': 'off',
    'curly': 'off',
    'eslint-comments/no-unlimited-disable': 'off',
    'style/semi': 'off',
    'ts/ban-ts-comment': 'off',
  },
  ignores: [
    'generated',
  ]
})
