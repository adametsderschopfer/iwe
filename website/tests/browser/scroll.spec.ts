import { expect, test, type Locator, type Page } from '@playwright/test'

async function openWebsite(page: Page) {
  await page.goto('./')
  await page.waitForLoadState('networkidle')
}

async function wheelTo(page: Page, target: Locator) {
  const bounds = await target.boundingBox()
  expect(bounds).not.toBeNull()
  await page.mouse.wheel(0, Math.round(bounds!.y - 150))
  await expect.poll(async () => (await target.boundingBox())?.y ?? Infinity).toBeLessThan(200)
}

async function imageIsExposed(image: Locator) {
  return image.evaluate((element) => {
    const imageElement = element as HTMLImageElement
    if (!imageElement.complete || !imageElement.naturalWidth) return false
    const rect = element.getBoundingClientRect()
    const left = Math.max(0, rect.left)
    const top = Math.max(0, rect.top)
    const right = Math.min(innerWidth, rect.right)
    const bottom = Math.min(innerHeight, rect.bottom)
    if (right <= left || bottom <= top) return false
    return document.elementFromPoint((left + right) / 2, (top + bottom) / 2) === element
  })
}

test('wipe image becomes exposed after wheel scrolling from below the fold', async ({ page }) => {
  await openWebsite(page)
  const wipe = page.locator('.reveal-wipe')
  const image = wipe.locator('img')
  await expect(wipe).toHaveAttribute('data-reveal-state', 'pending')
  expect((await wipe.boundingBox())!.y).toBeGreaterThan(800)

  await wheelTo(page, wipe)

  await expect(wipe).toHaveAttribute('data-reveal-state', 'visible')
  await expect.poll(() => imageIsExposed(image)).toBe(true)
})

test('Workflow anchor reveals the comparison without manual intervention', async ({ page }) => {
  await openWebsite(page)
  await page.getByRole('link', { name: 'Workflow', exact: true }).click()
  await expect(page).toHaveURL(/#workflow$/)
  const wipe = page.locator('.reveal-wipe')
  await expect(wipe).toHaveAttribute('data-reveal-state', 'visible')
  await expect.poll(() => imageIsExposed(wipe.locator('img'))).toBe(true)
})

test('rise and wipe content remain revealed after scrolling back', async ({ page }) => {
  await openWebsite(page)
  await expect(page.locator('.reveal-rise[data-reveal-state="pending"]').first()).toBeAttached()
  await expect(page.locator('.reveal-wipe[data-reveal-state="pending"]')).toBeAttached()

  for (let step = 0; step < 20; step += 1) {
    const position = await page.evaluate(() => ({
      y: scrollY,
      end: document.documentElement.scrollHeight - innerHeight,
    }))
    if (position.y >= position.end - 1) break
    await page.mouse.wheel(0, 600)
    await expect.poll(() => page.evaluate(() => scrollY)).toBeGreaterThan(position.y)
  }
  await expect(page.locator('.scroll-reveal[data-reveal-state="pending"]')).toHaveCount(0)
  await page.mouse.wheel(0, -100_000)
  await expect.poll(() => page.evaluate(() => scrollY)).toBe(0)
  await expect(page.locator('.scroll-reveal[data-reveal-state="pending"]')).toHaveCount(0)
})

test('reduced motion keeps all content visible without scrolling', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' })
  await openWebsite(page)
  await expect(page.locator('.scroll-reveal[data-reveal-state="pending"]')).toHaveCount(0)
  const wipe = page.locator('.reveal-wipe')
  await wheelTo(page, wipe)
  await expect.poll(() => imageIsExposed(wipe.locator('img'))).toBe(true)
})

test('keyboard navigation exposes a pending download card immediately', async ({ page }) => {
  await openWebsite(page)
  await expect(page.locator('.platform[data-reveal-state="pending"]').first()).toBeAttached()
  await page.locator('.hero-actions a').last().focus()
  for (let step = 0; step < 12; step += 1) {
    await page.keyboard.press('Tab')
    if (await page.locator('.platform a:focus').count()) break
  }
  const focusedCard = page.locator('.platform:has(a:focus)')
  await expect(focusedCard).toHaveCount(1)
  await expect(focusedCard).toHaveAttribute('data-reveal-state', 'visible')
  await expect(focusedCard).toHaveClass(/reveal-immediate/)
  await expect(focusedCard).toHaveCSS('opacity', '1')
})

test.describe('without JavaScript', () => {
  test.use({ javaScriptEnabled: false })

  test('server-rendered content and comparison remain visible', async ({ page }) => {
    await openWebsite(page)
    await expect(page.locator('.scroll-reveal[data-reveal-state="pending"]')).toHaveCount(0)
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
    const wipe = page.locator('.reveal-wipe')
    await wheelTo(page, wipe)
    await expect.poll(() => imageIsExposed(wipe.locator('img'))).toBe(true)
  })
})
