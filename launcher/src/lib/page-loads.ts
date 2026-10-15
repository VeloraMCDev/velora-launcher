export class PageLoads<Page> {
  private pending = new Map<string, Promise<Page>>();

  load(key: string, loader: () => Promise<Page>): Promise<Page> {
    const existing = this.pending.get(key);
    if (existing) return existing;
    const request = Promise.resolve().then(loader);
    this.pending.set(key, request);
    void request.catch(() => {
      if (this.pending.get(key) === request) this.pending.delete(key);
    });
    return request;
  }
}
