const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const root = 'D:\\QiLin\\Copy share';
const dist = path.join(root, 'dist');
const types = new Map([['.html','text/html; charset=utf-8'],['.js','text/javascript; charset=utf-8'],['.css','text/css; charset=utf-8'],['.ico','image/x-icon'],['.png','image/png'],['.svg','image/svg+xml']]);
const server = http.createServer((req, res) => {
  const raw = (req.url || '/').split('?')[0].split('#')[0];
  let rel = decodeURIComponent(raw === '/' ? '/index.html' : raw).replace(/^\/+/, '');
  let file = path.normalize(path.join(dist, rel));
  if (!file.startsWith(dist)) { res.writeHead(403); res.end('Forbidden'); return; }
  if (!fs.existsSync(file) || fs.statSync(file).isDirectory()) file = path.join(dist, 'index.html');
  const ext = path.extname(file);
  res.writeHead(200, { 'Content-Type': types.get(ext) || 'application/octet-stream', 'Cache-Control': 'no-store' });
  fs.createReadStream(file).pipe(res);
});
server.listen(61237, '127.0.0.1');