import { Alert, Badge, Checkbox, Code, Group, PasswordInput, Stack, Text } from '@mantine/core';
import { useCallback, useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import TitleCard from '@/elements/data-display/TitleCard.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import { connect, getStatus, installAuthlib, ServerStatus } from '../api/veloraCore.ts';

export default function ServerVeloraCore() {
  const { addToast } = useToast();
  const server = useServerStore((state) => state.server);
  const canManage = useServerCan('velora-core.manage');

  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [token, setToken] = useState('');
  const [overwrite, setOverwrite] = useState(false);

  const refresh = useCallback(() => getStatus(server.uuid).then(setStatus).catch((err) => addToast(httpErrorToHuman(err), 'error')), [server.uuid, addToast]);
  useEffect(() => {
    refresh();
    const timer = setInterval(refresh, 15000);
    return () => clearInterval(timer);
  }, [refresh]);

  const run = async (name: string, action: () => Promise<string | void>) => {
    setBusy(name);
    try {
      const message = await action();
      if (message) addToast(message, 'success');
      await refresh();
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    } finally {
      setBusy(null);
    }
  };

  if (!status) {
    return (
      <ServerContentContainer title='Velora Core'>
        <Spinner.Centered />
      </ServerContentContainer>
    );
  }

  const hasJar = status.installed_version !== null;

  return (
    <ServerContentContainer title='Velora Core' subtitle='Connect this server to your Velora Panel and inspect its Velora Core installation.'>
      <Stack gap='md'>
        {!status.configured && (
          <Alert color='yellow' title='The Velora Panel address is not set'>
            An administrator needs to set it under Admin, Extensions, Velora Core before this server can be connected.
          </Alert>
        )}
        {status.error && <Alert color='red'>{status.error}</Alert>}
        {status.last_result && <Alert color='blue'>{status.last_result}</Alert>}

        <TitleCard title='Mod'>
          <Stack gap='sm' p='md'>
            <Group gap='xs'>
              <Text size='sm'>Installed:</Text>
              {hasJar ? <Badge color={status.update_available ? 'yellow' : 'green'}>{status.installed_version}</Badge> : <Badge color='gray'>Not installed</Badge>}
              <Text size='sm'>Approved release:</Text>
              {status.latest ? <Badge>{status.latest.version} for Minecraft {status.latest.minecraft}</Badge> : <Badge color='gray'>None approved yet</Badge>}
            </Group>
            {status.legacy_install && <Text size='sm' c='dimmed'>An older build is installed. Download Velora Core from the Admin Panel and replace the old mod jar while the server is stopped.</Text>}
            {!status.fabric_api_present && hasJar && <Alert color='yellow'>Fabric API was not found in the mods folder. Velora Core needs it.</Alert>}
            <Text size='sm' c='dimmed'>Download the Server jar from the Velora Admin Panel under Instance setup / Mod downloads. Stop this server, upload the jar to mods/, remove the previous Velora Core jar and restart. Install Fabric API for the matching Minecraft version.</Text>
            {status.panel_url && <a href={status.panel_url + '/#/instances'} target='_blank' rel='noreferrer'>Open Velora Admin Panel</a>}
          </Stack>
        </TitleCard>

        <TitleCard title='Connect to the Velora Panel'>
          <Stack gap='sm' p='md'>
            <Group gap='xs'>
              <Text size='sm'>Config:</Text>
              {status.config_present ? <Badge color='green'>velora-core.properties</Badge> : status.legacy_config ? <Badge color='yellow'>Old scopenet.properties (copied on first start)</Badge> : <Badge color='gray'>Not written yet</Badge>}
            </Group>
            <Text size='sm' c='dimmed'>Paste the server token from the Velora Panel's Servers page. It is written to this server's config and is not kept by Calagopus.</Text>
            <PasswordInput label='Server token' placeholder='sn_…' value={token} onChange={(event) => setToken(event.currentTarget.value)} disabled={!canManage || !status.configured} autoComplete='off' />
            {status.config_present && <Checkbox label='Replace the existing config (resets every option in it)' checked={overwrite} onChange={(event) => setOverwrite(event.currentTarget.checked)} />}
            <Group>
              <Button
                loading={busy === 'connect'}
                disabled={!canManage || !status.configured || token.trim() === '' || (status.config_present && !overwrite) || busy !== null}
                onClick={() => run('connect', async () => { const message = await connect(server.uuid, token.trim(), overwrite); setToken(''); setOverwrite(false); return message; })}
              >
                Write config
              </Button>
            </Group>
          </Stack>
        </TitleCard>

        <TitleCard title='Velora sign-in'>
          <Stack gap='sm' p='md'>
            <Text size='sm'>Players sign in with their Velora account when the server starts with authlib-injector and online-mode stays on.</Text>
            <Group gap='xs'>
              <Text size='sm'>authlib-injector.jar:</Text>
              {status.authlib_present ? <Badge color='green'>In the server folder</Badge> : <Badge color='gray'>Missing</Badge>}
            </Group>
            {!status.authlib_present && (
              <Group>
                <Button loading={busy === 'authlib'} disabled={!canManage || !status.configured || busy !== null} onClick={() => run('authlib', () => installAuthlib(server.uuid))}>
                  Download authlib-injector
                </Button>
              </Group>
            )}
            {status.javaagent_flag && (
              <>
                <Text size='sm'>Add this to the start of the Java arguments (the startup command, before <Code>-jar</Code>), and keep <Code>online-mode=true</Code> in server.properties:</Text>
                <Code block>{status.javaagent_flag}</Code>
              </>
            )}
          </Stack>
        </TitleCard>
      </Stack>
    </ServerContentContainer>
  );
}

