export class ApiError extends Error { status: number; constructor(message: string, status: number); }
export class ApiVersionError extends Error { actual: unknown; constructor(actual: unknown); }
export interface ApiManifest { api_version: 1; [key: string]: unknown; }
export function assertApiVersion<T extends {api_version: number}>(manifest: T): T;
export interface Configuration {
  /** Optional HTTP(S) origin/prefix. Omit for browser same-origin requests. */
  baseUrl?: string;
  fetch?: typeof globalThis.fetch;
  getToken?: () => string | null | undefined;
  getInstance?: () => string | null | undefined;
  onUnauthorized?: () => void;
  /** 30 seconds by default; zero explicitly retains legacy unlimited requests. */
  timeoutMs?: number;
  instanceHeader?: 'X-SCOPENET-Instance' | 'X-Velora-Instance';
}
export interface Options { method?: string; body?: unknown; form?: FormData; signal?: AbortSignal; }
export interface HttpClient {
  api<T = unknown>(path: string, options?: Options): Promise<T>;
  get<T = unknown>(path: string): Promise<T>;
  post<T = unknown>(path: string, body?: unknown): Promise<T>;
  put<T = unknown>(path: string, body: unknown): Promise<T>;
  patch<T = unknown>(path: string, body: unknown): Promise<T>;
  del<T = unknown>(path: string): Promise<T>;
  manifest<T extends ApiManifest = ApiManifest>(): Promise<T>;
}
export function createHttpClient(configuration?: Configuration): HttpClient;
