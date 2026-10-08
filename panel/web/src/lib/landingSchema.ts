import type { BlockType, LandingBlock } from './types';

export type Field = {
  key: string;
  label: string;
  type: 'text' | 'textarea' | 'url' | 'media' | 'number' | 'color' | 'toggle' | 'select' | 'list';
  options?: { label: string; value: string }[];
  items?: Field[];
  defaultItem?: Record<string, string>;
  help?: string;
};

export type BlockSchema = {
  type: BlockType;
  label: string;
  description: string;
  group: 'Network' | 'Content' | 'Media' | 'Conversion' | 'Layout';
  defaults: Record<string, any>;
  fields: Field[];
};

const field = (key: string, label: string, type: Field['type'] = 'text'): Field => ({ key, label, type });
const choice = (key: string, label: string, choices: string[]): Field => ({
  key, label, type: 'select', options: choices.map((value) => ({ label: value.replace(/_/g, ' '), value })),
});
const list = (key: string, label: string, items: Field[], defaultItem: Record<string, string>): Field => ({ key, label, type: 'list', items, defaultItem });

export const BLOCK_SCHEMAS: Record<BlockType, BlockSchema> = {
  hero: { type: 'hero', label: 'Hero', description: 'Opening headline, media and calls to action', group: 'Conversion', defaults: { badge_text: '', headline: '', subtitle: '', cta_text: '', cta_url: '', show_ip_copy: true, image_url: '' }, fields: [field('badge_text', 'Badge'), field('headline', 'Headline'), field('subtitle', 'Description', 'textarea'), field('cta_text', 'Button text'), field('cta_url', 'Button URL', 'url'), field('image_url', 'Hero image', 'media'), field('show_ip_copy', 'Show copy IP button', 'toggle')] },
  download: { type: 'download', label: 'Launcher downloads', description: 'Hosted installers and external downloads', group: 'Network', defaults: { layout: 'cards', badge_note: '' }, fields: [choice('layout', 'Layout', ['cards', 'compact']), field('badge_note', 'Highlight note')] },
  servers: { type: 'servers', label: 'Server status', description: 'Live game server cards and copyable addresses', group: 'Network', defaults: { show_offline: true, show_badges: true, cta_label: 'Copy address' }, fields: [field('show_offline', 'Show offline servers', 'toggle'), field('show_badges', 'Show version badges', 'toggle'), field('cta_label', 'Action label')] },
  leaderboard: { type: 'leaderboard', label: 'Leaderboard', description: 'Player rankings from the panel', group: 'Network', defaults: { default_metric: 'playtime_secs', limit: 10, show_podium: true }, fields: [choice('default_metric', 'Metric', ['playtime_secs', 'global_level', 'player_kills', 'mob_kills', 'blocks_broken']), field('limit', 'Maximum players', 'number'), field('show_podium', 'Show podium', 'toggle')] },
  stats: { type: 'stats', label: 'Community stats', description: 'Live counters from your community', group: 'Network', defaults: { show_players: true, show_playtime: true, show_guilds: true, show_blocks: true }, fields: [field('show_players', 'Players', 'toggle'), field('show_playtime', 'Playtime', 'toggle'), field('show_guilds', 'Guilds', 'toggle'), field('show_blocks', 'Blocks', 'toggle')] },
  instances: { type: 'instances', label: 'Instances', description: 'Featured packs from the launcher manifest', group: 'Network', defaults: { limit: 6 }, fields: [field('limit', 'Maximum instances', 'number')] },
  news: { type: 'news', label: 'Announcements', description: 'Updates from launcher news', group: 'Network', defaults: { limit: 3 }, fields: [field('limit', 'Maximum posts', 'number')] },
  faq: { type: 'faq', label: 'FAQ', description: 'Expandable questions and answers', group: 'Content', defaults: {}, fields: [] },
  socials: { type: 'socials', label: 'Community links', description: 'Discord, store and social channels', group: 'Conversion', defaults: { discord_url: '', store_url: '', video_url: '', social_url: '' }, fields: [field('discord_url', 'Discord URL', 'url'), field('store_url', 'Store URL', 'url'), field('video_url', 'Video URL', 'url'), field('social_url', 'Social URL', 'url')] },
  text: { type: 'text', label: 'Rich text', description: 'A headline and readable body copy', group: 'Content', defaults: { eyebrow: '', body: '', align: 'left' }, fields: [field('eyebrow', 'Eyebrow'), field('body', 'Body', 'textarea'), choice('align', 'Alignment', ['left', 'center'])] },
  image: { type: 'image', label: 'Image', description: 'Full-width image with caption and link', group: 'Media', defaults: { url: '', alt: '', caption: '', link: '', fit: 'cover' }, fields: [field('url', 'Image', 'media'), field('alt', 'Alt text'), field('caption', 'Caption'), field('link', 'Link URL', 'url'), choice('fit', 'Fit', ['cover', 'contain'])] },
  split: { type: 'split', label: 'Image + text', description: 'Two-column story with image and button', group: 'Content', defaults: { image_url: '', image_alt: '', body: '', button_text: '', button_url: '', image_side: 'right' }, fields: [field('image_url', 'Image', 'media'), field('image_alt', 'Alt text'), field('body', 'Body', 'textarea'), field('button_text', 'Button text'), field('button_url', 'Button URL', 'url'), choice('image_side', 'Image side', ['left', 'right'])] },
  features: { type: 'features', label: 'Feature cards', description: 'Flexible grid of benefits or highlights', group: 'Content', defaults: { columns: '3', items: [{ icon: '✦', title: 'Feature', body: 'Describe why it matters.' }] }, fields: [choice('columns', 'Columns', ['2', '3', '4']), list('items', 'Cards', [field('icon', 'Icon or emoji'), field('title', 'Title'), field('body', 'Description', 'textarea')], { icon: '✦', title: 'New feature', body: '' })] },
  gallery: { type: 'gallery', label: 'Gallery', description: 'Grid of screenshots or community images', group: 'Media', defaults: { columns: '3', items: [] }, fields: [choice('columns', 'Columns', ['2', '3', '4']), list('items', 'Images', [field('url', 'Image', 'media'), field('alt', 'Alt text'), field('caption', 'Caption')], { url: '', alt: '', caption: '' })] },
  cta: { type: 'cta', label: 'Call to action', description: 'Focused invitation with one strong button', group: 'Conversion', defaults: { body: '', button_text: 'Join now', button_url: '', secondary_text: '', secondary_url: '' }, fields: [field('body', 'Body', 'textarea'), field('button_text', 'Button text'), field('button_url', 'Button URL', 'url'), field('secondary_text', 'Secondary button'), field('secondary_url', 'Secondary URL', 'url')] },
  divider: { type: 'divider', label: 'Divider', description: 'Space or rule between sections', group: 'Layout', defaults: { style: 'line', height: 48 }, fields: [choice('style', 'Style', ['line', 'space', 'glow']), field('height', 'Height', 'number')] },
  testimonials: { type: 'testimonials', label: 'Testimonials', description: 'Player quotes and reviews', group: 'Content', defaults: { columns: '3', items: [] }, fields: [choice('columns', 'Columns', ['2', '3']), list('items', 'Quotes', [field('quote', 'Quote', 'textarea'), field('name', 'Player name'), field('role', 'Role')], { quote: '', name: '', role: '' })] },
  video: { type: 'video', label: 'Video', description: 'Hosted video or YouTube embed', group: 'Media', defaults: { url: '', caption: '', poster: '' }, fields: [field('url', 'Video or YouTube URL', 'media'), field('poster', 'Poster image', 'media'), field('caption', 'Caption')] },
  map: { type: 'map', label: 'Live map', description: 'The Velora Map of a server: terrain, guild land and shops', group: 'Network', defaults: { server_id: 0, height: 560, show_claims: true, show_pins: true, show_players: false }, fields: [field('server_id', 'Server id (0 = the first server with a map)', 'number'), field('height', 'Height (px)', 'number'), field('show_claims', 'Show guild land', 'toggle'), field('show_pins', 'Show spawn, warps, markets and shops', 'toggle'), field('show_players', 'Show players online (live positions!)', 'toggle')] },
  buttons: { type: 'buttons', label: 'Button group', description: 'Custom links and action buttons', group: 'Conversion', defaults: { align: 'center', items: [] }, fields: [choice('align', 'Alignment', ['left', 'center']), list('items', 'Buttons', [field('label', 'Label'), field('url', 'URL', 'url'), choice('style', 'Style', ['primary', 'secondary'])], { label: 'Visit', url: '', style: 'primary' })] },
};

export const BLOCK_GROUPS: BlockSchema['group'][] = ['Network', 'Content', 'Media', 'Conversion', 'Layout'];

export function createLandingBlock(type: BlockType): LandingBlock {
  const schema = BLOCK_SCHEMAS[type];
  return { id: 'b_' + crypto.randomUUID().replaceAll('-', '').slice(0, 12), type, enabled: true, title: schema.label, subtitle: '', options: structuredClone(schema.defaults) };
}

export function upgradeLandingBlock(block: LandingBlock): LandingBlock {
  const schema = BLOCK_SCHEMAS[block.type];
  if (!schema) return block;
  block.options = { ...structuredClone(schema.defaults), ...(block.options ?? {}) };
  return block;
}
