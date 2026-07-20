#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const auditScript = path.join(scriptDir, 'audit-slskdn-controller-routes.mjs');
const args = process.argv.slice(2);
function option(name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}
const target = option('--target') ?? 'slskdn';
if (!['slskd', 'slskdn'].includes(target)) {
  throw new Error('--target must be slskd or slskdn');
}
const registryPath = path.resolve(
  option('--registry') ??
    path.join(repoRoot, `crates/slskr/data/${target}-controller-auth-policy.json`),
);
const rootIndex = args.indexOf('--slskdn-root');
const auditArgs = [auditScript];
if (rootIndex >= 0) {
  if (!args[rootIndex + 1]) {
    throw new Error('--slskdn-root requires a path');
  }
  auditArgs.push('--slskdn-root', args[rootIndex + 1]);
}
auditArgs.push('--json');

const inventory = JSON.parse(
  execFileSync(process.execPath, auditArgs, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'inherit'],
    maxBuffer: 32 * 1024 * 1024,
  }),
);

function accessFor(row) {
  if (row.auth.allowAnonymous) return 'anonymous';
  if (!row.auth.explicit) return 'delegated';
  if (
    row.auth.roles.includes('AdministratorOnly') ||
    row.auth.roles.includes('Administrator')
  ) {
    return 'administrator';
  }
  if (row.auth.roles.includes('ReadWriteOrAdministrator')) return 'read_write';
  return 'authenticated';
}

function schemeFor(row) {
  if (row.auth.policies.includes('JwtOnly')) return 'jwt';
  if (row.auth.policies.includes('ApiKeyOnly')) return 'api_key';
  return 'any';
}

const registry = inventory.map((row) => ({
  method: row.method,
  route: row.route,
  access: accessFor(row),
  scheme: schemeFor(row),
  scopes: row.auth.scopes,
}));
const serialized = `${JSON.stringify(registry, null, 2)}\n`;

if (args.includes('--write')) {
  writeFileSync(registryPath, serialized);
  process.stdout.write(`wrote ${registry.length} ${target} controller auth policies\n`);
} else {
  let committed;
  try {
    committed = readFileSync(registryPath, 'utf8');
  } catch {
    process.stderr.write(`${target} controller auth registry is missing: ${registryPath}\n`);
    process.exit(1);
  }
  if (committed !== serialized) {
    process.stderr.write(
      `${target} controller auth registry drifted; review upstream policies and regenerate with --write\n`,
    );
    process.exit(1);
  }
  process.stdout.write(`${target} controller auth registry check passed: ${registry.length} routes\n`);
}
