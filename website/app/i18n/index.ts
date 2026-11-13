import en from './en.json'
import ru from './ru.json'
import es from './es.json'

export type Locale = 'en' | 'ru' | 'es'
export type Messages = typeof en

export const catalogs: Record<Locale, Messages> = { en, ru, es }
export const languages = [
  { code: 'en', label: 'EN', name: 'English', path: '/' },
  { code: 'ru', label: 'RU', name: 'Русский', path: '/ru/' },
  { code: 'es', label: 'ES', name: 'Español', path: '/es/' },
] as const

export const repository = 'https://github.com/adametsderschopfer/iwe-code'
export const releases = `${repository}/releases/latest`

export const licensePath = (locale: Locale) => `${locale === 'en' ? '' : `/${locale}`}/license/`

export const modulesPath = (locale: Locale) => `${locale === 'en' ? '' : `/${locale}`}/modules/`
