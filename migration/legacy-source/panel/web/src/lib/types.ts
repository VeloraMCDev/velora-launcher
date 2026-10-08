import type { Branding } from '@scopenet/experience/branding';
export type { Branding, NewsItem, SocialLink } from '@scopenet/experience/branding';
import type { Experience } from '@scopenet/experience';
export interface MemoryDefaults { min_mb: number; max_mb: number }
export interface ServerEntry { name: string; address: string; port: number; auto_join: boolean; inject: boolean }

export interface Instance {
  experience?: Experience;
  id: string;
  name: string;
  description: string;
  icon_url: string | null;
  banner_url: string | null;
  logo_url: string | null;
  mc_version: string;
  loader: string;
  loader_version: string | null;
  source_kind: string;
  source_label: string;
  source_ref: any;
  visibility: 'public' | 'members' | 'groups';
  allowed_groups: string[];
  memory: MemoryDefaults;
  jvm_args: string;
  server: ServerEntry | null;
  featured: boolean;
  enabled: boolean;
  sort: number;
  revision: number;
  clean_epoch?: number;
  created_at: string;
  updated_at: string;
  file_count: number;
  total_size: number;
  missing_count: number;
}

export interface FileRow { path: string; url: string; sha1: string; size: number; origin: string; note: string | null }

export interface User {
  id: number;
  username: string;
  email: string | null;
  role: 'admin' | 'player';
  status: 'active' | 'pending' | 'disabled';
  created_at: string;
  last_login: string | null;
  uuid: string;
  groups: string[];
  skin_url: string | null;
  skin_model: 'classic' | 'slim';
  cape_id: number | null;
  status_reason: string | null;
  playtime_secs: number;
  last_seen_ingame: string | null;
}

export interface Group { id: number; name: string; color: string; members: number; luckperms_group?: string; discord_role?: string }

export interface Settings {
  auth: {
    panel_accounts: boolean;
    registration: 'closed' | 'open' | 'approval';
    offline_local: boolean;
  };
  public_url: string | null;
  curseforge_api_key?: string | null;
  launcher_download_url: string | null;
  username_blocklist: string[];
  curseforge_key_set: boolean;
  curseforge_key_from_env: boolean;
}

export interface Cape { id: number; name: string; url: string; visibility: 'public' | 'groups' | 'private'; allowed_groups: string[]; wearers: number; created_at: string }

export interface AuthServerInfo { public_url: string; yggdrasil_url: string; public_key: string }

export interface GameServer {
  instance_id: string;
  map_enabled: boolean;
  economy_group: string;
  id: number;
  name: string;
  token_hint: string;
  access: 'all' | 'members' | 'groups';
  allowed_groups: string[];
  require_launcher: boolean;
  software: string | null;
  mc_version: string | null;
  plugin_version: string | null;
  online_mode: boolean | null;
  max_players: number;
  online_count: number;
  tps: number | null;
  last_seen: string | null;
  created_at: string;
  online: boolean;
  players: number;
}

export interface PlayerStats {
  server_id: number;
  uuid: string;
  name: string;
  playtime_secs: number;
  joins: number;
  deaths: number;
  player_kills: number;
  mob_kills: number;
  blocks_broken: number;
  blocks_placed: number;
  messages: number;
  first_seen: string;
  last_seen: string;
  server_name?: string;
}

export interface ServerEvent { id: number; server_id: number; uuid: string | null; name: string | null; kind: string; detail: string | null; created_at: string }

export interface ServerDetail {
  server: GameServer;
  online_players: { uuid: string; name: string; joined_at: string }[];
  leaderboard: PlayerStats[];
  events: ServerEvent[];
  totals: { players: number; playtime_secs: number };
}

export interface UserActivity { servers: PlayerStats[]; events: ServerEvent[]; online_on: { id: number; name: string }[] }

