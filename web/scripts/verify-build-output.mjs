import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const buildDir = path.resolve(scriptDir, '..', 'build');
const indexPath = path.join(buildDir, 'index.html');
const manifestPath = path.join(buildDir, 'manifest.json');

function fail(message) {
  console.error(`ERROR: ${message}`);
  process.exit(1);
}

if (!fs.existsSync(indexPath)) {
  fail(`Missing built index.html at ${indexPath}`);
}

const html = fs.readFileSync(indexPath, 'utf8');

const requiredPatterns = [
  { pattern: /(?:src|href)="\.\/assets\//, reason: 'expected relative built asset URLs for reverse-proxy subpaths' },
  { pattern: /href="\.\/favicon\.ico"/, reason: 'expected relative favicon path for reverse-proxy subpaths' },
  { pattern: /href="\.\/manifest\.json"/, reason: 'expected relative manifest path for reverse-proxy subpaths' },
];

const forbiddenPatterns = [
  { pattern: /(?:src|href)="\/assets\//, reason: 'root-relative assets break non-root web.url_base deployments' },
];

for (const { pattern, reason } of requiredPatterns) {
  if (!pattern.test(html)) {
    fail(`Built index.html is missing an expected path (${pattern}): ${reason}`);
  }
}

for (const { pattern, reason } of forbiddenPatterns) {
  if (pattern.test(html)) {
    fail(`Built index.html contains a forbidden path (${pattern}): ${reason}`);
  }
}

const requiredFiles = [
  'manifest.json',
  'favicon.ico',
  'favicon-16x16.png',
  'favicon-32x32.png',
  'apple-touch-icon.png',
  'logo192.png',
  'logo512.png',
  'service-worker.js',
];

for (const file of requiredFiles) {
  if (!fs.existsSync(path.join(buildDir, file))) {
    fail(`Built web output is missing required public asset: ${file}`);
  }
}

const assetDir = path.join(buildDir, 'assets');
if (!fs.existsSync(assetDir) || !fs.statSync(assetDir).isDirectory()) {
  fail('Built web output is missing the assets directory');
}

const assetFiles = fs.readdirSync(assetDir);
const cssAssets = assetFiles.filter((file) => file.endsWith('.css'));
const jsAssets = assetFiles.filter((file) => file.endsWith('.js'));
if (cssAssets.length === 0) {
  fail('Built web output is missing a CSS asset; the app will render unstyled');
}
if (jsAssets.length === 0) {
  fail('Built web output is missing a JavaScript asset');
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
const iconSources = new Set((manifest.icons || []).map((icon) => icon.src));
for (const iconPath of ['./logo192.png', './logo512.png']) {
  if (!iconSources.has(iconPath)) {
    fail(`Built manifest.json is missing expected relative icon path: ${iconPath}`);
  }
}

console.log('Verified built web output uses subpath-safe relative asset references.');
