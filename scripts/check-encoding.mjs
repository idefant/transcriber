import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const rootDirectory = process.cwd();
const ignoredDirectories = new Set([
  '.git',
  '.codex',
  'coverage',
  'dist',
  'node_modules',
  'target',
  'ui-audit-artifacts',
]);
const checkedExtensions = new Set([
  '.css',
  '.html',
  '.js',
  '.json',
  '.jsx',
  '.md',
  '.rs',
  '.scss',
  '.toml',
  '.ts',
  '.tsx',
]);
const suspiciousSequences = [
  '\u0420\u045F',
  '\u0420\u0459',
  '\u0420\u045A',
  '\u0420\u045B',
  '\u0420\u040E',
  '\u0420\u201D',
  '\u0420\u2019',
  '\u0420\u0405',
  '\u0420\u00B5',
  '\u0421\u0402',
  '\u0421\u0403',
  '\u0421\u201A',
  '\u0421\u040A',
  '\u0421\u2039',
  '\u0421\u040B',
  '\u0421\u040F',
  '\u0432\u0402',
  '\u00C2',
  '\u00D0',
  '\u00D1',
];

const shouldCheckFile = (filePath) => checkedExtensions.has(path.extname(filePath));

const findTextFiles = async (directory) => {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const absolutePath = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      if (!ignoredDirectories.has(entry.name)) {
        files.push(...(await findTextFiles(absolutePath)));
      }

      continue;
    }

    if (entry.isFile() && shouldCheckFile(absolutePath)) {
      files.push(absolutePath);
    }
  }

  return files;
};

const getLineAndColumn = (text, index) => {
  const precedingText = text.slice(0, index);
  const lines = precedingText.split('\n');

  return {
    column: lines.at(-1).length + 1,
    line: lines.length,
  };
};

const findings = [];

for (const filePath of await findTextFiles(rootDirectory)) {
  const text = await readFile(filePath, 'utf8');

  for (const sequence of suspiciousSequences) {
    const index = text.indexOf(sequence);

    if (index === -1) {
      continue;
    }

    findings.push({
      ...getLineAndColumn(text, index),
      filePath: path.relative(rootDirectory, filePath),
      sequence,
    });
  }
}

if (findings.length > 0) {
  console.error('Possible mojibake sequences were found:');

  for (const finding of findings) {
    console.error(
      `- ${finding.filePath}:${finding.line}:${finding.column} (${JSON.stringify(
        finding.sequence,
      )})`,
    );
  }

  process.exitCode = 1;
}
