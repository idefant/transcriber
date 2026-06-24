/**
 * Extracts the release notes for a given version from CHANGELOG.md.
 * Usage: node scripts/extract-changelog.mjs <version>
 * Prints the extracted section to stdout so CI can capture it.
 *
 * Expected CHANGELOG.md format (Keep a Changelog):
 *   ## [1.2.3] - 2026-06-24
 *   ...release notes...
 *   ## [1.2.2] - 2026-05-10
 *   ...
 */

import fs from 'node:fs';
import path from 'node:path';

const version = process.argv[2];
if (!version) {
  console.error('Usage: node scripts/extract-changelog.mjs <version>');
  process.exit(1);
}

const changelogPath = path.join(import.meta.dirname, '..', 'CHANGELOG.md');

if (!fs.existsSync(changelogPath)) {
  // If no CHANGELOG exists, emit an empty body — CI will still work.
  process.exit(0);
}

const content = fs.readFileSync(changelogPath, 'utf8');
const lines = content.split('\n');

// Find the line that starts the section for this version.
const escapedVersion = version.replaceAll('.', String.raw`\.`);
const startPattern = new RegExp(String.raw`^##\s+\[${escapedVersion}\]`, 'i');
const startIndex = lines.findIndex((line) => startPattern.test(line));

if (startIndex === -1) {
  // Version section not found — emit empty body.
  process.exit(0);
}

// Find the next ## heading (next version section or end of file).
const endIndex = lines.findIndex((line, index) => index > startIndex && /^##\s/.test(line));
const sectionLines =
  endIndex === -1 ? lines.slice(startIndex + 1) : lines.slice(startIndex + 1, endIndex);

// Trim leading/trailing blank lines.
const trimmed = sectionLines.join('\n').trim();
process.stdout.write(trimmed);
