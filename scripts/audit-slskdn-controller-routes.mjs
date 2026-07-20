#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const args = process.argv.slice(2);

function option(name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

const siblingRoot = path.resolve(
  option('--slskdn-root') ?? process.env.SLSKR_SLSKDN_REPO ?? path.join(repoRoot, '../slskdn'),
);
const controllerRoot = path.join(siblingRoot, 'src/slskd');
const probeBase = option('--probe-base');
const staticGetOnly = args.includes('--static-get');
const getOnly = args.includes('--get');
const staticOnly = args.includes('--static');
const materialize = args.includes('--materialize');
const jsonOutput = args.includes('--json');
const unmatchedOnly = args.includes('--unmatched-only');
const fallbackOnly = args.includes('--fallback-only');
const failOnUnmatched = args.includes('--fail-on-unmatched');
const failOnFallback = args.includes('--fail-on-fallback');
const includeResponse = args.includes('--include-response');

function controllerFiles() {
  return execFileSync('rg', ['--files', controllerRoot, '-g', '*Controller.cs'], {
    encoding: 'utf8',
  })
    .trim()
    .split('\n')
    .filter(Boolean);
}

function normalizeRoute(route, controllerName) {
  return (`/${route}`)
    .replaceAll('[controller]', controllerName.toLowerCase())
    .replace(/v\{version:apiVersion\}/g, 'v0')
    .replace(/\/+/g, '/')
    .replace(/\/$/, '') || '/';
}

function authorizationMetadata(classAttributes, methodAttributes) {
  const allowAnonymous = /\[AllowAnonymous(?:Attribute)?\b/.test(methodAttributes);
  const attributes = `${classAttributes}\n${methodAttributes}`;
  const authorize = [...attributes.matchAll(/\[Authorize(?:Attribute)?(?:\(([^\]]*)\))?\]/g)];
  const policies = authorize
    .map((match) => match[1]?.match(/Policy\s*=\s*AuthPolicy\.(\w+)/)?.[1])
    .filter(Boolean);
  const roles = authorize
    .map((match) => {
      const args = match[1] ?? '';
      return (
        args.match(/Roles\s*=\s*AuthRole\.(\w+)/)?.[1] ??
        args.match(/Roles\s*=\s*"([^"]+)"/)?.[1]
      );
    })
    .filter(Boolean);
  const scopes = [...methodAttributes.matchAll(/\[RequireScope\("([^"]+)"\)\]/g)].map(
    (match) => match[1],
  );
  return {
    allowAnonymous,
    explicit: authorize.length > 0,
    policies: [...new Set(policies)],
    roles: [...new Set(roles)],
    scopes: [...new Set(scopes)],
  };
}

function extractRoutes(file) {
  const source = readFileSync(file, 'utf8');
  const classIndex = source.search(/\bclass\s+\w+Controller\b/);
  if (classIndex < 0) return [];

  const classPrefix = source.slice(Math.max(0, classIndex - 3_000), classIndex);
  const baseRoutes = [...classPrefix.matchAll(/\[Route\("([^"]+)"\)\]/g)].map(
    (match) => match[1],
  );
  if (baseRoutes.length === 0) return [];

  const controllerName =
    source.slice(classIndex).match(/class\s+(\w+)Controller/)?.[1] ?? '';
  const classAttributes = classPrefix;
  const classBody = source.slice(classIndex);
  const methodPattern =
    /((?:\s*\[[^\]]+\]\s*)+)(?:public|internal)\s+(?:async\s+)?(?:Task(?:<[^;{]+>)?|ActionResult(?:<[^;{]+>)?|IActionResult|[\w.<>,?]+)\s+\w+\s*\(/g;
  const routes = [];

  for (const methodMatch of classBody.matchAll(methodPattern)) {
    const attributes = methodMatch[1];
    const httpPattern = /\[Http(Get|Post|Put|Patch|Delete)(?:\("([^"]*)"\))?[^\]]*\]/g;
    for (const httpMatch of attributes.matchAll(httpPattern)) {
      for (const baseRoute of baseRoutes) {
        const suffix = httpMatch[2] ?? '';
        routes.push({
          method: httpMatch[1].toUpperCase(),
          route: normalizeRoute(`${baseRoute}/${suffix}`, controllerName),
          controller: path.relative(siblingRoot, file),
          auth: authorizationMetadata(classAttributes, attributes),
        });
      }
    }
  }
  return routes;
}

function inventory() {
  const unique = new Map();
  for (const file of controllerFiles()) {
    for (const route of extractRoutes(file)) {
      unique.set(`${route.method} ${route.route}`, route);
    }
  }
  return [...unique.values()]
    .filter((row) => !staticOnly || !row.route.includes('{'))
    .filter((row) => !getOnly || row.method === 'GET')
    .filter((row) => !staticGetOnly || (row.method === 'GET' && !row.route.includes('{')))
    .sort((left, right) =>
      `${left.method} ${left.route}`.localeCompare(`${right.method} ${right.route}`),
    );
}

function materializeRoute(route) {
  return route.replace(/\{([^}]+)\}/g, (_match, rawName) => {
    const parameter = rawName.replace(/^\*/, '');
    const [rawParameterName, ...constraints] = parameter.split(':');
    const name = rawParameterName.toLowerCase();
    if (constraints.some((constraint) => constraint.toLowerCase() === 'guid')) {
      return '00000000-0000-0000-0000-000000000001';
    }
    if (name.includes('ipaddress')) return '192.0.2.1';
    if (name.includes('port') || name === 'size' || name === 'token' || name === 'index') {
      return '1';
    }
    if (name.includes('intentid') || name.includes('executionid') || name === 'trackid') {
      return '00000000-0000-0000-0000-000000000001';
    }
    if (name.includes('contentid')) return 'content-id';
    if (name.includes('username') || name.includes('peer')) return 'route-audit-peer';
    if (name.includes('tags')) return 'audit,route';
    return 'route-audit-id';
  });
}

function versionedProbeUrl(base, route) {
  const url = new URL(materialize ? materializeRoute(route) : route, base);
  if (/^\/api\/v0(?:\/|$)/.test(route)) url.searchParams.set('api-version', '0');
  if (/^\/api\/v1(?:\/|$)/.test(route)) url.searchParams.set('api-version', '1');
  return url;
}

async function probe(row) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 2_000);
  try {
    const response = await fetch(versionedProbeUrl(probeBase, row.route), {
      method: row.method,
      headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
      body: ['POST', 'PUT', 'PATCH'].includes(row.method) ? '{}' : undefined,
      signal: controller.signal,
    });
    const body = await response.text();
    const htmlFallback =
      (response.headers.get('content-type') ?? '').toLowerCase().includes('text/html') ||
      /^\s*<!doctype\s+html/i.test(body);
    let compatibilityFallback = false;
    try {
      const value = JSON.parse(body);
      compatibilityFallback =
        response.status === 202 &&
        value?.accepted === true &&
        typeof value?.operationId === 'string' &&
        typeof value?.payloadSha256 === 'string';
    } catch {
      compatibilityFallback = false;
    }
    return {
      ...row,
      status: response.status,
      ...(includeResponse
        ? {
            contentType: response.headers.get('content-type') ?? '',
            responseBody: body,
          }
        : {}),
      result:
        response.status === 404 && body.trim() === '{"error":"route not found"}'
          ? 'generic_404'
          : htmlFallback
            ? 'html_fallback'
          : compatibilityFallback
            ? 'compatibility_fallback'
            : 'handled',
    };
  } catch (error) {
    return {
      ...row,
      status: 0,
      result: error instanceof Error ? error.name : 'probe_error',
    };
  } finally {
    clearTimeout(timeout);
  }
}

const rows = inventory();
async function probeAll(input, concurrency = 8) {
  const output = new Array(input.length);
  let next = 0;
  await Promise.all(
    Array.from({ length: Math.min(concurrency, input.length) }, async () => {
      while (next < input.length) {
        const index = next++;
        output[index] = await probe(input[index]);
      }
    }),
  );
  return output;
}

const output = probeBase ? await probeAll(rows) : rows;
const displayed = unmatchedOnly
  ? output.filter((row) => ['generic_404', 'html_fallback'].includes(row.result))
  : fallbackOnly
    ? output.filter((row) => row.result === 'compatibility_fallback')
  : output;

if (jsonOutput) {
  process.stdout.write(`${JSON.stringify(displayed, null, 2)}\n`);
} else {
  const probeColumns = probeBase ? '\tstatus\tresult' : '';
  process.stdout.write(`method\troute\tcontroller${probeColumns}\n`);
  for (const row of displayed) {
    const probeValues = probeBase ? `\t${row.status}\t${row.result}` : '';
    process.stdout.write(`${row.method}\t${row.route}\t${row.controller}${probeValues}\n`);
  }
}

const generic404 = output.filter((row) => row.result === 'generic_404').length;
const htmlFallbacks = output.filter((row) => row.result === 'html_fallback').length;
const compatibilityFallbacks = output.filter(
  (row) => row.result === 'compatibility_fallback',
).length;
process.stderr.write(
  `slskdN controller inventory: ${rows.length} routes${
    probeBase
      ? `; ${generic404} generic slskR 404 responses; ${htmlFallbacks} HTML fallbacks; ${compatibilityFallbacks} compatibility fallbacks`
      : ''
  }\n`,
);
if (failOnUnmatched && generic404 + htmlFallbacks > 0) process.exitCode = 1;
if (failOnFallback && compatibilityFallbacks > 0) process.exitCode = 1;
