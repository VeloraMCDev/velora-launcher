import type { Settings } from './types';

export class SettingsDomain {
  revision = 0;
  durable: Settings | null = null;
  tail: Promise<unknown> = Promise.resolve();

  persist(candidate: Settings, revision: number, save: () => Promise<unknown>, accepted: () => void, failed: (error: unknown, current: boolean) => void): void {
    const operation = this.tail.then(async () => {
      try {
        await save();
        this.durable = candidate;
        if (revision === this.revision) accepted();
      } catch (error) {
        failed(error, revision === this.revision);
      }
    });
    this.tail = operation.catch(() => {});
  }

  rollback(current: Settings, candidate: Settings): void {
    if (!this.durable) return;
    for (const key of Object.keys(candidate) as (keyof Settings)[]) {
      if (JSON.stringify(current[key]) === JSON.stringify(candidate[key])) {
        Object.assign(current, { [key]: this.durable[key] });
      }
    }
  }
}
