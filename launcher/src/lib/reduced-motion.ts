export function prefersReducedMotion(): boolean {
  if (typeof document !== 'undefined' && document.documentElement?.classList?.contains('reduce-motion')) {
    return true;
  }
  return typeof window !== 'undefined'
    && typeof window.matchMedia === 'function'
    && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}
