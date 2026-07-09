import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const runbookPath = path.join(
  repoRoot,
  "docs",
  "releases",
  "signed-binary-manifest-bootstrap-runbook.md",
);
const releaseReadmePath = path.join(repoRoot, "docs", "releases", "README.md");
const installDocPath = path.join(repoRoot, "docs", "INSTALL.md");
const installShPath = path.join(repoRoot, "scripts", "install.sh");
const installPs1Path = path.join(repoRoot, "scripts", "install.ps1");
const artifactRoot = path.join(repoRoot, "artifacts", "release-manifest-bootstrap-runbook");
const evidencePath =
  process.env.AI_RELEASE_MANIFEST_BOOTSTRAP_EVIDENCE_PATH ||
  path.join(artifactRoot, "release-manifest-bootstrap-runbook.json");

const runbook = await readFile(runbookPath, "utf8");
const releaseReadme = await readFile(releaseReadmePath, "utf8");
const installDoc = await readFile(installDocPath, "utf8");
const installSh = await readFile(installShPath, "utf8");
const installPs1 = await readFile(installPs1Path, "utf8");
const normalizedRunbook = runbook.replace(/\s+/g, " ").trim();

const requiredSections = [
  "## Scope",
  "## Trust Model",
  "## Selected Bootstrap Path",
  "## Supported Paths",
  "## Linux Or WSL Fresh Install",
  "## Windows Fresh Install",
  "## Managed Update",
  "## Downgrade Prevention",
  "## Operator Checklist",
  "## Failure Handling",
  "## Completion Criteria",
];

const requiredRunbookPhrases = [
  "AI_REQUIRE_SIGNED_MANIFEST=1",
  "AI_TERMINAL_ORG_TRUST_ANCHOR",
  "AI_MANIFEST_VERIFIER",
  "AI_MIN_MANIFEST_VERSION",
  ".ai-terminal-release-manifest-version",
  "managed verifier bundle",
  "MDM, a golden image, or an internal package manager",
  "fresh strict installs should always pass `AI_MANIFEST_VERIFIER`",
  "must never use the just-downloaded `ai` binary",
  "private release signing key never ships to clients",
  "checksum-compatible",
  "Manifest downgrade",
  "npm run check:release-manifest-bootstrap",
];

for (const section of requiredSections) {
  assert.ok(runbook.includes(section), `runbook missing section ${section}`);
}

for (const phrase of requiredRunbookPhrases) {
  assert.ok(
    normalizedRunbook.includes(phrase.replace(/\s+/g, " ").trim()),
    `runbook missing phrase ${phrase}`,
  );
}

assert.ok(
  releaseReadme.includes("signed-binary-manifest-bootstrap-runbook.md") &&
    releaseReadme.includes("npm run check:release-manifest-bootstrap"),
  "release docs index must link bootstrap runbook and quick check",
);

assert.ok(
  installDoc.includes("signed-binary-manifest-bootstrap-runbook.md") &&
    installDoc.includes("AI_REQUIRE_SIGNED_MANIFEST=1") &&
    installDoc.includes("AI_MANIFEST_VERIFIER") &&
    installDoc.includes("새로 내려받은 `ai`로 자기 자신을 검증하지 않는다"),
  "INSTALL.md must point strict installs at the bootstrap runbook and keep self-verification warning",
);

for (const marker of [
  "AI_REQUIRE_SIGNED_MANIFEST",
  "AI_MANIFEST_VERIFIER",
  "AI_MIN_MANIFEST_VERSION",
  ".ai-terminal-release-manifest-version",
]) {
  assert.ok(installSh.includes(marker), `install.sh missing ${marker}`);
  assert.ok(installPs1.includes(marker), `install.ps1 missing ${marker}`);
}

assert.ok(
  !/AI_MANIFEST_VERIFIER=.*(tmp|download|candidate)/i.test(runbook),
  "runbook must not suggest using a downloaded candidate as verifier",
);

const evidence = {
  status: "ok",
  generatedAt: new Date().toISOString(),
  objective:
    "Verify the signed binary manifest strict fresh-install bootstrap runbook and documentation links",
  runbookPath,
  checks: {
    requiredSections,
    requiredRunbookPhrases,
    releaseDocsLinked: true,
    installDocsLinked: true,
    installScriptsExposeStrictVariables: true,
    selectedVerifierDistributionPath: "managed-verifier-bundle",
    freshInstallSelfVerificationProhibited: true,
  },
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`AI_RELEASE_MANIFEST_BOOTSTRAP_RUNBOOK_OK ${evidencePath}`);
