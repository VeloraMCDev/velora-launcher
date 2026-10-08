import type {Configuration, HttpClient} from './client.mjs';

export interface RequestOptions {signal?: AbortSignal;}
export type SkinModel = 'classic' | 'slim';
export interface PublicUser {id: number; username: string; uuid: string; role: string; groups: string[];}
export interface LoginRequest {username: string; password: string;}
export interface RegisterRequest extends LoginRequest {email?: string | null;}
export interface AuthResponse {token: string; user: PublicUser; pending: boolean; yggdrasil?: {access_token: string; client_token: string} | null;}
export interface AuthConfig {panel_accounts: boolean; registration: 'closed' | 'open' | 'approval'; offline_local: boolean; yggdrasil_url: string | null;}
export interface SocialLink {label: string; url: string; icon: string;}
export interface Branding {
  name: string; tagline: string; logo_url: string | null; icon_url: string | null;
  background: {kind: string; url: string | null; blur: number; dim: number};
  colors: {accent: string; accent_2: string; background: string; surface: string; text: string; muted: string; success: string; danger: string};
  font: string; radius: number; glass: boolean; version_label: string;
  news: {id: string; title: string; body: string; image_url: string | null; link: string | null; date: string | null; pinned: boolean; tag: string | null}[];
  links: SocialLink[]; about: {title: string; icon_url: string | null; body: string; links: SocialLink[]};
  features: {news: boolean; server_status: boolean; allow_user_theme: boolean; allow_advanced_java: boolean}; custom_css: string;
}
/** Declarative local extension metadata; module values are opaque to the platform. */
export interface Experience {
  kind: string; branding: Branding | null; features: string[];
  navigation: {id: string; label: string}[];
  widgets: {id: string; component: string; title: string; body: string; config: unknown}[];
  modules: Record<string, unknown>;
}
export interface InstanceSummary {
  experience: Experience; id: string; name: string; description: string;
  icon_url: string | null; banner_url: string | null; logo_url: string | null;
  mc_version: string; loader: 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge'; loader_version: string | null;
  revision: number; clean_epoch: number;
  server: {name: string; address: string; port: number; auto_join: boolean; inject: boolean} | null;
  memory: {min_mb: number; max_mb: number}; jvm_args: string; featured: boolean;
  source_label: string; file_count: number; total_size: number;
}
export interface FileEntry {path: string; url: string; sha1: string; size: number;}
export interface InstanceManifest {instance: InstanceSummary; files: FileEntry[];}
export interface LauncherManifest {api_version: 1; panel_version: string; branding: Branding; auth: AuthConfig; instances: InstanceSummary[]; user: PublicUser | null;}
export interface LauncherUpdate {version: string; notes: string; url: string; size: number; sha256?: string | null;}
export interface LaunchEvent {instance_id: string; kind: string; username?: string | null;}
export interface CapeInfo {id: number; name: string; url: string;}
export interface PlayerProfile {uuid: string; name: string; skin_url: string | null; skin_model: SkinModel; cape: CapeInfo | null; available_capes: CapeInfo[];}
export interface OkResponse {ok: true;}
export interface ForgotPasswordResponse extends OkResponse {message: string;}
export interface PlatformClient {
  transport: HttpClient;
  launcher: {
    manifest(options?: RequestOptions): Promise<LauncherManifest>;
    instanceManifest(id: string, options?: RequestOptions): Promise<InstanceManifest>;
    event(body: LaunchEvent, options?: RequestOptions): Promise<OkResponse>;
    latestUpdate(options?: RequestOptions): Promise<LauncherUpdate | null>;
  };
  auth: {
    login(body: LoginRequest, options?: RequestOptions): Promise<AuthResponse>;
    register(body: RegisterRequest, options?: RequestOptions): Promise<AuthResponse>;
    me(options?: RequestOptions): Promise<PublicUser>;
    forgotPassword(email: string, options?: RequestOptions): Promise<ForgotPasswordResponse>;
    resetPassword(token: string, password: string, options?: RequestOptions): Promise<OkResponse>;
  };
  account: {
    profile(options?: RequestOptions): Promise<PlayerProfile>;
    setUsername(username: string, password: string, options?: RequestOptions): Promise<AuthResponse>;
    uploadSkin(file: Blob, model?: SkinModel, options?: RequestOptions): Promise<PlayerProfile>;
    deleteSkin(options?: RequestOptions): Promise<PlayerProfile>;
    setSkinModel(model: SkinModel, options?: RequestOptions): Promise<PlayerProfile>;
    setCape(cape_id: number | null, options?: RequestOptions): Promise<PlayerProfile>;
  };
}
export function createPlatformClient(configuration?: Configuration): PlatformClient;
