import type { Account } from './types';

export class AccountDomain {
  revision = 0;
  private additions = new Map<string, number>();

  addition(id: string): number {
    return this.additions.get(id) ?? 0;
  }

  added(accounts: Account[], account: Account): Account[] {
    this.additions.set(account.id, this.addition(account.id) + 1);
    const index = accounts.findIndex(existing => existing.id === account.id);
    if (index < 0) return [...accounts, account];
    return accounts.map((existing, position) => position === index ? account : existing);
  }

  removed(accounts: Account[], id: string): Account[] {
    return accounts.filter(account => account.id !== id);
  }
}
