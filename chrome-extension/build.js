import * as esbuild from 'esbuild';
import { copyFileSync, mkdirSync, existsSync, readdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const isWatch = process.argv.includes('--watch');

// Ensure dist directory exists
if (!existsSync('dist')) {
  mkdirSync('dist', { recursive: true });
}

// Copy DuckDB WASM files
const duckdbPath = join(__dirname, 'node_modules', '@duckdb', 'duckdb-wasm', 'dist');
if (existsSync(duckdbPath)) {
  const wasmFiles = readdirSync(duckdbPath).filter(f => f.endsWith('.wasm') || f.endsWith('.js'));
  for (const file of wasmFiles) {
    try {
      copyFileSync(join(duckdbPath, file), join('dist', file));
    } catch (e) {
      console.warn(`Could not copy ${file}:`, e.message);
    }
  }
}

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
