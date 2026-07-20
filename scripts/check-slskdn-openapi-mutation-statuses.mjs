#!/usr/bin/env node

import { readFileSync } from 'node:fs';

const args = process.argv.slice(2);
function option(name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

const openapiPath = option('--openapi');
const fixturesPath = option('--fixtures');
const referenceBase = option('--reference-base');
const candidateBase = option('--candidate-base');
if (!openapiPath || !fixturesPath || !referenceBase || !candidateBase) {
  process.stderr.write(
    'usage: check-slskdn-openapi-mutation-statuses.mjs --openapi FILE --fixtures FILE ' +
      '--reference-base URL --candidate-base URL\n',
  );
  process.exit(2);
}

const document = JSON.parse(readFileSync(openapiPath, 'utf8'));
const fixtures = JSON.parse(readFileSync(fixturesPath, 'utf8'));
const schemas = document.components?.schemas ?? {};
const parameters = document.components?.parameters ?? {};
const fixtureByOperation = new Map(
  fixtures.map((fixture) => [`${fixture.method} ${fixture.route}`, fixture]),
);
const failures = [];
const referenceUnavailableFailures = [];
const skipped = [];
let compared = 0;

function resolve(value, collection) {
  if (!value?.$ref) return value;
  return collection[value.$ref.split('/').at(-1)] ?? value;
}

function parameterValue(name, schema = {}) {
  const lower = name.toLowerCase();
  if (lower === 'version') return '0';
  if (schema.type === 'integer' || schema.type === 'number') return '1';
  if (schema.format === 'uuid') return '00000000-0000-4000-8000-000000000001';
  if (lower.includes('contentid')) return 'content:music:recording:route-audit';
  if (lower === 'podid') return 'pod:00000000000000000000000000000001';
  if (lower.includes('ipaddress') || lower === 'ip') return '192.0.2.1';
  if (lower.includes('base64')) return Buffer.from('/tmp/slskdn-route-audit').toString('base64');
  if (lower.includes('username') || lower.includes('peer')) return 'route-audit-peer';
  if (lower.includes('filename')) return 'Route Audit.flac';
  if (lower.includes('room')) return 'route-audit-room';
  if (lower.includes('channel')) return 'route-audit-channel';
  if (lower.includes('type')) return 'username';
  if (lower.includes('target')) return 'route-audit-peer';
  if (lower.endsWith('id') || lower === 'id') return '00000000-0000-4000-8000-000000000001';
  return 'route-audit';
}

function materializeRoute(route, pathItem, operation) {
  const operationParameters = [...(pathItem.parameters ?? []), ...(operation.parameters ?? [])]
    .map((parameter) => resolve(parameter, parameters))
    .filter((parameter) => parameter?.in === 'path');
  const values = new Map(
    operationParameters.map((parameter) => [
      parameter.name,
      parameterValue(parameter.name, resolve(parameter.schema, schemas)),
    ]),
  );
  return route.replaceAll(/\{([^}]+)\}/g, (_, name) =>
    encodeURIComponent(values.get(name) ?? parameterValue(name)),
  );
}

function skipReason(method, route) {
  if (route.startsWith('/api/v0/application')) {
    return 'application lifecycle mutation';
  }
  if (method === 'DELETE' && ['/api/v0/server', '/api/v0/session'].includes(route)) {
    return 'server lifecycle mutation';
  }
  if (method === 'DELETE' && route.includes('/files/')) return 'host filesystem deletion';
  return null;
}

async function request(base, method, route, body) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10_000);
  try {
    const headers = { Accept: 'application/json' };
    if (body !== null && body !== undefined) headers['Content-Type'] = 'application/json';
    const response = await fetch(`${base.replace(/\/$/, '')}${route}`, {
      method,
      headers,
      body: body === null || body === undefined ? undefined : JSON.stringify(body),
      signal: controller.signal,
    });
    const text = await response.text();
    let json = null;
    if (text) {
      try {
        json = JSON.parse(text);
      } catch {}
    }
    return { status: response.status, text, json };
  } catch (error) {
    return { status: 0, text: String(error), json: null };
  } finally {
    clearTimeout(timeout);
  }
}

function jsonKind(value) {
  if (value === null) return 'null';
  return Array.isArray(value) ? 'array' : typeof value;
}

const methods = new Set(['post', 'put', 'patch', 'delete']);
for (const [route, pathItem] of Object.entries(document.paths ?? {})) {
  for (const [methodLower, operation] of Object.entries(pathItem)) {
    if (!methods.has(methodLower)) continue;
    const method = methodLower.toUpperCase();
    const reason = skipReason(method, route);
    if (reason) {
      skipped.push(`${method} ${route}: ${reason}`);
      continue;
    }
    const fixture = fixtureByOperation.get(`${method} ${route}`);
    const materializedRoute = materializeRoute(route, pathItem, operation);
    const [reference, candidate] = await Promise.all([
      request(referenceBase, method, materializedRoute, fixture?.requestBody),
      request(candidateBase, method, materializedRoute, fixture?.requestBody),
    ]);
    compared += 1;
    const label = `${method} ${materializedRoute}`;
    if (reference.status !== candidate.status) {
      const failure =
        `${label}: status reference=${reference.status} candidate=${candidate.status}; ` +
        `reference=${reference.text.slice(0, 180)} candidate=${candidate.text.slice(0, 180)}`;
      failures.push(failure);
      if (reference.status === 0 || reference.status >= 500) {
        referenceUnavailableFailures.push(failure);
      }
      continue;
    }
    if (reference.status >= 200 && reference.status < 300 && reference.text) {
      if (jsonKind(reference.json) !== jsonKind(candidate.json)) {
        failures.push(
          `${label}: JSON type reference=${jsonKind(reference.json)} candidate=${jsonKind(candidate.json)}`,
        );
        continue;
      }
      if (
        reference.json &&
        candidate.json &&
        !Array.isArray(reference.json) &&
        typeof reference.json === 'object' &&
        typeof candidate.json === 'object'
      ) {
        const missing = Object.keys(reference.json).filter(
          (key) => !Object.hasOwn(candidate.json, key),
        );
        if (missing.length) failures.push(`${label}: candidate missing keys ${missing.join(', ')}`);
      }
    }
  }
}

process.stderr.write(
  `slskdN OpenAPI mutation differential: ${compared - failures.length}/${compared} operations matched; ` +
    `${skipped.length} destructive lifecycle/filesystem operations skipped\n`,
);
if (failures.length) {
  process.stderr.write(
    `actionable reference-contract mismatches: ${failures.length - referenceUnavailableFailures.length}; ` +
      `reference timeouts/5xx mismatches: ${referenceUnavailableFailures.length}\n`,
  );
}
for (const failure of failures) process.stderr.write(`- ${failure}\n`);
if (failures.length) process.exit(1);
