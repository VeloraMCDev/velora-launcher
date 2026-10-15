export function settingsTabIndex(key: string, current: number, length: number): number | null {
  if (!Number.isInteger(length) || length <= 0 || !Number.isInteger(current) || current < 0 || current >= length) return null;
  if (key === 'Home') return 0;
  if (key === 'End') return length - 1;
  if (key === 'ArrowDown' || key === 'ArrowRight') return (current + 1) % length;
  if (key === 'ArrowUp' || key === 'ArrowLeft') return (current + length - 1) % length;
  return null;
}
