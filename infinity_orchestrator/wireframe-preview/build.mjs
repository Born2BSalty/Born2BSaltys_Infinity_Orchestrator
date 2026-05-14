// Pre-compiles JSX with Babel and inlines React + ReactDOM into a single shareable HTML.
// Output: ../Infinity Orchestrator Wireframe (build).html

import babel from '@babel/core';
import fs from 'node:fs';
import path from 'node:path';
import https from 'node:https';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const ttf2woff2 = require('ttf2woff2').default;

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PREVIEW = __dirname;
const OUT = path.join(PREVIEW, '..', 'Infinity Orchestrator Wireframe (build).html');
const CACHE = '/tmp/wireframe-cdn-cache';
fs.mkdirSync(CACHE, { recursive: true });

function fetch(url, name) {
  const p = path.join(CACHE, name);
  if (fs.existsSync(p)) return fs.readFileSync(p, 'utf-8');
  return new Promise((resolve, reject) => {
    const handle = (u, hops = 0) => {
      if (hops > 5) return reject(new Error('too many redirects'));
      https
        .get(u, (resp) => {
          if ([301, 302, 307, 308].includes(resp.statusCode)) {
            return handle(resp.headers.location, hops + 1);
          }
          if (resp.statusCode !== 200) {
            return reject(new Error(`${u} → HTTP ${resp.statusCode}`));
          }
          const chunks = [];
          resp.on('data', (c) => chunks.push(c));
          resp.on('end', () => {
            const buf = Buffer.concat(chunks);
            fs.writeFileSync(p, buf);
            resolve(buf.toString('utf-8'));
          });
          resp.on('error', reject);
        })
        .on('error', reject);
    };
    handle(url);
  });
}

function fetchBinary(url, name) {
  const p = path.join(CACHE, name);
  if (fs.existsSync(p)) return Promise.resolve(fs.readFileSync(p));
  return new Promise((resolve, reject) => {
    const handle = (u, hops = 0) => {
      if (hops > 5) return reject(new Error('too many redirects'));
      https
        .get(u, (resp) => {
          if ([301, 302, 307, 308].includes(resp.statusCode)) {
            return handle(resp.headers.location, hops + 1);
          }
          if (resp.statusCode !== 200) {
            return reject(new Error(`${u} → HTTP ${resp.statusCode}`));
          }
          const chunks = [];
          resp.on('data', (c) => chunks.push(c));
          resp.on('end', () => {
            const buf = Buffer.concat(chunks);
            fs.writeFileSync(p, buf);
            resolve(buf);
          });
          resp.on('error', reject);
        })
        .on('error', reject);
    };
    handle(url);
  });
}

function compile(file) {
  const src = fs.readFileSync(file, 'utf8');
  const code = babel.transformSync(src, { presets: ['@babel/preset-react'], filename: file }).code;
  // Wrap in an IIFE so top-level `const`/`let` declarations in each file don't
  // collide across script blocks (e.g. `const { useState } = React;` appears in
  // all three files). Mirrors what babel-standalone does for in-page <script type="text/babel">.
  return `(function () {\n${code}\n})();`;
}

const react = await fetch(
  'https://unpkg.com/react@18.3.1/umd/react.production.min.js',
  'react.prod.js'
);
const reactDom = await fetch(
  'https://unpkg.com/react-dom@18.3.1/umd/react-dom.production.min.js',
  'react-dom.prod.js'
);

