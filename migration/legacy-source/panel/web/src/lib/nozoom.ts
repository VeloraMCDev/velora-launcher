// Makes the panel behave like an app on touch devices: no pinch zoom, no double-tap zoom, no rubber-band page bounce.
// The viewport meta tag covers Android and most browsers; iOS Safari ignores `user-scalable=no`, so the gesture events are
// cancelled here as well. Scrolling and normal taps are untouched.
const opts = { passive: false } as const;

function install() {
  // iOS Safari pinch gestures.
  for (const type of ['gesturestart', 'gesturechange', 'gestureend']) document.addEventListener(type, (e) => e.preventDefault(), opts);

  // Any two-finger touch is a pinch, never a scroll.
  document.addEventListener('touchmove', (e) => { if (e.touches.length > 1) e.preventDefault(); }, opts);

  // A second tap within 300ms is a double-tap zoom: swallow it (and turn it into a plain click so buttons still respond).
  let last = 0;
  document.addEventListener('touchend', (e) => {
    const now = Date.now();
    if (now - last < 300 && e.touches.length === 0 && e.cancelable) {
      const t = e.target as HTMLElement | null;
      if (!t?.closest('input, textarea, select, [contenteditable]')) { e.preventDefault(); t?.click?.(); }
    }
    last = now;
  }, opts);

  // Ctrl/Cmd + wheel or +/-/0 zooms the page on desktop browsers.
  window.addEventListener('wheel', (e) => { if (e.ctrlKey) e.preventDefault(); }, opts);
  window.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && ['+', '-', '=', '0'].includes(e.key)) e.preventDefault();
  });
}

// Only touch devices and installed apps: desktop users keep browser zoom for accessibility.
const touch = typeof matchMedia === 'function' && matchMedia('(pointer: coarse)').matches;
if (touch) {
  document.documentElement.classList.add('touch');
  install();
}
