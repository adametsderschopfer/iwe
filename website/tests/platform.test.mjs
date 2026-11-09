import assert from 'node:assert/strict'
import test from 'node:test'
import { detectPlatform } from '../app/utils/platform.ts'

const windowsAgent =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/143.0.0.0 Safari/537.36'
const macAgent =
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Version/26.0 Safari/605.1.15'
const linuxAgent =
  'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/143.0.0.0 Safari/537.36'
const unsupported = { system: undefined, architecture: undefined }

test('recognizes explicit Windows architecture without trusting Win32', () => {
  assert.deepEqual(detectPlatform({ userAgent: windowsAgent, platform: 'Win32' }), {
    system: 'windows',
    architecture: 'x64',
  })
  assert.deepEqual(
    detectPlatform({ userAgent: 'Mozilla/5.0 (Windows NT 10.0)', platform: 'Win32' }),
    {
      system: 'windows',
      architecture: undefined,
    },
  )
  assert.deepEqual(detectPlatform({ userAgent: 'Mozilla/5.0 (Windows NT 6.1; i686)' }), {
    system: 'windows',
    architecture: 'x86',
  })
})

test('recognizes Linux architecture while leaving omitted details unknown', () => {
  assert.deepEqual(detectPlatform({ userAgent: linuxAgent }), {
    system: 'linux',
    architecture: 'x64',
  })
  assert.deepEqual(detectPlatform({ userAgent: 'Mozilla/5.0 (X11; Linux i686)' }), {
    system: 'linux',
    architecture: 'x86',
  })
  assert.deepEqual(detectPlatform({ userAgent: 'Mozilla/5.0 (X11; Linux aarch64)' }), {
    system: 'linux',
    architecture: 'arm64',
  })
  assert.deepEqual(detectPlatform({ userAgent: 'Mozilla/5.0 (X11; Linux)' }), {
    system: 'linux',
    architecture: undefined,
  })
})

test('does not mistake the frozen Mac Intel identifiers for an Intel processor', () => {
  assert.deepEqual(detectPlatform({ userAgent: macAgent, platform: 'MacIntel' }), {
    system: 'macos',
    architecture: undefined,
  })
  assert.deepEqual(
    detectPlatform({ userAgent: macAgent, platform: 'MacIntel', maxTouchPoints: 1 }),
    {
      system: 'macos',
      architecture: undefined,
    },
  )
})

test('uses explicit hints for Apple Silicon and Intel Macs', () => {
  for (const [architecture, expected] of [
    ['arm', 'arm64'],
    ['x86', 'x64'],
  ]) {
    assert.deepEqual(
      detectPlatform({
        userAgent: macAgent,
        platform: 'MacIntel',
        hints: { platform: 'macOS', architecture, bitness: '64', mobile: false },
      }),
      { system: 'macos', architecture: expected },
    )
  }
})

test('prefers architecture hints to a reduced or emulated Windows user agent', () => {
  assert.deepEqual(
    detectPlatform({
      userAgent: windowsAgent,
      platform: 'Win32',
      hints: { platform: 'Windows', architecture: 'arm', bitness: '64' },
    }),
    { system: 'windows', architecture: 'arm64' },
  )
  assert.deepEqual(
    detectPlatform({
      userAgent: windowsAgent,
      hints: { platform: 'Windows', architecture: 'x86', bitness: '32' },
    }),
    { system: 'windows', architecture: 'x86' },
  )
})

test('keeps architecture unknown when hints hide bitness or specify an unsupported processor', () => {
  for (const hints of [
    { architecture: 'x86' },
    { architecture: 'arm', bitness: '32' },
    { architecture: 'mips', bitness: '64' },
  ]) {
    assert.deepEqual(detectPlatform({ userAgent: windowsAgent, hints }), {
      system: 'windows',
      architecture: undefined,
    })
  }
})

test('uses platform hints without borrowing architecture from a contradictory user agent', () => {
  assert.deepEqual(detectPlatform({ userAgent: linuxAgent, hints: { platform: 'Windows' } }), {
    system: 'windows',
    architecture: undefined,
  })
  assert.deepEqual(
    detectPlatform({
      userAgent: windowsAgent,
      hints: { platform: 'Linux', architecture: 'arm', bitness: '64' },
    }),
    { system: 'linux', architecture: 'arm64' },
  )
  assert.deepEqual(
    detectPlatform({ userAgent: linuxAgent, hints: { platform: 'Chrome OS' } }),
    unsupported,
  )
  assert.deepEqual(
    detectPlatform({
      userAgent: 'Mozilla/5.0 (Windows NT 10.0)',
      platform: 'Linux x86_64',
    }),
    { system: 'windows', architecture: undefined },
  )
  assert.deepEqual(
    detectPlatform({ userAgent: linuxAgent, platform: 'Win32', hints: { platform: 'Windows' } }),
    { system: 'windows', architecture: undefined },
  )
})

test('falls back safely when optional hints are absent or empty', () => {
  assert.deepEqual(
    detectPlatform({ userAgent: windowsAgent, hints: { platform: '', architecture: '' } }),
    { system: 'windows', architecture: 'x64' },
  )
  assert.deepEqual(detectPlatform({ userAgent: '', platform: 'Linux x86_64' }), {
    system: 'linux',
    architecture: 'x64',
  })
  assert.deepEqual(detectPlatform({ userAgent: '', platform: 'Win32' }), {
    system: 'windows',
    architecture: undefined,
  })
  assert.deepEqual(detectPlatform({ userAgent: '' }), unsupported)
})

test('does not offer desktop packages to mobile devices, iPad desktop mode, or ChromeOS', () => {
  for (const input of [
    {
      userAgent:
        'Mozilla/5.0 (Linux; Android 16; Pixel 9) AppleWebKit/537.36 Chrome/143.0.0.0 Mobile Safari/537.36',
      platform: 'Linux aarch64',
    },
    {
      userAgent:
        'Mozilla/5.0 (iPhone; CPU iPhone OS 26_0 like Mac OS X) Mobile/15E148 Safari/604.1',
    },
    { userAgent: 'Mozilla/5.0 (iPad; CPU OS 26_0 like Mac OS X) Mobile/15E148 Safari/604.1' },
    { userAgent: macAgent, platform: 'MacIntel', maxTouchPoints: 5 },
    { userAgent: windowsAgent, hints: { platform: 'Windows', mobile: true } },
    {
      userAgent:
        'Mozilla/5.0 (X11; CrOS x86_64 16371.40.0) AppleWebKit/537.36 Chrome/143.0.0.0 Safari/537.36',
    },
    { userAgent: windowsAgent, hints: { platform: 'Android', mobile: false } },
  ]) {
    assert.deepEqual(detectPlatform(input), unsupported)
  }
})
