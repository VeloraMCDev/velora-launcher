import { Alert, Badge, Group, Stack, Switch, Text, TextInput } from '@mantine/core';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import AdminContentContainer from '@/elements/containers/AdminContentContainer.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import { AdminSettings, getAdminSettings, saveAdminSettings } from '../api/veloraCore.ts';

export default function VeloraCoreSettings() {
  const { addToast } = useToast();
  const [loaded, setLoaded] = useState<AdminSettings | null>(null);
  const [panelUrl, setPanelUrl] = useState('');
  const [autoUpdate, setAutoUpdate] = useState(true);
  const [saving, setSaving] = useState(false);

  const load = () =>
    getAdminSettings()
      .then((settings) => {
        setLoaded(settings);
        setPanelUrl(settings.panel_url);
        setAutoUpdate(settings.auto_update);
      })
      .catch((err) => addToast(httpErrorToHuman(err), 'error'));
  // biome-ignore lint/correctness/useExhaustiveDependencies: load once
  useEffect(() => {
    load();
  }, []);

  const save = async () => {
    setSaving(true);
    try {
      await saveAdminSettings(panelUrl, autoUpdate);
      addToast('Velora Core settings saved.', 'success');
      await load();
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    } finally {
      setSaving(false);
    }
  };

  if (!loaded) {
    return (
      <AdminContentContainer title='Velora Core'>
        <Spinner.Centered />
      </AdminContentContainer>
    );
  }

  return (
    <AdminContentContainer title='Velora Core' subtitle='Where the Velora Panel lives, and whether servers update themselves.'>
      <Stack gap='md' maw={640}>
        <TextInput
          label='Velora Panel address'
          description='The public address players and servers use, for example https://velora.example.com. Wings nodes download the mod from here.'
          placeholder='https://velora.example.com'
          value={panelUrl}
          onChange={(event) => setPanelUrl(event.currentTarget.value)}
        />
        <Switch
          label='Allow automatic updates'
          description='Servers that opt in are updated to the newest approved release while they are stopped. Nothing is ever replaced under a running server.'
          checked={autoUpdate}
          onChange={(event) => setAutoUpdate(event.currentTarget.checked)}
        />
        <Group>
          <Button loading={saving} onClick={save}>
            Save
          </Button>
        </Group>
        {loaded.error && <Alert color='red'>The Velora Panel could not be reached: {loaded.error}</Alert>}
        {loaded.panel_url && !loaded.error && (
          <Group gap='xs'>
            <Text size='sm'>Approved release:</Text>
            {loaded.latest ? <Badge>{loaded.latest.version} for Minecraft {loaded.latest.minecraft}</Badge> : <Badge color='gray'>None approved yet. Approve one in the Velora Panel under Servers.</Badge>}
            <Text size='sm' c='dimmed'>{loaded.linked_servers} {loaded.linked_servers === 1 ? 'server updates' : 'servers update'} automatically</Text>
          </Group>
        )}
      </Stack>
    </AdminContentContainer>
  );
}
