/**
 * 平台环境检测与多平台兼容适配层
 * 支持 Tauri 原生环境（Android / Linux / Windows / macOS）与纯 Web 浏览器环境自适应切换
 */

export type PlatformType = 'tauri' | 'web' | 'android' | 'ios' | 'windows' | 'macos' | 'linux';

/**
 * 判断当前是否运行在 Tauri 宿主环境中
 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__);
}

/**
 * 判断当前是否运行在普通浏览器 (Web) 环境中
 */
export function isWeb(): boolean {
  return !isTauri();
}

/**
 * 获取当前运行时具体平台类型
 */
export function getPlatformType(): PlatformType {
  if (!isTauri()) {
    return 'web';
  }
  const userAgent = navigator.userAgent.toLowerCase();
  if (/android/.test(userAgent)) return 'android';
  if (/iphone|ipad|ipod/.test(userAgent)) return 'ios';
  if (/windows/.test(userAgent)) return 'windows';
  if (/macintosh|mac os x/.test(userAgent)) return 'macos';
  if (/linux/.test(userAgent)) return 'linux';
  return 'tauri';
}

