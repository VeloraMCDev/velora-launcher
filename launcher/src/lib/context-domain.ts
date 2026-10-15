export class ContextDomain {
  revision = 0;
  selection = 0;
  refresh = 0;
  panel = 0;
  uncertain = false;
  tail: Promise<unknown> = Promise.resolve();

  enqueue<Result>(work: () => Promise<Result>): Promise<Result> {
    const operation = this.tail.then(work);
    this.tail = operation.catch(() => {});
    return operation;
  }

  async mutate<Result>(revision: number, work: () => Promise<Result>, reconcile: (revision: number) => Promise<void>): Promise<Result> {
    try {
      return await this.enqueue(work);
    } catch (error) {
      this.uncertain = true;
      if (revision === this.revision) await reconcile(revision);
      throw error;
    }
  }

  beginRefresh(panel: string | null, account: string | null) {
    return { revision: this.revision, request: ++this.refresh, selection: this.selection, panel, account };
  }

  ownsRefresh(request: ReturnType<ContextDomain['beginRefresh']>, panel: string | null, account: string | null): boolean {
    return request.revision === this.revision && request.request === this.refresh && request.panel === panel && request.account === account;
  }
}