/** What the admin map card shows: tiles per dimension plus when the game server last sent things. */
export interface MapStatus {
  enabled: boolean;
  ready: boolean;
  message: string;
  dimensions: { id: string; slug: string; label: string; tiles: number; bytes: number }[];
  players: number;
  epoch: number;
  stats: { tiles_received: number; bytes_received: number; last_upload_at: string | null; last_players_at: string | null; last_overlay_at: string | null };
  diagnostics: { stage: string; error: string; world_root: string; region_files: number; regions_rendered: number; queued_tiles: number; tiles_uploaded: number; last_region: string; last_upload_ms: number; age_seconds: number } | null;
  roster: { uuid: string; name: string; dimension: string; x: number; y: number; z: number }[];
}

export type ServerDraft = { instance_id?: string; name: string; map_enabled: boolean; economy_group?: string; access: 'all' | 'members' | 'groups'; allowed_groups: string[]; require_launcher: boolean };

export interface HostedDownload {
  platform: 'windows' | 'mac' | 'linux' | 'android' | 'ios';
  version?: string;
  display_name?: string;
  label: string;
  filename: string;
  file_url: string;
  size: number;
  uploaded_at: string;
}

export interface FaqItem {
  id: string;
  question: string;
  answer: string;
}

export type BlockType = 'hero' | 'download' | 'servers' | 'leaderboard' | 'stats' | 'instances' | 'news' | 'faq' | 'socials' | 'text' | 'image' | 'split' | 'features' | 'gallery' | 'cta' | 'divider' | 'testimonials' | 'video' | 'buttons' | 'map';

export interface LandingBlock {
  id: string;
  type: BlockType;
  enabled: boolean;
  title?: string;
  subtitle?: string;
  options?: Record<string, any>;
}

export interface LandingConfig {
  enabled: boolean;
  brand_name: string;
  logo_url: string | null;
  tagline: string;
  hero_title: string;
  hero_subtitle: string;
  hero_cta_text: string;
  hero_bg_type: 'gradient' | 'image' | 'video';
  hero_bg_url: string | null;
  server_ip: string;
  server_port: number;
  hosted_downloads: HostedDownload[];
  external_download_url: string | null;
  blocks: LandingBlock[];
  faqs: FaqItem[];
  custom_css: string;
  theme: { background: string; surface: string; accent: string; text: string; muted: string; max_width: number; radius: number; font: string; footer_text: string };
}

export interface LevelReward {
  reward_data?: Record<string, unknown>;
  title_image?: string | null;
  id: number;
  level: number;
  reward_type: 'title' | 'item' | 'cosmetic' | 'profile_badge';
  reward_value: string;
  description: string;
  icon?: string;
  discord_role_id?: string;
  luckperms_group?: string;
}

export interface UserLevelInfo {
  uuid: string;
  level: number;
  current_xp: number;
  next_level_xp: number;
  progress_pct: number;
  title: string | null;
  badges: string[];
  rewards: LevelReward[];
}

export interface ServerLevelInfo {
  server_id: number;
  server_name: string;
  uuid: string;
  level: number;
  current_xp: number;
  next_level_xp: number;
  progress_pct: number;
  rank_title: string | null;
}

export interface Quest {
  id: string;
  title: string;
  description: string;
  quest_type: 'daily' | 'weekly';
  category: 'mining' | 'combat' | 'building' | 'farming' | 'social' | 'exploration';
  xp_reward: number;
  stat_type: string;
  target_count: number;
  icon: string;
  active: boolean;
  pinned?: boolean;
  instance_id?: string | null;
  server_id?: number | null;
  difficulty?: string;
  chain_id?: string | null;
  chain_step?: number;
}

export interface UserQuest {
  quest: Quest;
  current_count: number;
  completed: boolean;
  claimed: boolean;
  expires_at: string;
}

export interface Achievement {
  requirement_type?: string;
  id: string;
  title: string;
  description: string;
  category: string;
  xp_reward: number;
  frame_type: 'task' | 'goal' | 'challenge';
  icon_item: string;
  icon_bg: string;
  icon_border: string;
  stat_type?: string | null;
  target_count?: number | null;
  event_type?: string | null;
  unlocked?: boolean;
  unlocked_at?: string | null;
}

