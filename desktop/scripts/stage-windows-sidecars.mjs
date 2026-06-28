import { copyFileSync, mkdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const target = process.argv[2] ?? "x86_64-pc-windows-gnu";
const scriptDir = dirname(fileURLToPath(import.meta.url));
const desktopDir = dirname(scriptDir);
const repoDir = dirname(desktopDir);
const releaseDir = join(repoDir, "target", target, "release");
const sidecarDir = join(desktopDir, "src-tauri", "bin");

mkdirSync(sidecarDir, { recursive: true });

for (const name of ["ash", "ai"]) {
  const source = join(releaseDir, `${name}.exe`);
  const destination = join(sidecarDir, `${name}-${target}.exe`);
  statSync(source);
  copyFileSync(source, destination);
  console.log(`${source} -> ${destination}`);
}
