import {readFileSync, appendFileSync} from 'node:fs';
import assert from 'node:assert/strict';

const config = JSON.parse(readFileSync('desktop/tauri.conf.json', 'utf8'));
const cargo = readFileSync('desktop/Cargo.toml', 'utf8');
const version = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
assert.match(version ?? '', /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/, 'Invalid desktop version');
assert.equal(config.version, version, 'Cargo and Tauri versions must match');
if (process.env.GITHUB_REF_TYPE === 'tag') {
  assert.equal(process.env.GITHUB_REF_NAME, `v${version}`, 'Release tag must match the desktop version');
}
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `version=${version}\n`);
console.log(version);
