import { defineConfig } from 'tsup'

export default defineConfig({
  entry: ["src/index.ts", "src/generated/applications_service.ts"],
  format: ['esm', 'cjs'],
  target: 'node16',
  splitting: true,
  dts: true,
  clean: true,
  shims: false
})
