#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const parser = require('../web/node_modules/@babel/parser');
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const args = process.argv.slice(2);

function option(name, fallback) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : fallback;
}

function sourceFiles(root) {
  const upstreamLib = path.join(root, 'src/web/src/lib');
  const lib = path.join(root, 'web/src/lib');
  const selected = existsSync(upstreamLib) ? upstreamLib : lib;
  return execFileSync(
    'rg',
    ['--files', selected, '-g', '*.js', '-g', '!*.test.js', '-g', '!*.spec.js'],
    { encoding: 'utf8' },
  )
    .trim()
    .split('\n')
    .filter(Boolean);
}

function walk(node, visit) {
  if (!node || typeof node !== 'object') return;
  visit(node);
  for (const [key, value] of Object.entries(node)) {
    if (key === 'loc' || key === 'start' || key === 'end') continue;
    if (Array.isArray(value)) {
      for (const child of value) walk(child, visit);
    } else if (value && typeof value === 'object' && typeof value.type === 'string') {
      walk(value, visit);
    }
  }
}

function combine(left, right, limit = 32) {
  const values = [];
  for (const a of left) {
    for (const b of right) {
      values.push(`${a}${b}`);
      if (values.length >= limit) return values;
    }
  }
  return values;
}

function evaluate(node, bindings, seen = new Set()) {
  if (!node) return [':var'];
  if (node.type === 'StringLiteral') return [node.value];
  if (node.type === 'TemplateLiteral') {
    let values = [node.quasis[0]?.value?.cooked ?? ''];
    for (let index = 0; index < node.expressions.length; index += 1) {
      const expression = evaluate(node.expressions[index], bindings, seen).map((value) =>
        value.startsWith('/') || value.startsWith('?') ? value : ':var',
      );
      values = combine(values, expression);
      values = combine(values, [node.quasis[index + 1]?.value?.cooked ?? '']);
    }
    return values;
  }
  if (node.type === 'BinaryExpression' && node.operator === '+') {
    return combine(evaluate(node.left, bindings, seen), evaluate(node.right, bindings, seen));
  }
  if (node.type === 'ConditionalExpression') {
    return [
      ...evaluate(node.consequent, bindings, seen),
      ...evaluate(node.alternate, bindings, seen),
    ];
  }
  if (node.type === 'LogicalExpression') {
    return [...evaluate(node.left, bindings, seen), ...evaluate(node.right, bindings, seen)];
  }
  if (node.type === 'Identifier' && bindings.has(node.name) && !seen.has(node.name)) {
    const next = new Set(seen);
    next.add(node.name);
    return evaluate(bindings.get(node.name), bindings, next);
  }
  return [':var'];
}

function normalizeEndpoint(value) {
  let endpoint = value.trim();
  if (!endpoint.startsWith('/')) {
    if (endpoint.startsWith(':') || /^[a-z][a-z+.-]*:/i.test(endpoint)) return null;
    endpoint = `/${endpoint}`;
  }
  endpoint = endpoint.split('?')[0];
  endpoint = endpoint.replace(/:var[^/]*/g, ':var');
  endpoint = endpoint.replace(/([^/]):var$/, '$1');
  endpoint = endpoint.replace(/\/+$/, '') || '/';
  return endpoint;
}

