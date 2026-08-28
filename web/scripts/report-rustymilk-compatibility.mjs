import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const jsonOutput = process.argv.includes('--json');
const result = spawnSync('cargo', [
  'test',
  '-p',
  'slskr-web',
  'rustymilk_compatibility',
  '--',
  '--nocapture',
], {
  cwd: repoRoot,
  encoding: 'utf8',
});

if (jsonOutput) {
  console.log(JSON.stringify({
    source: 'rust-wasm',
    status: result.status === 0 ? 'supported' : 'failed',
  }, null, 2));
} else {
  process.stdout.write(result.stdout || '');
  process.stderr.write(result.stderr || '');
}

process.exit(result.status ?? 1);
