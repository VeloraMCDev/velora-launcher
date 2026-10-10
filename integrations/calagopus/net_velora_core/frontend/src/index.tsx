import { faCubes } from '@fortawesome/free-solid-svg-icons';
import { Extension, ExtensionContext } from 'shared';
import ServerVeloraCore from './pages/ServerVeloraCore.tsx';
import VeloraCoreSettings from './pages/VeloraCoreSettings.tsx';

class NetVeloraCoreExtension extends Extension {
  // Shown at /admin/extensions/net.velora.core
  public cardConfigurationPage: React.FC | null = VeloraCoreSettings;
  public cardComponent: React.FC | null = null;

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.routes.addServerRoute({
      name: () => 'Velora Core',
      icon: faCubes,
      path: '/velora-core',
      element: ServerVeloraCore,
      permission: 'velora-core.read',
    });
  }
}

export default new NetVeloraCoreExtension();
