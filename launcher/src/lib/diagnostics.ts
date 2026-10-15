export const DIAGNOSTIC_LIMITS = Object.freeze({ lines: 40, inputLine: 512, report: 8192 });

const signals = [
  { category: 'memory', pattern: /OutOfMemoryError|Java heap space/, evidence: 'Memory-related Java error signature.', advice: 'Review the configured game memory in Settings → Game; this snapshot does not measure available memory.' },
  { category: 'java-version', pattern: /UnsupportedClassVersionError/, evidence: 'Java class-version error signature.', advice: 'Review Java selection in Settings → Java; consider Automatic. Compatibility is not verified by this report.' },
  { category: 'memory-configuration', pattern: /Could not reserve enough space|Invalid maximum heap size/, evidence: 'Java memory-configuration error signature.', advice: 'Review the configured memory limit in Settings → Game before retrying.' },
  { category: 'mod-loader', pattern: /Mixin|mixin.*failed|Incompatible mod set/i, evidence: 'Mod or loader error signature.', advice: 'Review the intended mod and loader versions with support; generating this report does not modify files.' },
  { category: 'network', pattern: /UnknownHostException|ConnectException|SocketTimeoutException/, evidence: 'Network exception signature.', advice: 'Review connection settings or retry later; this report does not test connectivity or server availability.' },
  { category: 'file-access', pattern: /AccessDeniedException|Permission denied/, evidence: 'File-access error signature.', advice: 'Review launcher permissions with support; this report does not inspect files or permissions.' },
] as const;

const states = ['idle', 'preparing', 'running', 'error'] as const;
const stages = ['preparing', 'java', 'game', 'loader', 'files', 'launching'] as const;

export interface DiagnosticReport {
  categories: string[];
  advice: string[];
  text: string;
}

function record(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function allowed(value: unknown, choices: readonly string[]): string {
  return typeof value === 'string' && choices.includes(value) ? value : 'unavailable';
}

export function createDiagnosticReport(crashSnapshot?: unknown, preparationSnapshot?: unknown): DiagnosticReport {
  const crash = record(crashSnapshot);
  const preparation = record(preparationSnapshot);
  const exitCode = typeof crash.code === 'number' && Number.isSafeInteger(crash.code) && Math.abs(crash.code) <= 2147483647 ? String(crash.code) : 'unavailable';
  const tail = Array.isArray(crash.tail) ? crash.tail : [];
  const boundedTail = tail.slice(-DIAGNOSTIC_LIMITS.lines);
  const matched = new Set<string>();
  const evidence: string[] = [];
  let omitted = tail.length > boundedTail.length;
  const inspect = (value: unknown): string => {
    if (typeof value !== 'string') {
      omitted = true;
      return '[redacted unrecognized text]';
    }
    if (value.length > DIAGNOSTIC_LIMITS.inputLine) omitted = true;
    const bounded = value.slice(0, DIAGNOSTIC_LIMITS.inputLine).replace(/[\u0000-\u001f\u007f-\u009f\u200b-\u200f\u202a-\u202e\u2060-\u206f]/g, ' ');
    const found = signals.filter((signal) => signal.pattern.test(bounded));
    for (const signal of found) matched.add(signal.category);
    return found.length ? found.map((signal) => signal.evidence).join(' ') : '[redacted unrecognized text]';
  };
  for (const line of boundedTail) evidence.push(inspect(line));
  const preparationEvidence = typeof preparation.message === 'string' ? inspect(preparation.message) : 'unavailable';
  const selected = signals.filter((signal) => matched.has(signal.category));
  const categories = selected.length ? selected.map((signal) => signal.category) : ['unknown'];
  const advice = selected.length ? selected.map((signal) => signal.advice) : ['No recognized error signature in the bounded snapshot. Share this report with support and describe what you were doing; the cause is not established.'];
  const text = [
    'Velora user-support diagnostics v1',
    'Scope: existing crash/preparation snapshot only; not a health check or proof of a cause, repair, or successful launch.',
    'Privacy: original text is replaced with allowlisted signature summaries. Unknown text, identities, addresses, paths and extension fields are omitted.',
    'Sharing: no automatic upload. Copy or export only when you choose; review before sharing.',
    `Exit code: ${exitCode}`,
    `Preparation state: ${allowed(preparation.state, states)}`,
    `Preparation stage: ${allowed(preparation.stage, stages)}`,
    'Preparation context is not correlated to the crashed run and may describe a different operation.',
    `Categories: ${categories.join(', ')}`,
    'Suggested next steps (signature-based, not a diagnosis):',
    ...advice.map((step) => `- ${step}`),
    `Preparation message summary: ${preparationEvidence}`,
    `Crash tail summaries (at most ${DIAGNOSTIC_LIMITS.lines} entries, first ${DIAGNOSTIC_LIMITS.inputLine} characters inspected per entry):`,
    ...(evidence.length ? evidence : ['No crash tail data available.']),
    `Input omitted or truncated: ${omitted ? 'yes' : 'no'}; all original free text is redacted.`,
  ].join('\n');
  return { categories, advice, text: text.slice(0, DIAGNOSTIC_LIMITS.report) };
}
