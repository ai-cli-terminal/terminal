import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const target = process.argv[2] ?? "x86_64-pc-windows-gnu";
const scriptDir = dirname(fileURLToPath(import.meta.url));
const desktopDir = dirname(scriptDir);
const repoDir = dirname(desktopDir);
const tauriReleaseDir = join(
  desktopDir,
  "src-tauri",
  "target",
  target,
  "release"
);
const rootReleaseDir = join(repoDir, "target", target, "release");
const packageName = `ai-terminal-windows-${target}`;
const portableDir = join(tauriReleaseDir, "portable");
const packageDir = join(portableDir, packageName);
const archivePath = join(portableDir, `${packageName}.zip`);

rmSync(packageDir, { recursive: true, force: true });
mkdirSync(packageDir, { recursive: true });

const requiredFiles = [
  [join(tauriReleaseDir, "ai-terminal.exe"), "ai-terminal.exe"],
  [join(rootReleaseDir, "ash.exe"), "ash.exe"],
  [join(rootReleaseDir, "ai.exe"), "ai.exe"],
  [join(repoDir, "scripts", "smoke-gui.ps1"), "smoke-gui.ps1"]
];
const optionalFiles = [
  [join(tauriReleaseDir, "WebView2Loader.dll"), "WebView2Loader.dll"]
];
const files = [...requiredFiles];

for (const [source, name] of optionalFiles) {
  if (existsSync(source)) {
    files.push([source, name]);
  } else {
    console.warn(`optional portable file not found; skipping ${name}`);
  }
}

for (const [source, name] of requiredFiles) {
  statSync(source);
}

for (const [source, name] of files) {
  copyFileSync(source, join(packageDir, name));
}

const checksums = files
  .map(([, name]) => {
    const data = readFileSync(join(packageDir, name));
    const hash = createHash("sha256").update(data).digest("hex");
    return `${hash}  ${name}`;
  })
  .join("\n");

writeFileSync(join(packageDir, "SHA256SUMS.txt"), `${checksums}\n`);
console.log(packageDir);

if (process.env.AI_TERMINAL_SKIP_ZIP !== "1") {
  rmSync(archivePath, { force: true });
  const zip = spawnSync("zip", ["-qr", `${packageName}.zip`, packageName], {
    cwd: portableDir,
    stdio: "inherit"
  });

  if (zip.error?.code === "ENOENT") {
    const powershell = spawnSync(
      "powershell",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        "Compress-Archive -LiteralPath $env:AI_TERMINAL_PACKAGE_NAME -DestinationPath $env:AI_TERMINAL_ARCHIVE_PATH -Force"
      ],
      {
        cwd: portableDir,
        stdio: "inherit",
        env: {
          ...process.env,
          AI_TERMINAL_PACKAGE_NAME: packageName,
          AI_TERMINAL_ARCHIVE_PATH: archivePath
        }
      }
    );
    if (powershell.error?.code === "ENOENT") {
      console.warn("zip and powershell were not found; portable directory was generated without an archive");
    } else if (powershell.status !== 0) {
      process.exit(powershell.status ?? 1);
    } else {
      const data = readFileSync(archivePath);
      const hash = createHash("sha256").update(data).digest("hex");
      writeFileSync(`${archivePath}.sha256`, `${hash}  ${packageName}.zip\n`);
      console.log(archivePath);
    }
  } else if (zip.status !== 0) {
    process.exit(zip.status ?? 1);
  } else {
    const data = readFileSync(archivePath);
    const hash = createHash("sha256").update(data).digest("hex");
    writeFileSync(`${archivePath}.sha256`, `${hash}  ${packageName}.zip\n`);
    console.log(archivePath);
  }
}
