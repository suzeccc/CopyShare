const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');

const root = process.cwd();
const port = Number(process.env.PREVIEW_PORT || 61238);
const host = '127.0.0.1';
const types = new Map([
  ['.html', 'text/html; charset=utf-8'],
  ['.css', 'text/css; charset=utf-8'],
  ['.js', 'text/javascript; charset=utf-8'],
  ['.png', 'image/png'],
  ['.jpg', 'image/jpeg'],
  ['.jpeg', 'image/jpeg'],
  ['.svg', 'image/svg+xml; charset=utf-8'],
]);

function send(res, status, body, type = 'text/plain; charset=utf-8') {
  res.writeHead(status, { 'content-type': type, 'cache-control': 'no-store' });
  res.end(body);
}

const server = http.createServer((req, res) => {
  const url = new URL(req.url || '/', `http://${host}:${port}`);
  const pathname = decodeURIComponent(url.pathname === '/' ? '/previews/mobile-qr-layouts.html' : url.pathname);
  const file = path.resolve(root, `.${pathname}`);
  if (!file.startsWith(root + path.sep)) {
    send(res, 403, 'Forbidden');
    return;
  }
  fs.readFile(file, (error, data) => {
    if (error) {
      send(res, 404, 'Not found');
      return;
    }
    send(res, 200, data, types.get(path.extname(file).toLowerCase()) || 'application/octet-stream');
  });
});

server.listen(port, host, () => {
  console.log(`preview http://${host}:${port}/previews/mobile-qr-layouts.html`);
});