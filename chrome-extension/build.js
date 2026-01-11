import * as esbuild from 'esbuild';
import { mkdirSync, existsSync } from 'fs';

const isWatch = process.argv.includes('--watch');

// Ensure dist directory exists
if (!existsSync('dist')) {
  mkdirSync('dist', { recursive: true });
}

// Note: DuckDB WASM files are loaded from jsDelivr CDN at runtime
// No need to copy them locally

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

async function build() {
  try {
    if (isWatch) {
      const ctx1 = await esbuild.context(backgroundBuildOptions);
      const ctx2 = await esbuild.context(pagesBuildOptions);
      await Promise.all([ctx1.watch(), ctx2.watch()]);
      console.log('Watching for changes...');
    } else {
      await Promise.all([
        esbuild.build(backgroundBuildOptions),
        esbuild.build(pagesBuildOptions)
      ]);
      console.log('Build completed successfully!');
    }
  } catch (error) {
    console.error('Build failed:', error);
    process.exit(1);
  }
}

build();