function inventory(root) {
  const endpoints = new Set();
  const sources = new Map();
  const unresolved = [];
  for (const file of sourceFiles(root)) {
    const source = readFileSync(file, 'utf8');
    const ast = parser.parse(source, {
      sourceType: 'module',
      plugins: ['jsx', 'optionalChaining', 'nullishCoalescingOperator'],
    });
    const bindings = new Map();
    walk(ast, (node) => {
      if (node.type === 'VariableDeclarator' && node.id?.type === 'Identifier' && node.init) {
        bindings.set(node.id.name, node.init);
      }
    });
    walk(ast, (node) => {
      if (node.type !== 'CallExpression') return;
      let method;
      if (
        node.callee?.type === 'MemberExpression' &&
        node.callee.object?.type === 'Identifier' &&
        node.callee.object.name === 'api' &&
        node.callee.property?.type === 'Identifier' &&
        ['get', 'post', 'put', 'patch', 'delete'].includes(node.callee.property.name)
      ) {
        method = node.callee.property.name.toUpperCase();
      } else if (node.callee?.type === 'Identifier' && node.callee.name === 'safeGet') {
        method = 'GET';
      } else {
        return;
      }
      const values = [...new Set(evaluate(node.arguments[0], bindings))];
      if (values.length === 1 && values[0] === ':var') return;
      let accepted = 0;
      for (const value of values) {
        const endpoint = normalizeEndpoint(value);
        if (!endpoint || endpoint.replaceAll(':var', '').includes(':')) {
          continue;
        }
        const key = `${method} ${endpoint}`;
        endpoints.add(key);
        const rows = sources.get(key) ?? [];
        rows.push(`${path.relative(root, file)}:${node.loc?.start?.line ?? 0}`);
        sources.set(key, rows);
        accepted += 1;
      }
      if (accepted === 0) {
        unresolved.push({
          file: path.relative(root, file),
          line: node.loc?.start?.line ?? 0,
          method,
          evaluated: values,
        });
      }
    });
  }
  return {
    endpoints: [...endpoints].sort(),
    sources: Object.fromEntries(
      [...sources.entries()].sort(([left], [right]) => left.localeCompare(right)),
    ),
    unresolved,
  };
}

const slskdRoot = path.resolve(option('--slskd-root', path.join(repoRoot, '../slskdn')));
const slskdnRoot = path.resolve(option('--slskdn-root', path.join(repoRoot, '../slskdn')));
const slskrWebRoot = path.resolve(option('--slskr-web-root', repoRoot));
const slskd = inventory(slskdRoot);
const slskdn = inventory(slskdnRoot);
const slskr = inventory(slskrWebRoot);
const slskdSet = new Set(slskd.endpoints);
const slskdnSet = new Set(slskdn.endpoints);
const slskrSet = new Set(slskr.endpoints);
const union = new Set([...slskdSet, ...slskdnSet]);
const report = {
  slskd,
  slskdn,
  slskr,
  comparison: {
    targetUnionCount: union.size,
    commonTargetCount: [...slskdSet].filter((item) => slskdnSet.has(item)).length,
    slskdOnly: [...slskdSet].filter((item) => !slskdnSet.has(item)).sort(),
    slskdnOnly: [...slskdnSet].filter((item) => !slskdSet.has(item)).sort(),
    missingFromSlskrWebCalls: [...union].filter((item) => !slskrSet.has(item)).sort(),
    slskrOnly: [...slskrSet].filter((item) => !union.has(item)).sort(),
  },
};

if (args.includes('--json')) {
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
} else {
  process.stdout.write(
    `WebUI API-call inventory: slskd=${slskd.endpoints.length}, slskdN=${slskdn.endpoints.length}, ` +
      `target union=${union.size}, slskr=${slskr.endpoints.length}, ` +
      `missing slskr calls=${report.comparison.missingFromSlskrWebCalls.length}; ` +
      `unresolved=${slskd.unresolved.length}/${slskdn.unresolved.length}/${slskr.unresolved.length}\n`,
  );
}

if (args.includes('--fail-on-unresolved')) {
  const unresolved = slskd.unresolved.length + slskdn.unresolved.length + slskr.unresolved.length;
  if (unresolved > 0) process.exitCode = 1;
}
if (
  args.includes('--fail-on-missing') &&
  report.comparison.missingFromSlskrWebCalls.length > 0
) {
  process.stderr.write(
    `slskr WebUI is missing ${report.comparison.missingFromSlskrWebCalls.length} frozen target API calls\n`,
  );
  process.exitCode = 1;
}