// Inline Fira Code *Nerd Font Mono* — Nerd-patched, Mono variant (fixed glyph
// width even for icons). Source: ryanoasis/nerd-fonts repo via jsDelivr.
// We download TTF and convert to woff2 with ttf2woff2 (saves ~50%). Conversion
// is cached. In the build, the local `url('firacode-nerd-XXX.woff2')` refs in
// index.html are rewritten to base64 data: URLs so the HTML is fully self-contained.
const FONT_WEIGHTS = [
  { weight: 300, url: 'https://cdn.jsdelivr.net/gh/ryanoasis/nerd-fonts@v3.1.1/patched-fonts/FiraCode/Light/FiraCodeNerdFontMono-Light.ttf', name: 'firacode-nerd-300' },
  { weight: 500, url: 'https://cdn.jsdelivr.net/gh/ryanoasis/nerd-fonts@v3.1.1/patched-fonts/FiraCode/Medium/FiraCodeNerdFontMono-Medium.ttf', name: 'firacode-nerd-500' },
];
const fontDataUrls = {};
for (const { url, name } of FONT_WEIGHTS) {
  const ttfBuf = await fetchBinary(url, `${name}.ttf`);
  const woff2Path = path.join(CACHE, `${name}.woff2`);
  let woff2Buf;
  if (fs.existsSync(woff2Path)) {
    woff2Buf = fs.readFileSync(woff2Path);
  } else {
    woff2Buf = ttf2woff2(ttfBuf);
    fs.writeFileSync(woff2Path, woff2Buf);
  }
  fontDataUrls[`${name}.woff2`] = `data:font/woff2;base64,${woff2Buf.toString('base64')}`;
}

// Poppins woff2 files are committed alongside index.html; read them locally and inline as data URLs.
for (const name of ['poppins-300', 'poppins-500', 'poppins-700']) {
  const filePath = path.join(PREVIEW, `${name}.woff2`);
  if (fs.existsSync(filePath)) {
    const buf = fs.readFileSync(filePath);
    fontDataUrls[`${name}.woff2`] = `data:font/woff2;base64,${buf.toString('base64')}`;
  }
}

const tweaks = compile(path.join(PREVIEW, 'tweaks-panel.jsx'));
const screens = compile(path.join(PREVIEW, 'screens.jsx'));
const app = compile(path.join(PREVIEW, 'app.jsx'));

let html = fs.readFileSync(path.join(PREVIEW, 'index.html'), 'utf8');

const replacements = [
  // Rewrite local woff2 references in index.html @font-face blocks to base64 data: URLs
  // so the build is fully self-contained.
  ...Object.entries(fontDataUrls).map(([file, dataUrl]) => [
    new RegExp(`url\\('${file.replace('.', '\\.')}'\\)`, 'g'),
    `url(${dataUrl})`,
  ]),
  [
    /<script src="https:\/\/unpkg\.com\/react@[^"]*"[^>]*><\/script>/,
    `<script>${react}</script>`,
  ],
  [
    /<script src="https:\/\/unpkg\.com\/react-dom@[^"]*"[^>]*><\/script>/,
    `<script>${reactDom}</script>`,
  ],
  [
    /<script src="https:\/\/unpkg\.com\/@babel\/standalone[^"]*"[^>]*><\/script>/,
    '',
  ],
  [
    /<script type="text\/babel" src="tweaks-panel\.jsx"><\/script>/,
    `<script>${tweaks}</script>`,
  ],
  [
    /<script type="text\/babel" src="screens\.jsx"><\/script>/,
    `<script>${screens}</script>`,
  ],
  [
    /<script type="text\/babel" src="app\.jsx"><\/script>/,
    `<script>${app}</script>`,
  ],
];

for (const [re, rep] of replacements) {
  if (!re.test(html)) {
    console.warn('warn: regex did not match:', re);
  }
  // Use a function replacer so $-patterns inside the replacement (e.g. $& in
  // minified React) aren't reinterpreted by String.prototype.replace.
  html = html.replace(re, () => rep);
}

// Swap title so the built file is identifiable
html = html.replace(
  '<title>Infinity Orchestrator · Wireframe (preview)</title>',
  '<title>Infinity Orchestrator · Wireframe</title>'
);

fs.writeFileSync(OUT, html);
const kb = (fs.statSync(OUT).size / 1024).toFixed(1);
console.log(`wrote ${OUT} (${kb} KB)`);
