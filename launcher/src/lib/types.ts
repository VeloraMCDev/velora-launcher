import type { Branding } from '@velora/experience/branding';
export type { Branding, NewsItem, SocialLink } from '@velora/experience/branding';
import type { Experience } from '@velora/experience';
export type Loader = 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge';

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
  loader: Loader;
  loader_version: string | null;
  revision: number;
  server: ServerEntry | null;
  memory: { min_mb: number; max_mb: number };
  jvm_args: string;
  featured: boolean;
  source_label: string;
  file_count: number;
  total_size: number;
}

export interface AuthConfig {
  panel_accounts: boolean;
  registration: 'closed' | 'open' | 'approval';
  yggdrasil_url: string | null;
  offline_local: boolean;
}

export interface Manifest {
  api_version: number;
  panel_version: string;
  branding: Branding;
  auth: AuthConfig;
  instances: Instance[];
  user: { id: number; username: string; uuid: string; role: string; groups: string[] } | null;
}

export interface Account { id: string; kind: 'panel' | 'offline'; username: string; uuid: string; panel_url: string | null; role: string | null }

export type Gc = 'default' | 'g1' | 'zgc';

export interface InstanceOverride { memory_max_mb: number; memory_min_mb: number; jvm_args: string; java_path: string | null }

export interface Settings {
  companion: { enabled: boolean; notifications: boolean; claimBorders: boolean; scale: number; opacity: number; widgets: { id: string; enabled: boolean; x: number; y: number; scale?: number }[] };
  panel_url: string | null;
  selected_instance: string | null;
  memory_max_mb: number;
  memory_min_mb: number;
  custom_resolution: boolean;
  width: number;
  height: number;
  fullscreen: boolean;
  after_launch: 'minimize' | 'hide' | 'keep' | 'close';
  show_console: boolean;
  java_path: string | null;
  gc: Gc;
  jvm_args: string;
  theme_mode: 'server' | 'custom';
  accent: string | null;
  background_video: boolean;
  reduce_motion: boolean;
  glass: boolean | null;
  ui_scale: number;
  concurrent_downloads: number;
  check_updates: boolean;
  send_stats: boolean;
  keybinds_enabled: boolean;
  keybinds: Record<string, string>;
  vanilla_controls_enabled: boolean;
  auto_jump: boolean;
  sensitivity: number;
  show_news: boolean;
  show_social_sidebar: boolean;
  instance_game_options: Record<string, Record<string, string>>;
  instance_overrides: Record<string, InstanceOverride>;
}

export interface Bootstrap {
  app: { version: string; default_panel_url: string | null; panel_locked: boolean; repo: string | null; os: string; arch: string; total_ram_mb: number; data_dir: string };
  panel_url: string | null;
  settings: Settings;
  accounts: Account[];
  active_account: string | null;
  manifest: Manifest | null;
  game_running: { run_id: string; instance_id: string }[];
}

export type Stage = 'preparing' | 'java' | 'game' | 'loader' | 'files' | 'launching';
export type ProgressEvent =
  | { type: 'stage'; stage: Stage; label: string }
  | { type: 'progress'; stage: Stage; done: number; total: number; files_done: number; files_total: number }
  | { type: 'log'; line: string };

export interface ServerStatus { online: boolean; version: string; players_online: number; players_max: number; sample: string[]; motd: string; favicon: string | null; latency_ms: number }
export interface CapeInfo { id: number; name: string; url: string }
export interface PlayerProfile { uuid: string; name: string; skin_url: string | null; skin_model: 'classic' | 'slim'; cape: CapeInfo | null; available_capes: CapeInfo[] }
export interface SkinProfile {
  id: string;
  name: string;
  skin_data: string | null;
  skin_model: 'classic' | 'slim';
  cape_id: number | null;
  cape_name: string | null;
  cape_url: string | null;
  account_id?: string | null;
  created_at: string;
}
export interface UpdateInfo { version: string; notes: string; url: string; size: number; sha256?: string | null }

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

