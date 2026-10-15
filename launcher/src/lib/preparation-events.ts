export type PreparationPhase = 'admitting' | 'preparing' | 'running' | 'idle' | 'error';

export interface OperationIdentity {
  instance_id: string;
  operation_id: string;
}

export interface PreparationRequest {
  instanceId: string;
  operation: Promise<string>;
  operationId: string | null;
  phase: PreparationPhase;
  events: { kind: 'progress' | 'state' | 'log'; apply: () => void }[];
}

export class PreparationEvents {
  request: PreparationRequest | null = null;

  get busy(): boolean {
    return this.request?.phase === 'admitting' || this.request?.phase === 'preparing';
  }

  begin(instanceId: string, invoke: () => Promise<string>): PreparationRequest | null {
    if (this.busy) return null;
    const request: PreparationRequest = { instanceId, operation: invoke(), operationId: null, phase: 'admitting', events: [] };
    this.request = request;
    return request;
  }

  admit(request: PreparationRequest, operationId: string): void {
    request.operationId = operationId;
    request.phase = 'preparing';
    const events = request.events.splice(0);
    if (this.request === request) for (const event of events) event.apply();
  }

  reject(request: PreparationRequest, previous: PreparationRequest | null): boolean {
    request.events = [];
    request.phase = 'error';
    if (this.request !== request) return false;
    this.request = previous;
    return true;
  }

  event(identity: OperationIdentity, kind: 'progress' | 'state' | 'log', apply: () => void): void {
    const request = this.request;
    if (!request || identity.instance_id !== request.instanceId) return;
    const owned = () => {
      if (this.request === request && identity.operation_id === request.operationId) apply();
    };
    if (request.operationId !== null) {
      owned();
      return;
    }
    if (request.events.length >= 128) {
      const expendable = request.events.findIndex((event) => event.kind !== 'state');
      if (expendable < 0 && kind !== 'state') return;
      request.events.splice(Math.max(0, expendable), 1);
    }
    request.events.push({ kind, apply: owned });
  }

  transition(phase: PreparationPhase): boolean {
    if (!this.request || this.request.phase === 'idle' || this.request.phase === 'error') return false;
    if (phase === 'preparing' && this.request.phase === 'running') return false;
    this.request.phase = phase;
    return true;
  }
}
