import * as esbuild from 'esbuild';
import { mkdirSync, existsSync, copyFileSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const isWatch = process.argv.includes('--watch');

// Ensure dist directory exists
if (!existsSync('dist')) {
  mkdirSync('dist', { recursive: true });
}

// Copy DuckDB WASM files locally (required for Manifest V3 CSP compliance)
const duckdbDistPath = join(__dirname, 'node_modules/@duckdb/duckdb-wasm/dist');
const filesToCopy = [
  // WASM files
  'duckdb-eh.wasm',
  'duckdb-mvp.wasm',
  // Worker scripts
  'duckdb-browser-eh.worker.js',
  'duckdb-browser-mvp.worker.js',
];

console.log('Copying DuckDB WASM files...');
for (const file of filesToCopy) {
  const src = join(duckdbDistPath, file);
  const dest = join(__dirname, 'dist', file);
  if (existsSync(src)) {
    copyFileSync(src, dest);
    console.log(`  Copied ${file}`);
  } else {
    console.warn(`  Warning: ${file} not found`);
  }
}

// Background service worker - use iife format
const backgroundBuildOptions = {
  entryPoints: [{ in: 'src/background/index.ts', out: 'background' }],
  bundle: true,
  outdir: 'dist',
  format: 'iife',
  platform: 'browser',
  target: 'chrome100',
  sourcemap: false,
  define: {
    'process.env.NODE_ENV': '"production"'
  }
};

// Popup and options - use esm format
const pagesBuildOptions = {
  entryPoints: [
    { in: 'src/popup/popup.ts', out: 'popup' },
    { in: 'src/options/options.ts', out: 'options' },
  ],
  bundle: true,
  outdir: 'dist',
  format: 'esm',
  platform: 'browser',
  target: 'chrome100',
  sourcemap: false,
  define: {
    'process.env.NODE_ENV': '"production"'
  }
};

// Offscreen document - use iife format (ESM may have issues in offscreen)
const offscreenBuildOptions = {
  entryPoints: [
    { in: 'src/offscreen/index.ts', out: 'offscreen' },
  ],
  bundle: true,
  outdir: 'dist',
  format: 'iife',
  platform: 'browser',
  target: 'chrome100',
  sourcemap: false,
  define: {
    'process.env.NODE_ENV': '"production"'
  }
};

async function build() {
  try {
    if (isWatch) {
      const ctx1 = await esbuild.context(backgroundBuildOptions);
      const ctx2 = await esbuild.context(pagesBuildOptions);
      const ctx3 = await esbuild.context(offscreenBuildOptions);
      await Promise.all([ctx1.watch(), ctx2.watch(), ctx3.watch()]);
      console.log('Watching for changes...');
    } else {
      await Promise.all([
        esbuild.build(backgroundBuildOptions),
        esbuild.build(pagesBuildOptions),
        esbuild.build(offscreenBuildOptions)
      ]);
      console.log('Build completed successfully!');
    }
  } catch (error) {
    console.error('Build failed:', error);
    process.exit(1);
  }
}

build();
