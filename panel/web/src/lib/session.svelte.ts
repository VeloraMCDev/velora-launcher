const KEY = 'scopenet.panel.token';

function read(): string {
  try {
    return localStorage.getItem(KEY) ?? '';
  } catch {
    return '';
  }
}

export interface Me {
  id: number;
  username: string;
  uuid: string;
  role: string;
  groups: string[];
}

export const session = $state<{ token: string; user: Me | null }>({ token: read(), user: null });

export function setToken(token: string) {
  session.token = token;
  try {
    if (token) localStorage.setItem(KEY, token);
    else localStorage.removeItem(KEY);
  } catch {
    /* storage unavailable: session lasts until reload */
  }
}

export function logout() {
  setToken('');
  session.user = null;
}
