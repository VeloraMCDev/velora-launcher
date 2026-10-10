import { axiosInstance } from '@/api/axios.ts';

export interface ServerStatus {
  configured: boolean;
  panel_url: string;
  latest: { version: string; minecraft: string; notes: string } | null;
  installed_version: string | null;
  installed_file: string | null;
  legacy_install: boolean;
  fabric_api_present: boolean;
  config_present: boolean;
  legacy_config: boolean;
  authlib_present: boolean;
  auto_update: boolean;
  auto_update_enabled: boolean;
  update_available: boolean;
  server_state: string | null;
  last_result: string | null;
  javaagent_flag: string | null;
  error: string | null;
}

export interface AdminSettings {
  panel_url: string;
  auto_update: boolean;
  linked_servers: number;
  latest: { version: string; minecraft: string; server_jar: string } | null;
  error: string | null;
}

const server = (uuid: string) => `/api/client/servers/${uuid}/velora-core`;
const admin = '/api/admin/extensions/net.velora.core';

export const getStatus = async (uuid: string): Promise<ServerStatus> => (await axiosInstance.get(server(uuid))).data;
export const install = async (uuid: string): Promise<string> => (await axiosInstance.post(`${server(uuid)}/install`, {})).data.message;
export const connect = async (uuid: string, token: string, overwrite: boolean): Promise<string> =>
  (await axiosInstance.post(`${server(uuid)}/connect`, { token, overwrite })).data.message;
export const installAuthlib = async (uuid: string): Promise<string> => (await axiosInstance.post(`${server(uuid)}/authlib`, {})).data.message;
export const setAutoUpdate = async (uuid: string, enabled: boolean): Promise<void> => {
  await axiosInstance.put(`${server(uuid)}/auto-update`, { enabled });
};

export const getAdminSettings = async (): Promise<AdminSettings> => (await axiosInstance.get(admin)).data;
export const saveAdminSettings = async (panelUrl: string, autoUpdate: boolean): Promise<void> => {
  await axiosInstance.put(admin, { panel_url: panelUrl, auto_update: autoUpdate });
};