export interface Guild {
  id: string;
  instance_id: string;
  name: string;
  tag: string;
  description: string;
  icon_url: string | null;
  banner_url: string | null;
  leader_uuid: string;
  member_count: number;
  max_members: number;
  claims_count: number;
  created_at: string;
  primary?: boolean;
}

export interface GuildMember {
  guild_id: string;
  user_uuid: string;
  username: string;
  role: 'leader' | 'officer' | 'member';
  joined_at: string;
  is_online?: boolean;
}

export interface GuildPost {
  id: string;
  guild_id: string;
  author_uuid: string;
  author_name: string;
  title: string;
  content: string;
  pinned: boolean;
  created_at: string;
}

export interface GuildClaim {
  id: number;
  guild_id: string;
  instance_id: string;
  dimension: string;
  chunk_x: number;
  chunk_z: number;
  claimed_by_uuid: string;
  claimed_at: string;
  guild_name?: string;
  guild_tag?: string;
}

export interface FriendInfo {
  uuid: string;
  username: string;
  status: 'pending_sent' | 'pending_received' | 'accepted';
  is_online: boolean;
  playing_on: string | null;
  avatar_url?: string | null;
}

export interface DirectMessage {
  id: number;
  from_uuid: string;
  to_uuid: string;
  content: string;
  created_at: string;
  is_read: boolean;
}

export interface GameInvite {
  id: string;
  from_uuid: string;
  from_username: string;
  to_uuid: string;
  instance_id: string;
  server_address: string;
  server_name: string;
  status: 'pending' | 'accepted' | 'declined' | 'expired';
  created_at: string;
}

export interface UserPost {
  id: string;
  user_uuid: string;
  username: string;
  content: string;
  image_url: string | null;
  likes_count: number;
  liked_by_me: boolean;
  created_at: string;
}

export interface UserProfileView {
  uuid: string;
  username: string;
  bio: string;
  banner_url: string | null;
  skin_url: string | null;
  skin_model: string;
  cape_url: string | null;
  global_level: UserLevelInfo;
  server_levels: ServerLevelInfo[];
  badges: string[];
  achievements: Achievement[];
  posts: UserPost[];
  friends_count: number;
  stats: PlayerStats | null;
  created_at: string;
}

export interface ProgressionSettings {
  daily_quest_limit: number;
  weekly_quest_limit: number;
  quest_rotation: 'per_player' | 'shared';
  level_base: number;
  level_exponent: number;
  max_level: number;
  global_xp_multiplier: number;
  server_xp_multiplier: number;
  xp_rates: { playtime_per_hour: number; player_kill: number; mob_kill: number; block_broken: number; block_placed: number; message: number };
  rules: { starting_balance: number; guild_base_claims: number; guild_claims_per_member: number; guild_claims_per_level: number; guild_max_members: number; market_max_listings: number;
    auto_money: { enabled: boolean; scale: number; quest_daily: number; quest_weekly: number; quest_other: number; achievement: number; level_base: number; level_exponent: number; milestone: number } };
}

export interface ProgressionView {
  settings: ProgressionSettings;
  quest_pool: { daily: { enabled: number; pinned: number }; weekly: { enabled: number; pinned: number } };
  curve: { level: number; xp: number }[];
}

export interface PlayerProgress {
  uuid: string;
  name: string;
  title: string | null;
  global_xp: number;
  global_level: number;
  level_xp: number;
  level_span: number;
  progress_pct: number;
}

export interface EmbedStyle {
  enabled: boolean; username: string; avatar_url: string; content: string; title: string; url: string; description: string; color: string;
  thumbnail: string; image: string; author_name: string; author_icon: string; footer: string; footer_icon: string; timestamp: boolean;
  fields: { name: string; value: string; inline: boolean }[];
}

export interface LiveEmbed {
  style: EmbedStyle; channel_id: string; webhook_url: string; webhook_set: boolean; interval_secs: number; limit: number; sort: string; server_id: number; row: string; posted: boolean;
}
