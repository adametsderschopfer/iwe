import { expect, test } from '@playwright/test'

for (const language of ['', 'ru/', 'es/']) {
  for (const width of [390, 1280]) {
    test(`screenshot gallery stays complete and usable: ${language || 'en'} at ${width}px`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height: 900 })
      await page.emulateMedia({ reducedMotion: 'reduce' })
      await page.goto(`./${language}`)
      const buttons = page.locator('.preview-options button')
      await expect(buttons).toHaveCount(5)
      for (const button of await buttons.all()) {
        await button.click()
        await expect(button).toHaveAttribute('aria-pressed', 'true')
        const image = page.locator('#product-preview img:visible')
        await expect(image).toHaveCount(1)
        await expect(image).toHaveCSS('object-fit', 'contain')
        await expect
          .poll(() =>
            image.evaluate((element) => {
              const screenshot = element as HTMLImageElement
              return screenshot.complete && screenshot.naturalWidth > 0
            }),
          )
          .toBe(true)
        const bounds = await image.boundingBox()
        expect(bounds!.x).toBeGreaterThanOrEqual(0)
        expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(width)
      }
      await expect(page.locator('#features article')).toHaveCount(6)
      await expect(page.locator('.deployment')).toHaveCount(0)
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
        width,
      )
      // Keep local review captures with the test artifacts, outside published assets.
      await page.locator('#features').screenshot({
        path: test.info().outputPath('features.png'),
      })
      await page.locator('.hero').screenshot({
        path: test.info().outputPath('editor-overview.png'),
      })
    })
  }
}
