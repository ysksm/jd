import * as esbuild from 'esbuild';
import { mkdirSync, existsSync } from 'fs';

const isWatch = process.argv.includes('--watch');

// Ensure dist directory exists
if (!existsSync('dist')) {
  mkdirSync('dist', { recursive: true });
}

// Note: DuckDB WASM files are loaded from jsDelivr CDN at runtime
// No need to copy them locally

const buildOptions = {
  entryPoints: [
    { in: 'src/background/index.ts', out: 'background' },
    { in: 'src/popup/popup.ts', out: 'popup' },
    { in: 'src/options/options.ts', out: 'options' },
  ],
  bundle: true,
  outdir: 'dist',
  format: 'esm',
  platform: 'browser',
  target: 'chrome100',
  sourcemap: true,
  external: [],
  define: {
    'process.env.NODE_ENV': '"production"'
  },
  loader: {
    '.wasm': 'file'
  }
};

async function build() {
  try {
    if (isWatch) {
      const ctx = await esbuild.context(buildOptions);
      await ctx.watch();
      console.log('Watching for changes...');
    } else {
      await esbuild.build(buildOptions);
      console.log('Build completed successfully!');
    }
  } catch (error) {
    console.error('Build failed:', error);
    process.exit(1);
  }
}

build();
