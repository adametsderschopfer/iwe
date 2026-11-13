import assert from 'node:assert/strict'
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { dirname, join, resolve, sep } from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const websiteDirectory = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const outputDirectory = join(websiteDirectory, '.output', 'public')
const baseURL = `${(process.env.NUXT_APP_BASE_URL || '/').replace(/\/$/, '')}/`
const siteURL = 'https://adametsderschopfer.github.io/iwe-code/'
const localOrigin = 'https://website.invalid'
const locales = [
  { language: 'en', route: '' },
  { language: 'ru', route: 'ru/' },
  { language: 'es', route: 'es/' },
]

function decode(value) {
  return value.replace(/&(#x[\da-f]+|#\d+|amp|quot|apos|lt|gt);/gi, (_, entity) => {
    if (entity.startsWith('#x')) return String.fromCodePoint(Number.parseInt(entity.slice(2), 16))
    if (entity.startsWith('#')) return String.fromCodePoint(Number.parseInt(entity.slice(1), 10))
    return { amp: '&', quot: '"', apos: "'", lt: '<', gt: '>' }[entity.toLowerCase()]
  })
}

function attributes(source) {
  return Object.fromEntries(
    [...source.matchAll(/([^\s=/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s=<>`]+)))?/g)].map(
      ([, name, doubleQuoted, singleQuoted, unquoted]) => [
        name.toLowerCase(),
        decode(doubleQuoted ?? singleQuoted ?? unquoted ?? ''),
      ],
    ),
  )
}

function elements(html, name) {
  return [...html.matchAll(new RegExp(`<${name}\\b([^>]*)>`, 'gi'))].map((match) =>
    attributes(match[1]),
  )
}

function readPage(route) {
  const path = join(outputDirectory, route, 'index.html')
  assert.ok(existsSync(path), `Missing generated page: ${path}. Run npm run generate first.`)
  return readFileSync(path, 'utf8')
}

function readCatalog(language) {
  return JSON.parse(readFileSync(join(websiteDirectory, 'app', 'i18n', `${language}.json`), 'utf8'))
}

function catalogLeaves(value, prefix = '') {
  if (typeof value === 'string') {
    assert.ok(value.trim(), `Empty translation: ${prefix}`)
    return [prefix]
  }
  assert.ok(value && typeof value === 'object', `Unsupported translation value: ${prefix}`)
  const entries = Object.entries(value)
  assert.ok(entries.length, `Empty translation group: ${prefix}`)
  return entries.flatMap(([key, child]) => catalogLeaves(child, `${prefix}.${key}`))
}

function staticTarget(url) {
  assert.ok(
    url.pathname.startsWith(baseURL) || url.pathname === baseURL.slice(0, -1),
    `Local URL escapes deployment base ${baseURL}: ${url.pathname}`,
  )
  const relativePath = decodeURIComponent(url.pathname.slice(baseURL.length))
  let target = resolve(outputDirectory, relativePath)
  assert.ok(
    target === outputDirectory || target.startsWith(`${outputDirectory}${sep}`),
    `Local URL escapes generated files: ${url.pathname}`,
  )
  assert.ok(existsSync(target), `Broken local URL: ${url.pathname}`)
  if (statSync(target).isDirectory()) target = join(target, 'index.html')
  assert.ok(existsSync(target), `Missing generated route: ${url.pathname}`)
  return target
}

test('all catalogs contain the same nonempty translations and recovery text', () => {
  const expected = catalogLeaves(readCatalog('en')).sort()
  for (const { language } of locales) {
    const catalog = readCatalog(language)
    assert.deepEqual(catalogLeaves(catalog).sort(), expected, `${language} catalog differs`)
    for (const key of ['title', 'description', 'back']) {
      assert.equal(typeof catalog.error?.[key], 'string', `${language} lacks error.${key}`)
      assert.ok(catalog.error[key].trim(), `${language} has empty error.${key}`)
    }
  }
})

test('published bitmap images use WebP throughout every language', () => {
  assert.ok(existsSync(outputDirectory), 'Run npm run generate before checking published images')
  const files = readdirSync(outputDirectory, { recursive: true })
  assert.ok(
    files.some((file) => String(file).endsWith('.webp')),
    'The website needs its WebP images',
  )
  assert.deepEqual(
    files.filter((file) => /\.(png|jpe?g|gif|bmp|avif|tiff?|ico)$/i.test(String(file))),
    [],
  )
  for (const { route } of locales) {
    for (const image of elements(readPage(route), 'img')) {
      assert.match(new URL(image.src, localOrigin).pathname, /\.webp$/i)
    }
  }
})

for (const { language, route } of locales) {
  test(`${language} page has localized content and complete search metadata`, () => {
    const html = readPage(route)
    const catalog = readCatalog(language)
    assert.equal(elements(html, 'html')[0]?.lang, language)
    const title = html.match(/<title>([\s\S]*?)<\/title>/i)?.[1]
    assert.equal(decode(title ?? ''), catalog.meta.title)
    const description = elements(html, 'meta').find((entry) => entry.name === 'description')
    assert.equal(description?.content, catalog.meta.description)
    assert.equal(elements(html, 'h1').length, 1, 'Each page needs one main heading')

    const visibleText = decode(
      html
        .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '')
        .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, '')
        .replace(/<[^>]*>/g, ' '),
    ).replace(/\s+/g, ' ')
    for (const key of ['features', 'workflow', 'download']) {
      assert.ok(visibleText.includes(catalog.nav[key]), `Missing localized navigation: ${key}`)
    }

    const links = elements(html, 'link')
    const canonical = links.filter((entry) => entry.rel === 'canonical')
    assert.equal(canonical.length, 1, 'Each page needs one canonical URL')
    assert.equal(canonical[0].href, new URL(route, siteURL).href)
    for (const locale of locales) {
      const alternate = links.find((entry) => entry.hreflang === locale.language)
      assert.equal(alternate?.rel, 'alternate')
      assert.equal(alternate?.href, new URL(locale.route, siteURL).href)
    }
    const developer = elements(html, 'a').find((entry) =>
      /^https:\/\/vamez\.ru\/?$/.test(entry.href ?? ''),
    )
    assert.ok(developer, 'The developer credit must link to https://vamez.ru')
  })

  test(`${language} page resolves local assets, routes, and fragment links under ${baseURL}`, () => {
    const html = readPage(route)
    const pageURL = new URL(`${baseURL}${route}`, localOrigin)
    let localAssets = 0
    for (const match of html.matchAll(/<([a-z][\w-]*)\b([^>]*)>/gi)) {
      const tag = match[1].toLowerCase()
      const entry = attributes(match[2])
      const references = [entry.src, entry.href].filter(Boolean)
      if (entry.srcset) {
        references.push(
          ...entry.srcset.split(',').map((candidate) => candidate.trim().split(/\s+/)[0]),
        )
      }
      for (const reference of references) {
        const url = new URL(reference, pageURL)
        if (url.origin !== localOrigin) continue
        const target = staticTarget(url)
        if (tag !== 'a') localAssets += 1
        if (url.hash && url.hash !== '#') {
          const targetHTML = readFileSync(target, 'utf8')
          const ids = [...targetHTML.matchAll(/<[a-z][\w-]*\b([^>]*)>/gi)].map(
            (element) => attributes(element[1]).id,
          )
          assert.ok(
            ids.includes(decodeURIComponent(url.hash.slice(1))),
            `Broken fragment link: ${reference}`,
          )
        }
      }
    }
    assert.ok(localAssets > 0, 'The generated page must reference its local assets')
  })
}

const releaseNotes = JSON.parse(
  readFileSync(join(websiteDirectory, 'app', 'release-notes', 'releases.json'), 'utf8'),
).filter((release) => release.items.length > 0)

for (const { language, route } of locales) {
  test(`${language} release history links every authored version`, () => {
    const html = readPage(`${route}releases/`)
    const catalog = readCatalog(language)
    assert.equal(elements(html, 'html')[0]?.lang, language)
    assert.equal(elements(html, 'h1').length, 1)
    assert.ok(decode(html).includes(catalog.releaseNotes.title))
    const links = elements(html, 'a')
    for (const release of releaseNotes) {
      const path = `${baseURL}${route}releases/${release.version}/`
      assert.ok(
        links.some((link) => link.href === path),
        `Missing release link: ${path}`,
      )
      staticTarget(new URL(path, localOrigin))
    }
  })

  test(`${language} version pages preserve translations, publication status, and navigation`, () => {
    const catalog = readCatalog(language)
    for (const release of releaseNotes) {
      const path = `${route}releases/${release.version}/`
      const html = readPage(path)
      const content = decode(html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, ''))
      assert.equal(elements(html, 'html')[0]?.lang, language)
      assert.equal(elements(html, 'h1').length, 1)
      assert.ok(content.includes(catalog.releaseNotes.status[release.status]))
      for (const item of release.items) {
        assert.ok(content.includes(item.text[language]), `Missing ${language} note: ${item.id}`)
      }
      const dates = elements(html, 'time')
      assert.deepEqual(
        dates.map((date) => date.datetime),
        release.publishedAt ? [release.publishedAt] : [],
      )
      const metadata = elements(html, 'link')
      assert.equal(
        metadata.find((link) => link.rel === 'canonical')?.href,
        new URL(path, siteURL).href,
      )
      const links = elements(html, 'a')
      for (const locale of locales) {
        const alternate = `${locale.route}releases/${release.version}/`
        assert.equal(
          metadata.find((link) => link.hreflang === locale.language)?.href,
          new URL(alternate, siteURL).href,
        )
        assert.ok(
          links.some(
            (link) => link.href === `${baseURL}${alternate}` && link.lang === locale.language,
          ),
        )
      }
      const tags = links.filter((link) => link.href?.includes('/releases/tag/'))
      assert.deepEqual(
        tags.map((link) => decodeURIComponent(link.href.split('/releases/tag/')[1])),
        release.tags,
      )
      for (const link of links) {
        const url = new URL(link.href, new URL(`${baseURL}${path}`, localOrigin))
        if (url.origin === localOrigin) staticTarget(url)
      }
    }
  })
}

for (const { language, route } of locales) {
  test(`${language} license page preserves the project license and localized navigation`, () => {
    const html = readPage(`${route}license/`)
    const catalog = readCatalog(language)
    const original = readFileSync(join(websiteDirectory, '..', 'LICENSE'), 'utf8')
    assert.equal(
      decode(html.match(/<title>([\s\S]*?)<\/title>/i)?.[1] ?? ''),
      catalog.license.metaTitle,
    )
    const displayed = html.match(/<pre\b[^>]*>([\s\S]*?)<\/pre>/i)?.[1]
    assert.equal(decode(displayed ?? '').trim(), original.trim())
    assert.equal(elements(html, 'html')[0]?.lang, language)
    assert.equal(elements(html, 'pre')[0]?.lang, 'en')
    assert.equal(elements(html, 'h1').length, 1)
    const links = elements(html, 'link')
    assert.equal(
      links.find((link) => link.rel === 'canonical')?.href,
      new URL(`${route}license/`, siteURL).href,
    )
    const anchors = elements(html, 'a')
    for (const locale of locales) {
      const path = `${locale.route}license/`
      assert.equal(
        links.find((link) => link.hreflang === locale.language)?.href,
        new URL(path, siteURL).href,
      )
      assert.ok(
        anchors.some((link) => link.lang === locale.language && link.href === `${baseURL}${path}`),
      )
    }
    assert.ok(
      elements(readPage(route), 'a').some((link) => link.href === `${baseURL}${route}license/`),
    )
  })
}

for (const { language, route } of locales) {
  test(`${language} module guide documents the package format and protocol`, () => {
    const html = readPage(`${route}modules/`)
    const catalog = readCatalog(language)
    assert.equal(elements(html, 'html')[0]?.lang, language)
    assert.equal(elements(html, 'h1').length, 1)
    assert.ok(decode(html).includes(catalog.modules.title))
    const visibleText = decode(
      html
        .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '')
        .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, '')
        .replace(/<[^>]*>/g, ' '),
    ).replace(/\s+/g, ' ')
    for (const key of ['conceptTitle', 'manifestTitle', 'protocolTitle', 'permissionsTitle']) {
      assert.ok(visibleText.includes(catalog.modules[key]), `Missing module section: ${key}`)
    }
    for (const entry of catalog.modules.manifestFields) {
      assert.ok(visibleText.includes(entry.field), `Missing manifest field: ${entry.field}`)
    }
    const code = decode(html.match(/<pre\b[^>]*>([\s\S]*?)<\/pre>/gi)?.join(' ') ?? '')
    assert.ok(code.includes('"id": "hello-module"'), 'The guide must show a manifest example')
    assert.ok(code.includes('"type":"initialize"'), 'The guide must show protocol examples')
    assert.ok(code.includes('.iwe-module') || code.includes('iwe-module'), 'Guide covers packages')
    const links = elements(html, 'link')
    assert.equal(
      links.find((link) => link.rel === 'canonical')?.href,
      new URL(`${route}modules/`, siteURL).href,
    )
    for (const locale of locales) {
      assert.equal(
        links.find((link) => link.hreflang === locale.language)?.href,
        new URL(`${locale.route}modules/`, siteURL).href,
      )
    }
    for (const anchor of elements(html, 'a')) {
      const url = new URL(anchor.href, new URL(`${baseURL}${route}modules/`, localOrigin))
      if (url.origin === localOrigin) staticTarget(url)
    }
  })
}
