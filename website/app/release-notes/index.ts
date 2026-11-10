import data from './releases.json'
import type { Locale } from '~/i18n'

export type Category = 'added' | 'changed' | 'fixed' | 'maintenance'
export interface ReleaseItem {
  id: string
  category: Category
  text: Record<Locale, string>
  sources: string[]
}
export interface ReleaseNote {
  version: string
  status: 'released' | 'development' | 'prepared'
  publishedAt: string | null
  tags: string[]
  source: string | null
  items: ReleaseItem[]
}

// The release preflight validates these authored records before publication.
export const releaseNotes = (data as ReleaseNote[])
  .filter((release) => release.items.length > 0)
  .sort((left, right) => {
    if (left.version === 'unreleased') return -1
    if (right.version === 'unreleased') return 1
    return right.version.localeCompare(left.version, 'en', { numeric: true })
  })
export const categories: Category[] = ['added', 'changed', 'fixed', 'maintenance']
export const releaseNotesPath = (locale: Locale, version?: string) =>
  `${locale === 'en' ? '' : `/${locale}`}/releases/${version ? `${version}/` : ''}`
export const isReleaseVersion = (version: unknown) =>
  version === undefined ||
  version === '' ||
  releaseNotes.some((release) => release.version === version)
