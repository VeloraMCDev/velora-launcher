export interface Container {
  id: string;
  name: string;
  image: string;
  state: string;
  status: string;
  health: string | null;
  project: string | null;
  service: string | null;
  ports: string[];
  stats?: { cpu: number; memory: number; memoryLimit: number } | null;
}
export interface Endpoint { name: string; url: string; ok: boolean; status: number; ms: number | null; error?: string }
export interface Certificate { host: string; expires?: string; issuer?: string; error?: string }
export interface BackupStatus {
  status: 'ok' | 'failed';
  message: string;
  started_at: string;
  finished_at: string;
  archive: string;
  offsite: boolean;
  bytes?: number;
  sha256?: string;
}
export interface Snapshot {
  at: string;
  host: {
    cpu: number | null;
    load: number[];
    cores: number;
    memory: { total: number; used: number; percent: number };
    disk: { total: number; used: number; percent: number } | null;
    uptime: number;
  };
  containers: Container[];
  endpoints: Endpoint[];
  backups: { last: BackupStatus | null; archives: { name: string; bytes: number; at: string }[]; pending: boolean };
  certificates: Certificate[];
}
export interface Alert { key: string; event: string; severity: 'critical' | 'warning'; title: string; detail?: string; since: string; notified: boolean }
export interface Job { id: string; kind: string; target: string | null; user: string; status: 'running' | 'succeeded' | 'failed'; started: string; finished: string | null; lines?: number; log?: string[] }
export interface Overview {
  snapshot: Snapshot | null;
  alerts: Alert[];
  jobs: Job[];
  project: string;
  updates: { checked: string | null; launcher_pending: number; images: Record<string, boolean>; containers: number };
}
export interface HistoryPoint { t: string; cpu: number | null; mem: number; load: number; panel_ms: number | null; panel_cpu: number | null; panel_mem: number | null }
export interface Release { tag: string; version: string; name: string; published_at: string; prerelease: boolean; html_url: string; assets: { name: string; size: number }[]; has_manifest: boolean; approved: boolean }
export interface ImageRelease { tag: string; commit: string; run: number; digest: string; image: string; commitInfo?: { message?: string; date?: string; url?: string } }
export interface ContainerUpdate { name: string; project: string | null; service: string | null; image: string; status: 'current' | 'update-available' | 'pinned' | 'local-build' | 'unknown'; error?: string }
export interface Updates {
  checked: string | null;
  errors?: string[];
  launcher?: { releases: Release[] };
  launcher_served?: string | null;
  images?: Record<string, { service: string; current: string | null; running: string | null; releases: ImageRelease[] }>;
  containers?: ContainerUpdate[];
}
export interface PanelSummary {
  version: string;
  users: number;
  admins: number;
  logins_24h: number;
  logins_7d: number;
  unique_logins_7d: number;
  launcher_sign_ins_24h: number;
  launches_24h: number;
  admin_changes_7d: number;
  recent_logins: { name: string | null; created_at: string }[];
  recent_admin_changes: { name: string | null; detail: string | null; created_at: string }[];
}
export interface ActivityEntry { id: number; source: string; server: string | null; name: string | null; kind: string; detail: string | null; created_at: string }
export interface AuditEntry { at: string; user: string; action: string; target?: string; result?: string; ip?: string; job?: string }
export interface Settings {
  notifications: { enabled: boolean; resend_api_key: string; resend_api_key_set: boolean; from: string; recipients: string[]; events: Record<string, boolean> };
  github_token_set: boolean;
  alerts: { disk_percent: number; backup_max_age_hours: number; certificate_days: number };
}
