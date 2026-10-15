import type { LaunchState } from './store.svelte';

export type OperationKind = 'launch' | 'repair';
export function launchShortcutAllowed(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'repeat' | 'defaultPrevented' | 'isComposing'>, editable: boolean, dialogOpen: boolean, inert: boolean): boolean {
  return (event.ctrlKey || event.metaKey) && event.key === 'Enter' && !event.repeat
    && !event.defaultPrevented && !event.isComposing && !editable && !dialogOpen && !inert;
}
export interface OperationContext {
  selectedId: string;
  ownerId: string | null;
  launch: LaunchState;
  hasAccount: boolean;
}

const phases: Record<string, string> = {
  preparing: 'Preparing Minecraft', java: 'Preparing Java', game: 'Preparing game files',
  loader: 'Preparing mod loader', files: 'Preparing instance files', launching: 'Starting Minecraft',
};
const labels: Record<string, string> = {
  'Getting ready…': 'Getting ready', 'Verifying files…': 'Verifying files',
  'Checking Java': 'Checking Java', 'Staging instance files': 'Staging instance files',
  'Checking instance files': 'Checking instance files', 'Downloading loader libraries': 'Downloading loader libraries',
  'Preparing sign-in': 'Preparing sign-in', 'Starting Minecraft': 'Starting Minecraft',
};

export function safePhase(launch: LaunchState): string {
  if (Object.hasOwn(labels, launch.label)) return labels[launch.label];
  const stage = launch.stage ?? 'preparing';
  return Object.hasOwn(phases, stage) ? phases[stage] : 'Preparing Minecraft';
}

function count(value: number): number {
  return Number.isFinite(value) && value >= 0 ? Math.min(Number.MAX_SAFE_INTEGER, Math.floor(value)) : 0;
}

export function preparationProgress(launch: LaunchState) {
  const total = count(launch.total);
  const done = count(launch.done);
  const filesTotal = count(launch.filesTotal);
  const filesDone = count(launch.filesDone);
  const determinate = total > 0;
  return {
    determinate, total, done, filesTotal, filesDone,
    percent: determinate ? Math.min(100, Math.floor(done / total * 100)) : null,
    description: determinate ? 'Current phase only; not overall installation progress.'
      : done > 0 ? 'Work reported; total size is not available.'
      : 'No byte total reported yet; this does not mean installation is complete.',
  };
}

export class OperationPresentation {
  scope = 0;
  ownerId: string | null = null;
  kind: OperationKind | null = null;
  phase = 'Getting ready';
  pending: 'start' | 'cancel' | null = null;
  cancellationRequested = false;
  actionError: string | null = null;
  private previous: LaunchState | null = null;
  private previousState: LaunchState['state'] = 'idle';

  observe(context: OperationContext) {
    const launch = context.launch;
    if (launch.state === 'preparing' && (launch !== this.previous || this.previousState !== 'preparing' || context.ownerId !== this.ownerId)) {
      this.scope++;
      this.ownerId = context.ownerId;
      this.kind = null;
      this.cancellationRequested = false;
      this.actionError = null;
      this.pending = null;
    }
    if (launch.state === 'preparing') this.phase = safePhase(launch);
    this.previous = launch;
    this.previousState = launch.state;
    const mine = context.ownerId === context.selectedId;
    const state = !mine ? 'idle' : launch.state === 'error' ? 'failed'
      : launch.state === 'idle' && this.cancellationRequested ? 'cancelled' : launch.state;
    const busy = launch.state === 'preparing';
    return {
      scope: this.scope, mine, state, busy, phase: this.phase, kind: this.kind,
      progress: preparationProgress(launch), pending: this.pending, actionError: this.actionError,
      canStart: !busy && this.pending === null,
      canRetry: mine && (state === 'failed' || state === 'cancelled') && this.kind !== null && !busy && this.pending === null && (this.kind === 'repair' || context.hasAccount),
      canCancel: mine && busy && this.pending !== 'cancel' && !this.cancellationRequested,
      cancellationRequested: this.cancellationRequested,
      announcement: busy ? mine ? this.cancellationRequested ? 'Cancellation requested. Waiting for preparation to end.' : this.phase
        : 'Another instance is preparing. Switch to that instance to cancel.'
        : state === 'failed' ? `Preparation failed. Last observed phase: ${this.phase}.`
        : state === 'cancelled' ? 'Preparation ended after cancellation request.'
        : state === 'running' ? 'Minecraft is running.' : '',
    };
  }

  async start(kind: OperationKind, expected: OperationContext, current: () => OperationContext, perform: (kind: OperationKind, instanceId: string) => Promise<void>): Promise<boolean> {
    const context = current();
    const view = this.observe(context);
    if (context.selectedId !== expected.selectedId || context.launch !== expected.launch || !view.canStart || (kind === 'launch' && !context.hasAccount)) return false;
    this.pending = 'start';
    this.actionError = null;
    let result: Promise<void>;
    try {
      result = perform(kind, context.selectedId);
      this.observe(current());
      this.kind = kind;
      this.pending = 'start';
    } catch {
      this.pending = null;
      this.actionError = 'Could not request preparation. Please try again.';
      return false;
    }
    const scope = this.scope;
    try { await result; }
    catch {
      this.observe(current());
      if (scope === this.scope) this.actionError = 'Could not request preparation. Please try again.';
    }
    finally {
      this.observe(current());
      if (scope === this.scope && this.pending === 'start') this.pending = null;
    }
    return true;
  }

  async retry(expectedScope: number, expected: OperationContext, current: () => OperationContext, perform: (kind: OperationKind, instanceId: string) => Promise<void>): Promise<boolean> {
    const view = this.observe(current());
    if (expectedScope !== this.scope || !view.canRetry || !this.kind) return false;
    return this.start(this.kind, expected, current, perform);
  }

  async cancel(expectedScope: number, expected: OperationContext, current: () => OperationContext, perform: () => Promise<void>): Promise<boolean> {
    const context = current();
    const view = this.observe(context);
    if (expectedScope !== this.scope || context.selectedId !== expected.selectedId || context.launch !== expected.launch || !view.canCancel) return false;
    this.pending = 'cancel';
    this.cancellationRequested = true;
    this.actionError = null;
    try { await perform(); }
    catch {
      this.observe(current());
      if (expectedScope === this.scope) {
        this.cancellationRequested = false;
        this.actionError = 'Could not request cancellation. Preparation may still be active; try Cancel again.';
      }
    } finally {
      this.observe(current());
      if (expectedScope === this.scope) this.pending = null;
    }
    return true;
  }
}
