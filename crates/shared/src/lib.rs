//! Wire types shared by the Velora launcher and the admin panel.
//!
//! Everything the panel sends to the launcher is described here, so both
//! sides always agree on the JSON shape.

use serde::{Deserialize, Serialize};
mod experience;
mod updates;
pub use experience::*;
pub use updates::{newer_release, release_version, LauncherUpdate};

pub const API_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Branding / theming
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Branding {
    pub name: String,
    pub tagline: String,
    pub logo_url: Option<String>,
    pub icon_url: Option<String>,
    pub background: Background,
    pub colors: Palette,
    /// One of the fonts bundled in the launcher: "Inter", "Outfit",
    /// "Space Grotesk", "Plus Jakarta Sans".
    pub font: String,
    /// Corner radius in px used for cards and buttons.
    pub radius: u8,
    /// Frosted-glass surfaces (backdrop blur). Disable for very old GPUs.
    pub glass: bool,
    /// Shown on the Minecraft title screen / F3 (the `version_type` arg).
    pub version_label: String,
    pub news: Vec<NewsItem>,
    pub links: Vec<SocialLink>,
    pub about: AboutContent,
    pub features: Features,
    /// Raw CSS appended to the launcher stylesheet (power users).
    pub custom_css: String,
}

impl Default for Branding {
    fn default() -> Self {
        Self {
            name: "Velora".into(),
            tagline: "Your worlds, one click away.".into(),
            logo_url: None,
            icon_url: None,
            background: Background::default(),
            colors: Palette::default(),
            font: "Inter".into(),
            radius: 10,
            glass: false,
            version_label: "Velora".into(),
            news: vec![NewsItem {
                id: "welcome".into(),
                title: "Welcome to Velora".into(),
                body: "Your admin can customise this launcher from the panel: colours, news, instances and more.".into(),
                image_url: None,
                link: None,
                date: None,
                pinned: true,
                tag: Some("News".into()),
            }],
            links: vec![],
            about: AboutContent::default(),
            features: Features::default(),
            custom_css: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct AboutContent {
    pub title: String,
    pub icon_url: Option<String>,
    pub body: String,
    pub links: Vec<SocialLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Background {
    /// "gradient" | "image" | "video"
    pub kind: String,
    pub url: Option<String>,
    /// Blur in px applied to image/video backgrounds.
    pub blur: u8,
    /// Darkening overlay 0-100.
    pub dim: u8,
}

impl Default for Background {
    fn default() -> Self {
        Self { kind: "gradient".into(), url: None, blur: 0, dim: 55 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Palette {
    pub accent: String,
    pub accent_2: String,
    pub background: String,
    pub surface: String,
    pub text: String,
    pub muted: String,
    pub success: String,
    pub danger: String,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            accent: "#6d6af5".into(),
            accent_2: "#8b88f8".into(),
            background: "#0d0e12".into(),
            surface: "#16181e".into(),
            text: "#ecedf1".into(),
            muted: "#8b8f9a".into(),
            success: "#3fb97c".into(),
            danger: "#e5484d".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub body: String,
    pub image_url: Option<String>,
    pub link: Option<String>,
    /// ISO date (YYYY-MM-DD)
    pub date: Option<String>,
    pub pinned: bool,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct SocialLink {
    pub label: String,
    pub url: String,
    /// "discord" | "website" | "youtube" | "twitter" | "github" | "store" | "twitch"
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Features {
    pub news: bool,
    pub server_status: bool,
    /// Let players override colours/theme in their own settings.
    pub allow_user_theme: bool,
    /// Let players change JVM arguments / Java path.
    pub allow_advanced_java: bool,
}

impl Default for Features {
    fn default() -> Self {
        Self { news: true, server_status: true, allow_user_theme: true, allow_advanced_java: true }
    }
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RegistrationMode {
    /// Only admins can create accounts.
    #[default]
    Closed,
    /// Anyone can sign up from the launcher.
    Open,
    /// Anyone can sign up, but an admin must approve the account.
    Approval,
}

/// Public auth configuration the launcher uses to decide which sign-in
/// options to show.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AuthConfig {
    pub panel_accounts: bool,
    pub registration: RegistrationMode,
    pub offline_local: bool,
    /// Absolute URL of the panel's Yggdrasil (authlib-injector) API root.
    /// Filled in by the panel; the launcher falls back to `{panel}/api/yggdrasil`.
    pub yggdrasil_url: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self { panel_accounts: true, registration: RegistrationMode::Closed, offline_local: true, yggdrasil_url: None }
    }
}

/// Yggdrasil session tokens handed to the launcher at sign-in. The access
/// token is what the game (via authlib-injector) uses to join servers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YggdrasilTokens {
    pub access_token: String,
    pub client_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct CapeInfo {
    pub id: i64,
    pub name: String,
    pub url: String,
}

/// A player's in-game identity: UUID, skin and cape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct PlayerProfile {
    pub uuid: String,
    pub name: String,
    pub skin_url: Option<String>,
    /// "classic" (Steve arms) or "slim" (Alex arms).
    pub skin_model: String,
    pub cape: Option<CapeInfo>,
    /// Capes this player is allowed to pick.
    pub available_capes: Vec<CapeInfo>,
}

/// A saved skin and cape preset (wardrobe look).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkinProfile {
    pub id: String,
    pub name: String,
    pub skin_data: Option<String>,
    #[serde(default = "default_skin_model")]
    pub skin_model: String,
    #[serde(default)]
    pub cape_id: Option<i64>,
    #[serde(default)]
    pub cape_name: Option<String>,
    #[serde(default)]
    pub cape_url: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub created_at: String,
}

fn default_skin_model() -> String {
    "classic".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicUser {
    pub id: i64,
    pub username: String,
    /// The player's UUID (dashed). New accounts get the offline-mode UUID for
    /// their name, so offline and authenticated servers agree.
    pub uuid: String,
    pub role: String,
    pub groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Panel session token (manifest, skins, account API).
    pub token: String,
    pub user: PublicUser,
    /// Set when the account exists but is waiting for admin approval.
    #[serde(default)]
    pub pending: bool,
    /// Game session for authlib-injector; absent for pending accounts.
    #[serde(default)]
    pub yggdrasil: Option<YggdrasilTokens>,
}

// ---------------------------------------------------------------------------
// Instances
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    #[default]
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl Loader {
    pub fn as_str(&self) -> &'static str {
        match self {
            Loader::Vanilla => "vanilla",
            Loader::Fabric => "fabric",
            Loader::Quilt => "quilt",
            Loader::Forge => "forge",
            Loader::NeoForge => "neoforge",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "vanilla" | "" => Loader::Vanilla,
            "fabric" | "fabric-loader" => Loader::Fabric,
            "quilt" | "quilt-loader" => Loader::Quilt,
            "forge" => Loader::Forge,
            "neoforge" | "neo-forge" => Loader::NeoForge,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ServerEntry {
    pub name: String,
    pub address: String,
    pub port: u16,
    /// Connect straight to the server when the game starts.
    pub auto_join: bool,
    /// Add the server to the in-game multiplayer list (servers.dat).
    pub inject: bool,
}

impl Default for ServerEntry {
    fn default() -> Self {
        Self { name: String::new(), address: String::new(), port: 25565, auto_join: false, inject: true }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MemoryDefaults {
    pub min_mb: u32,
    pub max_mb: u32,
}

impl Default for MemoryDefaults {
    fn default() -> Self {
        Self { min_mb: 1024, max_mb: 4096 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct InstanceSummary {
    pub experience: Experience,
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub logo_url: Option<String>,
    pub mc_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    /// Bumped by the panel whenever the instance changes; the launcher
    /// re-syncs files when this differs from what it last installed.
    pub revision: i64,
    /// Raised by an admin's "clean update": a launcher that last cleaned at a lower number wipes the instance's old files first.
    pub clean_epoch: i64,
    pub server: Option<ServerEntry>,
    pub memory: MemoryDefaults,
    pub jvm_args: String,
    pub featured: bool,
    /// Human description of where the instance came from ("Modrinth: Fabulously Optimized 6.2").
    pub source_label: String,
    pub file_count: u32,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FileEntry {
    /// Path relative to the instance game directory, always using `/`.
    pub path: String,
    /// Absolute URL, or a panel-relative URL starting with `/`.
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct InstanceManifest {
    pub instance: InstanceSummary,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct LauncherManifest {
    pub api_version: u32,
    pub panel_version: String,
    pub branding: Branding,
    pub auth: AuthConfig,
    pub instances: Vec<InstanceSummary>,
    /// Present when the request carried a valid panel token.
    pub user: Option<PublicUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchEvent {
    pub instance_id: String,
    pub kind: String,
    #[serde(default)]
    pub username: Option<String>,
}

// ---------------------------------------------------------------------------
// Leveling & Quests
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UserLevelInfo {
    pub uuid: String,
    pub global_xp: i64,
    #[serde(default)]
    pub current_xp: i64,
    pub global_level: i64,
    #[serde(default)]
    pub level: i64,
    pub current_level_xp: i64,
    pub next_level_xp: i64,
    pub progress_pct: f64,
    pub title: Option<String>,
    #[serde(default)]
    pub title_image: Option<String>,
    pub badges: Vec<String>,
    pub server_levels: Vec<ServerLevelInfo>,
    #[serde(default)]
    pub rank: Option<usize>,
    #[serde(default)]
    pub rewards: Vec<LevelReward>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LevelReward {
    pub id: i64,
    pub level: i64,
    pub reward_type: String,
    pub reward_value: String,
    pub description: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub server_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServerLevelInfo {
    pub server_id: i64,
    pub server_name: String,
    pub server_xp: i64,
    pub server_level: i64,
    pub current_level_xp: i64,
    pub next_level_xp: i64,
    pub progress_pct: f64,
    pub rank_name: Option<String>,
    #[serde(default)]
    pub title_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub period: String,   // "daily", "weekly"
    pub category: String, // "mining", "combat", "building", "farming", "social", "exploration"
    pub target_stat: String,
    pub target_count: i64,
    pub xp_reward: i64,
    pub icon: String,
    pub enabled: bool,
    #[serde(default)]
    pub quest_type: Option<String>,
    #[serde(default)]
    pub stat_type: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub server_id: Option<i64>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub chain_id: Option<String>,
    #[serde(default)]
    pub chain_step: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UserQuest {
    pub quest: Quest,
    #[serde(default)]
    pub progress: i64,
    #[serde(default)]
    pub current_count: i64,
    pub completed: bool,
    pub claimed: bool,
    pub period_key: String,
    #[serde(default)]
    pub expires_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Achievements
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Achievement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub icon_frame: String, // "task", "goal", "challenge"
    #[serde(default)]
    pub frame_type: Option<String>,
    pub icon_item: String,
    pub icon_bg: String,
    pub icon_border: String,
    pub xp_reward: i64,
    pub secret: bool,
    #[serde(default)]
    pub stat_type: Option<String>,
    #[serde(default)]
    pub target_count: Option<i64>,
    #[serde(default)]
    pub unlocked: bool,
    #[serde(default)]
    pub unlocked_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Economy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServerEconomyBalance {
    pub server_id: i64,
    pub server_name: String,
    pub balance: f64,
    pub currency_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EconomyTransaction {
    pub id: i64,
    pub server_id: i64,
    #[serde(default)]
    pub server_name: Option<String>,
    pub from_uuid: String,
    pub from_name: String,
    pub to_uuid: String,
    pub to_name: String,
    pub amount: f64,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BaltopEntry {
    pub rank: usize,
    pub uuid: String,
    pub username: String,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MarketListing {
    pub id: i64,
    pub server_id: i64,
    pub seller_uuid: String,
    pub seller_name: String,
    pub item_id: String,
    pub item_name: String,
    pub amount: i32,
    pub price: f64,
    pub created_at: String,
    /// Set when a guild is selling; the sale is paid into its bank.
    #[serde(default)]
    pub seller_guild_tag: Option<String>,
    /// `buy_now` or `auction`.
    #[serde(default)]
    pub kind: String,
    /// Auctions only: when bidding closes, the highest bid so far and who made it.
    #[serde(default)]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub current_bid: Option<f64>,
    #[serde(default)]
    pub bidder_name: Option<String>,
    #[serde(default)]
    pub bid_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MemberProfile {
    pub uuid: String,
    pub username: String,
    pub role: String,
    pub status: String,
    pub skin_url: Option<String>,
    pub global_level: i64,
    pub title: Option<String>,
    pub playtime_secs: i64,
    pub last_seen: Option<String>,
    pub online: bool,
    pub is_friend: bool,
    /// `none`, `accepted`, `pending_outgoing` or `pending_incoming`.
    #[serde(default)]
    pub friendship_status: String,
}

// ---------------------------------------------------------------------------
// Guilds & Land Claiming
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Guild {
    pub id: String,
    pub instance_id: String,
    pub name: String,
    pub tag: String,
    pub description: String,
    pub motd: String,
    pub leader_uuid: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub level: i64,
    pub xp: i64,
    pub max_claims: i64,
    pub member_count: i64,
    pub claims_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GuildMember {
    pub uuid: String,
    pub name: String,
    pub role: String, // "leader", "officer", "member", "recruit"
    pub joined_at: String,
    #[serde(default)]
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GuildPost {
    pub id: i64,
    pub guild_id: String,
    pub author_uuid: String,
    pub author_name: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GuildClaim {
    pub id: i64,
    pub guild_id: String,
    pub guild_name: String,
    pub guild_tag: String,
    pub server_id: i64,
    pub dimension: String,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub claimed_by_uuid: String,
    pub claimed_at: String,
}

// ---------------------------------------------------------------------------
// Social: Friends, DMs, Invites, Profiles & Posts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FriendInfo {
    pub uuid: String,
    pub username: String,
    pub status: String, // "accepted", "pending_incoming", "pending_outgoing"
    pub online: bool,
    pub playing_on: Option<String>,
    pub last_seen: Option<String>,
    /// Messages from this friend that you haven't read yet.
    #[serde(default)]
    pub unread: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DirectMessage {
    pub id: i64,
    pub sender_uuid: String,
    pub sender_name: String,
    pub recipient_uuid: String,
    pub content: String,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GameInvite {
    pub id: String,
    pub sender_uuid: String,
    pub sender_name: String,
    pub recipient_uuid: String,
    pub instance_id: String,
    pub instance_name: String,
    pub server_id: Option<i64>,
    pub server_name: Option<String>,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UserPost {
    pub id: i64,
    pub user_uuid: String,
    pub author_name: String,
    pub content: String,
    pub image_url: Option<String>,
    pub likes_count: i64,
    #[serde(default)]
    pub liked_by_me: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ProfileGuild {
    pub id: String,
    pub name: String,
    pub tag: String,
    pub role: String,
}

/// A rank reported by a server's permissions plugin (LuckPerms), shown read-only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ProfileRank {
    pub display: String,
    pub prefix: String,
    pub server_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ProfileActivity {
    /// `join`, `death`, `advancement`, `kill` or `achievement`.
    pub kind: String,
    pub text: String,
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MutualFriend {
    pub uuid: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UserProfileView {
    pub uuid: String,
    pub username: String,
    pub bio: String,
    pub banner_url: Option<String>,
    pub custom_badge: Option<String>,
    pub title: Option<String>,
    pub global_level: i64,
    pub global_xp: i64,
    pub server_levels: Vec<ServerLevelInfo>,
    pub featured_achievement: Option<Achievement>,
    pub achievements_count: usize,
    pub posts: Vec<UserPost>,
    pub level_info: UserLevelInfo,
    pub achievements: Vec<Achievement>,
    pub badges: Vec<String>,
    pub friends_count: i64,
    pub created_at: String,
    pub stats: Option<serde_json::Value>,
    #[serde(default)]
    pub online: bool,
    #[serde(default)]
    pub last_seen: Option<String>,
    #[serde(default)]
    pub guild: Option<ProfileGuild>,
    #[serde(default)]
    pub rank: Option<ProfileRank>,
    /// The server they spend the most time on.
    #[serde(default)]
    pub favorite_server: Option<String>,
    /// Total balance across the economies they hold money in.
    #[serde(default)]
    pub wealth: f64,
    #[serde(default)]
    pub recent_activity: Vec<ProfileActivity>,
    #[serde(default)]
    pub mutual_friends: Vec<MutualFriend>,
    /// Relationship to the person looking: `self`, `none`, `accepted`, `pending_outgoing` or `pending_incoming`.
    #[serde(default)]
    pub relationship: String,
    /// Profile cosmetic: a `#rrggbb` accent colour.
    #[serde(default)]
    pub accent_color: Option<String>,
    #[serde(default)]
    pub discord_linked: bool,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The UUID an offline-mode server assigns to `name`
/// (Java's `UUID.nameUUIDFromBytes("OfflinePlayer:" + name)`).
pub fn offline_uuid(name: &str) -> String {
    use md5::{Digest, Md5};
    let mut hash: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    hash[6] = (hash[6] & 0x0f) | 0x30; // version 3
    hash[8] = (hash[8] & 0x3f) | 0x80; // IETF variant
    let h: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// Minecraft usernames: 3-16 chars of `[A-Za-z0-9_]`.
pub fn valid_username(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_matches_java() {
        // Reference values from java.util.UUID.nameUUIDFromBytes.
        assert_eq!(offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
        assert_eq!(offline_uuid("jeb_"), "a762f560-4fce-3236-812a-b80efff0b62b");
    }

    #[test]
    fn usernames() {
        assert!(valid_username("Steve_01"));
        assert!(!valid_username("ab"));
        assert!(!valid_username("has space"));
        assert!(!valid_username("waytoolongusername1"));
    }

    #[test]
    fn manifest_roundtrip_with_defaults() {
        let m: LauncherManifest = serde_json::from_str("{}").unwrap();
        assert_eq!(m.branding.name, "Velora");
        let loader: Loader = serde_json::from_str("\"neoforge\"").unwrap();
        assert_eq!(loader, Loader::NeoForge);
    }
}
