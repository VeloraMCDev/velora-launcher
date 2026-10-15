export type FocusOptions = { onclose: () => void; initialFocus?: () => HTMLElement | null | undefined };

type FocusEntry = { node: HTMLElement; initial: () => void };
type FocusSession = { entries: FocusEntry[]; inert: Map<HTMLElement, boolean> };
const sessions = new WeakMap<Document, FocusSession>();
const focusSelector = 'a[href], button, input, select, textarea, [tabindex], [contenteditable="true"]';

function available(node: HTMLElement): boolean {
  const style = node.ownerDocument.defaultView?.getComputedStyle(node);
  return node.isConnected && !node.closest('[inert], [hidden]') && !node.matches(':disabled')
    && style?.visibility !== 'hidden' && style?.display !== 'none' && node.getClientRects().length > 0;
}

export function focusableElements(node: HTMLElement): HTMLElement[] {
  return Array.from(node.querySelectorAll<HTMLElement>(focusSelector))
    .filter(candidate => candidate.tabIndex >= 0 && available(candidate))
    .sort((first, second) => (first.tabIndex || Infinity) - (second.tabIndex || Infinity));
}

function isolate(session: FocusSession): void {
  for (const [node, previous] of session.inert) node.inert = previous;
  session.inert.clear();
  let branch = session.entries.at(-1)?.node;
  while (branch?.parentElement) {
    for (const sibling of Array.from(branch.parentElement.children)) {
      if (sibling !== branch && sibling instanceof branch.ownerDocument.defaultView!.HTMLElement) {
        session.inert.set(sibling, sibling.inert);
        sibling.inert = true;
      }
    }
    branch = branch.parentElement;
  }
}

export function focusTrap(node: HTMLElement, options: FocusOptions) {
  const document = node.ownerDocument;
  const session = sessions.get(document) ?? { entries: [], inert: new Map<HTMLElement, boolean>() };
  sessions.set(document, session);
  const previous = document.activeElement as HTMLElement | null;
  const previousTabindex = node.getAttribute('tabindex');
  node.tabIndex = -1;
  let destroyed = false;
  const entry: FocusEntry = {
    node,
    initial: () => {
      const preferred = options.initialFocus?.();
      const target = preferred && node.contains(preferred) && available(preferred)
        ? preferred : focusableElements(node)[0] ?? node;
      target.focus({ preventScroll: true });
    },
  };
  session.entries.push(entry);
  isolate(session);
  queueMicrotask(() => {
    if (!destroyed && session.entries.at(-1) === entry) entry.initial();
  });

  function keydown(event: KeyboardEvent): void {
    if (session.entries.at(-1) !== entry || event.isComposing) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopImmediatePropagation();
      options.onclose();
    } else if (event.key === 'Tab') {
      event.preventDefault();
      event.stopImmediatePropagation();
      const candidates = focusableElements(node);
      const current = candidates.indexOf(document.activeElement as HTMLElement);
      const next = current < 0 ? (event.shiftKey ? candidates.length - 1 : 0)
        : (current + (event.shiftKey ? -1 : 1) + candidates.length) % candidates.length;
      (candidates[next] ?? node).focus({ preventScroll: true });
    }
  }

  function focusin(event: FocusEvent): void {
    if (session.entries.at(-1) === entry && !node.contains(event.target as Node)) entry.initial();
  }

  document.addEventListener('keydown', keydown, true);
  document.addEventListener('focusin', focusin, true);
  return {
    update(nextOptions: FocusOptions) { options = nextOptions; },
    destroy() {
      destroyed = true;
      document.removeEventListener('keydown', keydown, true);
      document.removeEventListener('focusin', focusin, true);
      const wasTop = session.entries.at(-1) === entry;
      session.entries.splice(session.entries.indexOf(entry), 1);
      isolate(session);
      if (previousTabindex === null) node.removeAttribute('tabindex');
      else node.setAttribute('tabindex', previousTabindex);
      if (wasTop) {
        const top = session.entries.at(-1);
        if (previous && available(previous) && (!top || top.node.contains(previous))) previous.focus({ preventScroll: true });
        else top?.initial();
      }
      if (!session.entries.length) sessions.delete(document);
    },
  };
}
