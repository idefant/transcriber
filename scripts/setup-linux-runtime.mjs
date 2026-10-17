import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, copyFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

// Версия соответствует ONNX API, используемому ort 2.0.0-rc.10.
const version = '1.22.0';
const archiveName = `onnxruntime-linux-x64-${version}`;
const archiveHash = '8344d55f93d5bc5021ce342db50f62079daf39aaafb5d311a451846228be49b3';
const libraryHash = '3da6146e14e7b8aaec625dde11d6114c7457c87a5f93d744897da8781e35c673';
const resourcePath = fileURLToPath(
  new URL('../src-tauri/resources/libonnxruntime.so', import.meta.url),
);
const hash = (data) => createHash('sha256').update(data).digest('hex');

if (process.platform !== 'linux' || process.arch !== 'x64') {
  throw new Error('The bundled Linux runtime currently supports Linux x86_64 only.');
}

const existing = await readFile(resourcePath).catch(() => {});
if (existing !== undefined && hash(existing) === libraryHash) {
  console.log(`ONNX Runtime ${version} is already installed.`);
} else {
  const temporary = await mkdtemp(path.join(tmpdir(), 'transcriber-onnx-'));
  try {
    const response = await fetch(
      `https://github.com/microsoft/onnxruntime/releases/download/v${version}/${archiveName}.tgz`,
    );
    if (!response.ok) throw new Error(`ONNX Runtime download failed: HTTP ${response.status}`);
    const archive = Buffer.from(await response.arrayBuffer());
    if (hash(archive) !== archiveHash) throw new Error('ONNX Runtime archive checksum mismatch.');
    const archivePath = path.join(temporary, 'runtime.tgz');
    await writeFile(archivePath, archive);
    execFileSync('tar', ['-xzf', archivePath, '-C', temporary]);
    const libraryPath = path.join(temporary, archiveName, 'lib', `libonnxruntime.so.${version}`);
    if (hash(await readFile(libraryPath)) !== libraryHash)
      throw new Error('ONNX Runtime library checksum mismatch.');
    await copyFile(libraryPath, resourcePath);
    console.log(`Installed ONNX Runtime ${version}.`);
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}
