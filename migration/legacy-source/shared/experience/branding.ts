/** Shared branding contract for platform defaults and instance identities. */
export interface NewsItem { id: string; title: string; body: string; image_url: string | null; link: string | null; date: string | null; pinned: boolean; tag: string | null }
export interface SocialLink { label: string; url: string; icon: string }

export interface Branding {
  name: string;
  tagline: string;
  logo_url: string | null;
  icon_url: string | null;
  background: { kind: 'gradient' | 'image' | 'video'; url: string | null; blur: number; dim: number };
  colors: { accent: string; accent_2: string; background: string; surface: string; text: string; muted: string; success: string; danger: string };
  font: string;
  radius: number;
  glass: boolean;
  version_label: string;
  news: NewsItem[];
  links: SocialLink[];
  about: { title: string; icon_url: string | null; body: string; links: SocialLink[] };
  features: { news: boolean; server_status: boolean; allow_user_theme: boolean; allow_advanced_java: boolean };
  custom_css: string;
}
