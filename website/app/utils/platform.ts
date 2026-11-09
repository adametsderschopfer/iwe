export type DesktopSystem = 'macos' | 'windows' | 'linux'
export type DesktopArchitecture = 'arm64' | 'x64' | 'x86'

export interface PlatformInput {
  userAgent: string
  platform?: string
  maxTouchPoints?: number
  hints?: {
    platform?: string
    architecture?: string
    bitness?: string
    mobile?: boolean
  }
}

export interface DetectedPlatform {
  system: DesktopSystem | undefined
  architecture: DesktopArchitecture | undefined
}

function legacySystem(value: string): DesktopSystem | undefined {
  if (/windows|\bwin(?:32|64)\b/i.test(value)) return 'windows'
  if (/macintosh|macintel|macppc|mac os/i.test(value)) return 'macos'
  if (/linux/i.test(value)) return 'linux'
  return undefined
}

function hintedSystem(value: string): DesktopSystem | undefined {
  if (value === 'windows') return 'windows'
  if (value === 'macos') return 'macos'
  if (value === 'linux') return 'linux'
  return undefined
}

function hintedArchitecture(
  architecture: string,
  bitness: string | undefined,
): DesktopArchitecture | undefined {
  if (/^(arm64|aarch64)$/.test(architecture)) {
    return bitness === '32' ? undefined : 'arm64'
  }
  if (architecture === 'arm') return bitness === '64' ? 'arm64' : undefined
  if (/^(x64|x86_64|amd64)$/.test(architecture)) {
    return bitness === '32' ? undefined : 'x64'
  }
  if (architecture === 'x86') {
    if (bitness === '64') return 'x64'
    if (bitness === '32') return 'x86'
  }
  return undefined
}

export function detectPlatform(input: PlatformInput): DetectedPlatform {
  const legacy = `${input.userAgent} ${input.platform ?? ''}`
  const unsupported = { system: undefined, architecture: undefined }
  const isTouchMac = /macintel/i.test(input.platform ?? '') && (input.maxTouchPoints ?? 0) > 1
  if (
    input.hints?.mobile ||
    isTouchMac ||
    /android|iphone|ipad|ipod|\bmobile\b|\btablet\b|\bcros\b|chrome\s?os/i.test(legacy)
  ) {
    return unsupported
  }

  const platformHint = input.hints?.platform?.trim().toLowerCase()
  const userAgentSystem = legacySystem(input.userAgent)
  const navigatorSystem = legacySystem(input.platform ?? '')
  const system = platformHint ? hintedSystem(platformHint) : (userAgentSystem ?? navigatorSystem)
  if (!system) return unsupported

  const architectureHint = input.hints?.architecture?.trim().toLowerCase()
  if (architectureHint) {
    return { system, architecture: hintedArchitecture(architectureHint, input.hints?.bitness) }
  }

  // Mac browser identifiers do not establish the processor architecture.
  if (system === 'macos') {
    return { system, architecture: undefined }
  }

  const architectureSource = [
    userAgentSystem === system ? input.userAgent : '',
    navigatorSystem === system ? input.platform : '',
  ].join(' ')
  let architecture: DesktopArchitecture | undefined
  if (/\baarch64\b|\barm64\b/i.test(architectureSource)) architecture = 'arm64'
  else if (/\bwin64\b|\bwow64\b|\bx64\b|\bx86_64\b|\bamd64\b/i.test(architectureSource)) {
    architecture = 'x64'
  } else if (/\bi[3-6]86\b/i.test(architectureSource)) architecture = 'x86'

  return { system, architecture }
}
