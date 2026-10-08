// True when the panel runs as an app: inside the Android/iOS shell (Capacitor), opened with ?app=1, or installed to the home
// screen. In that mode there is no landing page: the app opens to the sign-in screen, then the player panel.
function detect(): boolean {
  try {
    const w = window as any;
    if (w.Capacitor?.isNativePlatform?.()) return true;
    if (new URLSearchParams(location.search).has('app')) localStorage.setItem('scopenet.native', '1');
    if (localStorage.getItem('scopenet.native') === '1') return true;
    return !!(matchMedia('(display-mode: standalone)').matches || (navigator as any).standalone);
  } catch {
    return false;
  }
}

export const nativeApp = detect();
if (nativeApp) document.documentElement.classList.add('native-app');
