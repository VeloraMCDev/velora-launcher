import Ajv from 'ajv';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { artifactFields } from '../deployment/contracts/artifact-fields.mjs';

const schema = JSON.parse(readFileSync(new URL('../deployment/contracts/artifact.schema.json', import.meta.url), 'utf8'));
const validate = new Ajv({ strict: true, allErrors: true }).compile(schema);
export function artifactMetadata(input) {
  const {normalized,identity}=artifactFields(input,validate);
  const id = createHash('sha256').update(identity).digest('hex');
  if (input.candidate_id !== undefined && input.candidate_id !== `rc_${id}`) throw Error('Candidate identity and immutable target disagree');
  return { ...normalized, candidate_id: `rc_${id}` };
}

// Metadata is an input, not proof of CI authentication or authorization.
// The future control API must independently verify OIDC claims/run/artifact ownership.
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [input, output] = process.argv.slice(2);
  if (!input || !output) throw Error('Usage: node scripts/artifact-metadata.mjs INPUT.json OUTPUT.json');
  writeFileSync(output, JSON.stringify(artifactMetadata(JSON.parse(readFileSync(input, 'utf8'))), null, 2) + '\n');
  console.log('Validated exact immutable candidate metadata; no registration or deployment performed.');
}
