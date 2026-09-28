import * as esbuild from 'esbuild'

// The SDK Index re-exports @ohos adapters; in the Node harness those
// @kit/@ohos imports are stubbed out (never instantiated in tests).
const kitStub = {
  name: 'kit-stub',
  setup(build) {
    build.onResolve({ filter: /^(@kit\.|@ohos\.)/ }, (args) => ({ path: args.path, namespace: 'kit-stub' }))
    build.onLoad({ filter: /.*/, namespace: 'kit-stub' }, () => ({
      contents: `module.exports = new Proxy({}, { get: () => () => ({}) })`,
      loader: 'js'
    }))
  }
}

// Bundle the ArkTS SDK core (plain-TS .ets files) for Node runtime.
await esbuild.build({
  entryPoints: ['../Library/restsend/src/main/ets/Index.ets'],
  bundle: true,
  format: 'esm',
  outfile: './sdk.mjs',
  resolveExtensions: ['.ets', '.ts', '.js'],
  loader: { '.ets': 'ts' },
  plugins: [kitStub],
  logLevel: 'error'
})
console.log('sdk.mjs built')
