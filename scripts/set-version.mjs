/**
 * Sets the version across all three project manifests.
 * Usage: node scripts/set-version.mjs <version>
 * Example: node scripts/set-version.mjs 1.2.3
 *
 * This script is invoked by CI before building so that package.json,
 * src-tauri/tauri.conf.json, and src-tauri/Cargo.toml all carry the
 * same version derived from the git tag.  It does NOT commit changes.
 */

import fs from 'node:fs';
import path from 'node:path';

const version = process.argv[2];
if (!version) {
  console.error('Usage: node scripts/set-version.mjs <version>');
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error(
    `Invalid version format: "${version}". Expected semver like "1.2.3" or "1.2.3-beta.1".`,
  );
  process.exit(1);
}

const root = path.resolve(import.meta.dirname, '..');

// --- package.json ---
const packagePath = path.join(root, 'package.json');
const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
packageJson.version = version;
fs.writeFileSync(packagePath, JSON.stringify(packageJson, undefined, 2) + '\n', 'utf8');
console.log(`package.json          → ${version}`);

// --- src-tauri/tauri.conf.json ---
const tauriConfigPath = path.join(root, 'src-tauri', 'tauri.conf.json');
const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, 'utf8'));
tauriConfig.version = version;
fs.writeFileSync(tauriConfigPath, JSON.stringify(tauriConfig, undefined, 2) + '\n', 'utf8');
console.log(`tauri.conf.json       → ${version}`);

// --- src-tauri/Cargo.toml ---
// Replace only the `version = "..."` line inside the [package] section.
const cargoPath = path.join(root, 'src-tauri', 'Cargo.toml');
let cargoContent = fs.readFileSync(cargoPath, 'utf8');

// We look for the version line that appears before the first [dependencies] section.
// This ensures we only change the [package] version, not dependency versions.
const packageSectionEnd = cargoContent.search(/^\[(?!package)[a-zA-Z]/m);
const packageSection =
  packageSectionEnd === -1 ? cargoContent : cargoContent.slice(0, packageSectionEnd);
const rest = packageSectionEnd === -1 ? '' : cargoContent.slice(packageSectionEnd);

const updatedPackageSection = packageSection.replace(
  /^version\s*=\s*"[^"]*"/m,
  `version = "${version}"`,
);

if (updatedPackageSection === packageSection) {
  console.error('Cargo.toml: could not find `version = "..."` in [package] section.');
  process.exit(1);
}

fs.writeFileSync(cargoPath, updatedPackageSection + rest, 'utf8');
console.log(`Cargo.toml            → ${version}`);

console.log('\nAll versions updated successfully.');
