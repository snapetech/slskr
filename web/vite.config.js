import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const nodeModulesPath = path.resolve(__dirname, 'node_modules');

const resolveNodeModuleLessImport = (filename) => {
  const requested = filename.slice(1);
  if (path.isAbsolute(requested)) {
    throw new Error(`absolute Less alias imports are not allowed: ${filename}`);
  }
  if (requested.split(/[\\/]+/u).includes('..')) {
    throw new Error(`Less alias import escapes node_modules: ${filename}`);
  }
  const resolved = path.resolve(nodeModulesPath, requested); // nosemgrep: javascript.lang.security.audit.path-traversal.path-join-resolve-traversal.path-join-resolve-traversal
  const relative = path.relative(nodeModulesPath, resolved);
  if (relative === '' || relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error(`Less alias import escapes node_modules: ${filename}`);
  }
  return resolved;
};

// Custom Less plugin to handle semantic-ui-less webpack-style aliases.
// @semantic-ui-react/craco-less previously provided these via webpack Less loader.
// Handles:
//   - '../../theme.config' (imported by node_modules/semantic-ui-less/semantic.less)
//     → src/semantic-ui/theme.config
//   - '~foo/bar' (webpack-style node_modules imports in Less files)
//     → node_modules/foo/bar
class SemanticUILessFileManager {
  install(less, pluginManager) {
    const themeConfigPath = path.resolve(__dirname, 'src/semantic-ui/theme.config');

    class AliasFileManager extends less.FileManager {
      supports(filename) {
        return filename === '../../theme.config' || filename.startsWith('~');
      }

      loadFile(filename, _currentDirectory, options, environment) {
        if (filename === '../../theme.config') {
          const contents = fs.readFileSync(themeConfigPath, 'utf-8');
          return Promise.resolve({ filename: themeConfigPath, contents });
        }
        if (filename.startsWith('~')) {
          const resolved = resolveNodeModuleLessImport(filename);
          const contents = fs.readFileSync(resolved, 'utf-8');
          return Promise.resolve({ filename: resolved, contents });
        }
        return super.loadFile(filename, _currentDirectory, options, environment);
      }
    }

    pluginManager.addFileManager(new AliasFileManager());
  }

  get minVersion() {
    return [2, 7, 0];
  }
}

export default defineConfig({
  base: './',
  plugins: [react()],

  css: {
    preprocessorOptions: {
      less: {
        math: 'always',
        relativeUrls: true,
        javascriptEnabled: true,
        paths: [nodeModulesPath],
        plugins: [new SemanticUILessFileManager()],
      },
    },
  },

  server: {
    port: 3001,
    proxy: {
      '/api': {
        target: 'http://localhost:5030',
        changeOrigin: true,
      },
      '/hub': {
        target: 'http://localhost:5030',
        ws: true,
        changeOrigin: true,
      },
    },
  },

  build: {
    outDir: 'build',
    emptyOutDir: true,
    chunkSizeWarningLimit: 700,
    // Vite 8 defaults to lightningcss for CSS minification, which rejects some
    // valid-but-unusual CSS emitted by semantic-ui-less (e.g. 0.0px dimensions).
    // Use esbuild CSS minifier instead, which is tolerant of this output.
    cssMinify: 'esbuild',
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) return undefined;

          if (id.includes('butterchurn-presets')) return 'milkdrop-presets';
          if (id.includes('butterchurn')) return 'milkdrop';
          if (id.includes('semantic-ui')) return 'semantic-ui';
          if (
            id.includes('/react/') ||
            id.includes('/react-dom/') ||
            id.includes('/react-router') ||
            id.includes('/scheduler/')
          ) {
            return 'react-vendor';
          }
          if (id.includes('axios')) return 'api-vendor';

          return 'vendor';
        },
      },
    },
  },

  define: {
    // Shim for any remaining process.env.NODE_ENV references
    'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV || 'development'),
  },

  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/setupTests.js'],
    exclude: ['e2e/**', 'node_modules/**'],
  },
});
