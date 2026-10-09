// Email notifications through Resend (https://resend.com/docs/api-reference/emails/send-email).
const escape = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

export function renderEmail({ title, severity = 'info', lines = [], link }) {
  const colour = { critical: '#e5484d', warning: '#f5a524', resolved: '#30a46c', info: '#8f6bff' }[severity] ?? '#8f6bff';
  const text = [title, '', ...lines, ...(link ? ['', link] : []), '', '— Velora Operations'].join('\n');
  const html = `<div style="font-family:Inter,system-ui,sans-serif;background:#0d0d17;padding:28px;color:#dddbe8">
<div style="max-width:560px;margin:auto;background:#14131f;border:1px solid #2c2a40;border-radius:14px;overflow:hidden">
<div style="height:4px;background:${colour}"></div><div style="padding:24px 26px">
<div style="font-size:12px;letter-spacing:.08em;text-transform:uppercase;color:${colour};font-weight:700">${escape(severity)}</div>
<h2 style="margin:6px 0 14px;color:#fff;font-size:20px">${escape(title)}</h2>
${lines.map(l => `<p style="margin:6px 0;line-height:1.55">${escape(l)}</p>`).join('')}
${link ? `<p style="margin-top:20px"><a href="${escape(link)}" style="background:#8f6bff;color:#fff;padding:10px 16px;border-radius:9px;text-decoration:none;font-weight:600">Open the dashboard</a></p>` : ''}
</div></div><p style="text-align:center;color:#6b6780;font-size:12px">Velora Operations</p></div>`;
  return { text, html };
}

export async function sendEmail(settings, message, fetchImpl = fetch) {
  const n = settings.notifications;
  if (!n.resend_api_key || !n.from || !n.recipients.length) throw new Error('Configure a Resend API key, sender and at least one recipient first');
  const { text, html } = renderEmail(message);
  const response = await fetchImpl('https://api.resend.com/emails', {
    method: 'POST',
    headers: { authorization: `Bearer ${n.resend_api_key}`, 'content-type': 'application/json' },
    body: JSON.stringify({ from: n.from, to: n.recipients, subject: `[Velora] ${message.title}`, text, html }),
    signal: AbortSignal.timeout(15_000),
  });
  if (!response.ok) {
    let detail = '';
    try { detail = (await response.json()).message ?? ''; } catch { /* no body */ }
    throw new Error(`Resend rejected the email (${response.status}${detail ? `: ${detail}` : ''})`);
  }
  return response.json();
}
