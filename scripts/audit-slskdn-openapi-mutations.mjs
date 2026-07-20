#!/usr/bin/env node

import { readFileSync } from 'node:fs';

const args = process.argv.slice(2);
function option(name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

const source = option('--openapi');
if (!source) {
  process.stderr.write('usage: audit-slskdn-openapi-mutations.mjs --openapi FILE_OR_URL\n');
  process.exit(2);
}

const document = source.startsWith('http://') || source.startsWith('https://')
  ? await fetch(source).then(async (response) => {
      if (!response.ok) throw new Error(`OpenAPI request failed with ${response.status}`);
      return response.json();
    })
  : JSON.parse(readFileSync(source, 'utf8'));

const schemas = document.components?.schemas ?? {};

function propertyString(name, schema) {
  const lower = name.toLowerCase();
  if (schema.enum?.length) return schema.enum[0];
  if (lower.includes('contentid')) return 'content:music:recording:route-audit';
  if (lower === 'podid') return 'pod:00000000000000000000000000000001';
  if (lower.includes('externalid')) return 'route-audit';
  if (schema.format === 'uuid' || lower.endsWith('id')) {
    return '00000000-0000-4000-8000-000000000001';
  }
  if (schema.format === 'date-time') return '2026-01-01T00:00:00Z';
  if (schema.format === 'date') return '2026-01-01';
  if (schema.format === 'date-span') return '00:01:00';
  if (schema.format === 'uri' || lower.includes('url')) return 'http://127.0.0.1:1/';
  if (lower === 'pixels') return 'AAAAAA==';
  if (
    schema.format === 'byte' ||
    lower.includes('base64') ||
    lower.includes('publickey') ||
    lower.includes('signature')
  ) {
    return 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=';
  }
  if (lower === 'hasha' || lower === 'hashb') return '0000000000000000';
  if (lower.includes('sha256') || lower.endsWith('hash')) return 'a'.repeat(64);
  if (lower === 'csvtext') return 'Artist,Track Title,Album\nRoute Audit,Parity Track,Contract';
  if (lower.includes('ipaddress') || lower === 'ip') return '192.0.2.1';
  if (lower.includes('username') || lower.includes('peer')) return 'route-audit-peer';
  if (lower.includes('filename')) return 'Route Audit.flac';
  if (lower.includes('directory') || lower.endsWith('path')) return '/tmp/slskdn-route-audit';
  if (lower.includes('email')) return 'route-audit@example.invalid';
  if (lower.includes('domain')) return 'music';
  if (lower.includes('type')) return 'recording';
  if (lower.includes('algorithm')) return 'PHash';
  if (lower.includes('name') || lower.includes('title')) return 'Route Audit';
  if (lower.includes('message') || lower.includes('body') || lower.includes('description')) {
    return 'route audit';
  }
  return 'route-audit';
}

function materialize(schema, name = 'value', depth = 0, seen = new Set()) {
  if (!schema || depth > 8) return null;
  if (schema.$ref) {
    const key = schema.$ref.split('/').at(-1);
    if (!key || seen.has(key)) return null;
    return materialize(schemas[key], name, depth + 1, new Set([...seen, key]));
  }
  if (schema.allOf) {
    return Object.assign(
      {},
      ...schema.allOf.map((part) => materialize(part, name, depth + 1, seen) ?? {}),
    );
  }
  if (schema.oneOf || schema.anyOf) {
    return materialize((schema.oneOf ?? schema.anyOf)[0], name, depth + 1, seen);
  }
  if (schema.enum?.length) return schema.enum[0];
  switch (schema.type) {
    case 'object': {
      const value = {};
      for (const [property, propertySchema] of Object.entries(schema.properties ?? {})) {
        if (propertySchema.readOnly) continue;
        value[property] = materialize(propertySchema, property, depth + 1, seen);
      }
      return value;
    }
    case 'array':
      if (name.toLowerCase().includes('samples')) return [0.5];
      return schema.minItems > 0
        ? [materialize(schema.items ?? {}, name, depth + 1, seen)]
        : [];
    case 'integer':
    case 'number':
      return Math.max(schema.minimum ?? 1, 1);
    case 'boolean':
      return true;
    case 'string':
      return propertyString(name, schema);
    default:
      return schema.properties ? materialize({ ...schema, type: 'object' }, name, depth, seen) : null;
  }
}

const methods = new Set(['post', 'put', 'patch', 'delete']);
const operations = [];
for (const [route, pathItem] of Object.entries(document.paths ?? {})) {
  for (const [method, operation] of Object.entries(pathItem)) {
    if (!methods.has(method)) continue;
    const schema = operation.requestBody?.content?.['application/json']?.schema;
    operations.push({
      method: method.toUpperCase(),
      route,
      operationId: operation.operationId ?? null,
      requestBody: schema ? materialize(schema) : null,
      declaredResponses: Object.keys(operation.responses ?? {}),
    });
  }
}

operations.sort((left, right) =>
  `${left.method} ${left.route}`.localeCompare(`${right.method} ${right.route}`),
);
process.stdout.write(`${JSON.stringify(operations, null, 2)}\n`);
process.stderr.write(
  `slskdN OpenAPI mutation inventory: ${operations.length} operations; ` +
    `${operations.filter((operation) => operation.requestBody !== null).length} JSON fixtures\n`,
);
