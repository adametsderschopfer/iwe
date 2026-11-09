import { createServer } from 'node:http'
import { readFile, stat } from 'node:fs/promises'
import { dirname, extname, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const output = resolve(dirname(fileURLToPath(import.meta.url)), '../../.output/public')
const basePath = `/${process.env.NUXT_APP_BASE_URL || '/'}/`.replace(/\/+/g, '/')
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.webp': 'image/webp',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.xml': 'application/xml; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.woff2': 'font/woff2',
}

const server = createServer(async (request, response) => {
  if (!['GET', 'HEAD'].includes(request.method ?? '')) {
    response.writeHead(405).end()
    return
  }
  try {
    const pathname = decodeURIComponent(new URL(request.url ?? '/', 'http://127.0.0.1').pathname)
    if (!pathname.startsWith(basePath)) {
      response.writeHead(404).end('Not found')
      return
    }
    let file = resolve(output, pathname.slice(basePath.length))
    if (file !== output && !file.startsWith(`${output}${sep}`)) {
      response.writeHead(404).end('Not found')
      return
    }
    if ((await stat(file)).isDirectory()) file = resolve(file, 'index.html')
    const body = await readFile(file)
    response.writeHead(200, {
      'Content-Type': types[extname(file)] ?? 'application/octet-stream',
      'Cache-Control': 'no-store',
      'Content-Length': body.length,
    })
    response.end(request.method === 'HEAD' ? undefined : body)
  } catch {
    response.writeHead(404).end('Not found')
  }
})

server.listen(4182, '127.0.0.1', () => {
  console.log(`Static test website: http://127.0.0.1:4182${basePath}`)
})
