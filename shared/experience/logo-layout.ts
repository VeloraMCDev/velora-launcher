import type { Experience } from './index';
export interface LogoLayout { width: number; max_height: number; title_size: number }
export const DEFAULT_LOGO_LAYOUT: LogoLayout = { width: 420, max_height: 140, title_size: 44 };
export function logoLayout(experience?: Experience): LogoLayout {
  const raw = experience?.modules.launcher_presentation as Partial<LogoLayout> | undefined;
  const bounded = (value: unknown, fallback: number, min: number, max: number) => typeof value === 'number' && Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;
  return {
    width: bounded(raw?.width, DEFAULT_LOGO_LAYOUT.width, 120, 1200),
    max_height: bounded(raw?.max_height, DEFAULT_LOGO_LAYOUT.max_height, 40, 400),
    title_size: bounded(raw?.title_size, DEFAULT_LOGO_LAYOUT.title_size, 24, 96),
  };
}
