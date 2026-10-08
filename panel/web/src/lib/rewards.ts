import { get, put } from './api';

export type RewardAction = {
  type: 'money' | 'item' | 'custom_item' | 'permission' | 'group' | 'claim_chunks' | 'unlock' | 'badge' | 'message' | 'command';
  custom?: string;
  key?: string;
  amount?: number;
  item?: string;
  node?: string;
  value?: boolean;
  minutes?: number;
  group?: string;
  badge?: string;
  text?: string;
  command?: string;
  server_id?: number | null;
};

export type RewardSource = 'quest' | 'achievement' | 'level_reward';

export const loadBundle = (kind: RewardSource, id: string | number) =>
  get<{ actions: RewardAction[] }>(`/api/admin/reward-bundles/${kind}/${encodeURIComponent(String(id))}`).then((r) => r.actions ?? []);

export const saveBundle = (kind: RewardSource, id: string | number, actions: RewardAction[]) =>
  put(`/api/admin/reward-bundles/${kind}/${encodeURIComponent(String(id))}`, { actions });
