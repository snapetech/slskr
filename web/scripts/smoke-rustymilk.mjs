import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const distDir = resolve(repoRoot, process.env.SLSKR_RUST_WEB_DIST || 'target/slskr-web');

const build = spawnSync('scripts/build-rust-web.sh', {
  cwd: repoRoot,
  stdio: 'inherit',
});
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

const mimeTypes = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.wasm': 'application/wasm',
  '.css': 'text/css',
};

const presetSource = `
  name=Rust WASM smoke
  decay=0.88
  wave_r=0.9
  wave_g=0.45
  wave_b=0.18
  wave_a=0.9
  wave_scale=1.3
  per_frame_1=rot=0.02*sin(time);
  per_pixel_1=dx=0.01*sin((x+time)*6.28);
  shape00_enabled=1
  shape00_sides=5
  shape00_rad=0.22
  shape00_a=0.45
  wavecode_0_enabled=1
  wavecode_0_samples=32
  wavecode_0_per_point1=x=i;
  wavecode_0_per_point2=y=0.5+sample*0.25;
`;

const smokeHtml = `
  <!doctype html>
  <html>
    <body>
      <div id="root"></div>
      <canvas id="canvas" width="240" height="160"></canvas>
      <script type="module">
        import init, { RustyMilkEngine } from '/slskr_web.js';

        await init({ module_or_path: '/slskr_web_bg.wasm' });
        const canvas = document.getElementById('canvas');
        const engine = new RustyMilkEngine(canvas);
        engine.resize(canvas.width, canvas.height);
        engine.loadPresetText(${JSON.stringify(presetSource)}, 'smoke.milk', '{}');

        for (let frame = 0; frame < 8; frame += 1) {
          engine.render(
            frame / 30,
            0.4 + frame * 0.02,
            0.3,
            0.2,
            '-1,-0.5,0,0.5,1,0.5,0,-0.5',
            '0.1,0.25,0.8,0.35,0.2,0.1',
            0,
            0.5,
            0.5,
            0,
            0,
          );
        }

        const gl = canvas.getContext('webgl2');
        const pixels = new Uint8Array(canvas.width * canvas.height * 4);
        gl.readPixels(0, 0, canvas.width, canvas.height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
        let litPixels = 0;
        let channelTotal = 0;
        for (let index = 0; index < pixels.length; index += 4) {
          const total = pixels[index] + pixels[index + 1] + pixels[index + 2];
          if (total > 12) litPixels += 1;
          channelTotal += total;
        }
        window.__rustyMilkSmoke = {
          channelTotal,
          litPixels,
          pixelCount: canvas.width * canvas.height,
          renderer: engine.rendererLabel(),
        };
      </script>
    </body>
  </html>
`;

const server = createServer(async (request, response) => {
  try {
    if (request.url === '/rustymilk-smoke') {
      response.writeHead(200, { 'Content-Type': 'text/html' });
      response.end(smokeHtml);
      return;
    }
    const requestPath = decodeURIComponent((request.url || '/').split('?')[0]);
    const filePath = join(distDir, requestPath === '/' ? 'index.html' : requestPath);
    const body = await readFile(filePath);
    response.writeHead(200, {
      'Content-Type': mimeTypes[extname(filePath)] || 'application/octet-stream',
    });
    response.end(body);
  } catch {
    response.writeHead(404);
    response.end('not found');
  }
});

await new Promise((resolveServer) => {
  server.listen(0, '127.0.0.1', resolveServer);
});

const { port } = server.address();
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
const browserMessages = [];
page.on('console', (message) => {
  browserMessages.push(`${message.type()}: ${message.text()}`);
});
page.on('pageerror', (error) => {
  browserMessages.push(`pageerror: ${error.message}`);
});

try {
  await page.goto(`http://127.0.0.1:${port}/rustymilk-smoke`);
  const handle = await page.waitForFunction(() => window.__rustyMilkSmoke, null, {
    timeout: 10_000,
  });
  const stats = await handle.jsonValue();
  if (stats.litPixels < stats.pixelCount * 0.05 || stats.channelTotal <= 0) {
    throw new Error(`RustyMilk smoke rendered a blank canvas: ${JSON.stringify(stats)}`);
  }
  console.log(`RustyMilk smoke passed: ${JSON.stringify(stats)}`);
} catch (error) {
  if (browserMessages.length > 0) {
    console.log(browserMessages.join('\n'));
  }
  throw error;
} finally {
  await browser.close();
  await new Promise((resolveClose) => server.close(resolveClose));
}
