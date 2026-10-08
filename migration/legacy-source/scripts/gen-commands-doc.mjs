// Writes docs/commands.md from shared/commands/index.ts, the same list the launcher, the player panel and both
// command palettes show. Run it after editing that file:
//   node --experimental-strip-types scripts/gen-commands-doc.mjs
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { COMMAND_GROUPS, COMMAND_TIPS } from '../shared/commands/index.ts';

const esc = (s) => s.replace(/\|/g, '\\|');
let out = `# Command guide

Every command players and staff can type in game. This page is generated from \`shared/commands/index.ts\`, the same list the launcher's **Command guide**, the player panel's **Commands** page and the **Ctrl+K** palettes use, so they never disagree. Regenerate it with \`node --experimental-strip-types scripts/gen-commands-doc.mjs\`.

Permission nodes are LuckPerms-style. Servers using LuckPerms can grant each node to any group; without it, the defaults in the Paper \`plugin.yml\` (and the Fabric/Forge defaults) apply.

`;
for (const g of COMMAND_GROUPS) {
  out += `## ${g.title}\n\n${g.blurb}\n\n| Command | What it does | Permission |\n|---|---|---|\n`;
  for (const c of g.cmds) {
    const aliases = c.aliases?.length ? ` <br><sub>aliases: ${c.aliases.map((a) => `\`/${a.replace(/^\//, '')}\``).join(', ')}</sub>` : '';
    out += `| \`${esc(c.usage)}\`${aliases} | ${esc(c.does)} | \`${c.perm}\` |\n`;
  }
  out += '\n';
}
out += `## Good to know\n\n${COMMAND_TIPS.map((t) => `- ${t}`).join('\n')}\n`;
writeFileSync(fileURLToPath(new URL('../docs/commands.md', import.meta.url)), out);
console.log(`docs/commands.md written (${COMMAND_GROUPS.reduce((n, g) => n + g.cmds.length, 0)} commands)`);
