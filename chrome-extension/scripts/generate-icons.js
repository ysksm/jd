// Script to generate extension icons
// Run with: node scripts/generate-icons.js
// Requires: npm install canvas

import { createCanvas } from 'canvas';
import { writeFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = join(__dirname, '..', 'public', 'icons');

if (!existsSync(iconsDir)) {
  mkdirSync(iconsDir, { recursive: true });
}

function generateIcon(size) {
  const canvas = createCanvas(size, size);
  const ctx = canvas.getContext('2d');

  // Background
  ctx.fillStyle = '#0052CC';
  ctx.fillRect(0, 0, size, size);

  // Rounded corners effect
  ctx.fillStyle = '#0052CC';
  ctx.beginPath();
  const radius = size * 0.15;
  ctx.roundRect(0, 0, size, size, radius);
  ctx.fill();

  // Text
  ctx.fillStyle = '#FFFFFF';
  ctx.font = `bold ${size * 0.6}px Arial`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText('J', size / 2, size / 2 + size * 0.05);

  return canvas.toBuffer('image/png');
}

const sizes = [16, 48, 128];

for (const size of sizes) {
  const buffer = generateIcon(size);
  const filename = join(iconsDir, `icon${size}.png`);
  writeFileSync(filename, buffer);
  console.log(`Generated: ${filename}`);
}

console.log('Icons generated successfully!');