export interface LevelReward {
  id: number;
  level: number;
  reward_type: 'title' | 'item' | 'cosmetic' | 'profile_badge';
  reward_value: string;
  description: string;
  icon?: string;
}

export interface UserLevelInfo {
  title_image?: string | null;
  uuid: string;
  level: number;
  current_xp: number;
  next_level_xp: number;
  progress_pct: number;
  title: string | null;
  rank?: number | null;
  badges: string[];
  rewards: LevelReward[];
}

export interface ServerLevelInfo {
  title_image?: string | null;
  rank_name?: string | null;
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
  motd?: string;
  icon_url: string | null;
  banner_url: string | null;
  leader_uuid: string;
  member_count: number;
  max_members: number;
  claims_count: number;
  created_at: string;
  primary?: boolean;
}

export interface GuildRelation {
  terms_revision?:string;
  reward_cents?: number; reward_bps?: number; expires_at?: string | null;
  id: number;
  guild_id: string;
  other_guild_id: string;
  relation: 'alliance' | 'rival';
  status: 'pending' | 'accepted' | 'declined' | 'ended';
  name: string;
  tag: string;
}

export interface CommunityEvent {
  id: string;
  title: string;
  description: string;
  instance_id?: string | null;
  server_id?: number | null;
  status: string;
  starts_at: string;
  ends_at: string;
  objectives: Array<Record<string, unknown>>;
}

export interface GuildMember {
  uuid: string;
  name: string;
  role: string;
  joined_at: string;
  online?: boolean;
}

export interface GuildPost {
  id: number;
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
  server_id: number;
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
  status: 'pending_outgoing' | 'pending_incoming' | 'accepted';
  online: boolean;
  playing_on: string | null;
  avatar_url?: string | null;
  unread?: number;
}

export interface DirectMessage {
  id: number;
  sender_uuid: string;
  recipient_uuid: string;
  content: string;
  created_at: string;
  is_read: boolean;
}

export interface GameInvite {
  id: string;
  sender_uuid: string;
  sender_name: string;
  recipient_uuid: string;
  instance_id: string;
  instance_name: string;
  server_name: string;
  status: 'pending' | 'accepted' | 'declined' | 'expired';
  created_at: string;
}

export interface UserPost {
  id: number;
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
  global_level: number;
  global_xp: number;
  title: string | null;
  level_info: UserLevelInfo;
  server_levels: ServerLevelInfo[];
  badges: string[];
  achievements: Achievement[];
  achievements_count: number;
  posts: UserPost[];
  friends_count: number;
  stats: PlayerStats | null;
  created_at: string;
  online?: boolean;
  last_seen?: string | null;
  guild?: { id: string; name: string; tag: string; role: string } | null;
  rank?: { display: string; prefix: string; server_name: string } | null;
  favorite_server?: string | null;
  wealth?: number;
  recent_activity?: { kind: string; text: string; at: string }[];
  mutual_friends?: { uuid: string; username: string }[];
  /** self | none | accepted | pending_outgoing | pending_incoming */
  relationship?: string;
  accent_color?: string | null;
  discord_linked?: boolean;
  featured_achievement?: Achievement | null;
}

export interface MemberProfile {
  uuid: string;
  username: string;
  role: string;
  status: string;
  skin_url: string | null;
  global_level: number;
  title: string | null;
  playtime_secs: number;
  last_seen: string | null;
  online: boolean;
  is_friend: boolean;
}

export interface ServerEconomyBalance {
  server_id: number;
  server_name: string;
  balance: number;
  currency_symbol: string;
}

export interface EconomyTransaction {
  id: number;
  server_id: number;
  server_name?: string;
  from_uuid: string;
  from_name: string;
  to_uuid: string;
  to_name: string;
  amount: number;
  description: string;
  created_at: string;
}

export interface BaltopEntry {
  rank: number;
  uuid: string;
  username: string;
  balance: number;
}
