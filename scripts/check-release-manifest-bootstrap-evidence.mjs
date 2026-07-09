import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const defaultInputPath = path.join(
  repoRoot,
  "artifacts",
  "release-manifest-bootstrap-external",
  "evidence.json",
);
const artifactRoot = path.join(repoRoot, "artifacts", "release-manifest-bootstrap-evidence");
const defaultOutputPath = path.join(artifactRoot, "release-manifest-bootstrap-evidence.json");

const args = process.argv.slice(2);
let inputPath = process.env.AI_RELEASE_MANIFEST_BOOTSTRAP_EXTERNAL_EVIDENCE || defaultInputPath;
let outputPath = process.env.AI_RELEASE_MANIFEST_BOOTSTRAP_EVIDENCE_STATUS || defaultOutputPath;
let failOnBlocked = false;
let allowSample = false;

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === "--input") {
    inputPath = path.resolve(args[++index] ?? "");
  } else if (arg === "--output") {
    outputPath = path.resolve(args[++index] ?? "");
  } else if (arg === "--fail-on-blocked") {
    failOnBlocked = true;
  } else if (arg === "--allow-sample") {
    allowSample = true;
  } else {
    throw new Error(`Unknown argument: ${arg}`);
  }
}

function normalizePathLike(value) {
  return String(value ?? "").replaceAll("\\", "/").toLowerCase();
}

function isPositiveInteger(value) {
  return Number.isInteger(value) && value > 0;
}

function hasDownloadedCandidateMarker(value) {
  return /(download|candidate|tmp|temp|release-under-install)/i.test(String(value ?? ""));
}

function collectProhibitedEvidenceKeys(value, prefix = "") {
  if (!value || typeof value !== "object") {
    return [];
  }

  const prohibited = [];
  for (const [key, child] of Object.entries(value)) {
    const currentPath = prefix ? `${prefix}.${key}` : key;
    const normalizedKey = key.replace(/[_\-\s]/g, "").toLowerCase();
    if (
      normalizedKey.includes("privatekey") ||
      normalizedKey.includes("signingkeyhex") ||
      normalizedKey.includes("signingsecret") ||
      normalizedKey.includes("password") ||
      normalizedKey.includes("token") ||
      normalizedKey === "secret" ||
      normalizedKey.endsWith("secret")
    ) {
      prohibited.push(currentPath);
    }
    prohibited.push(...collectProhibitedEvidenceKeys(child, currentPath));
  }
  return prohibited;
}

function validateReadyEvidence(evidence) {
  const errors = [];
  const allowedDistributions = new Set(["mdm", "golden-image", "internal-package-manager"]);

  if (evidence.sample === true && !allowSample) {
    errors.push("sample evidence requires --allow-sample and cannot close production evidence");
  }
  if (evidence.status !== "ready") {
    errors.push("status must be ready");
  }
  if (typeof evidence.releaseTag !== "string" || !/^v\d+\.\d+\.\d+/.test(evidence.releaseTag)) {
    errors.push("releaseTag must be a semver tag such as v0.3.5");
  }
  if (evidence.releaseAssets?.binaryManifestJson !== true) {
    errors.push("releaseAssets.binaryManifestJson must be true");
  }
  if (evidence.releaseAssets?.binaryManifestTrustManifestJson !== true) {
    errors.push("releaseAssets.binaryManifestTrustManifestJson must be true");
  }
  if (!isPositiveInteger(evidence.manifest?.version)) {
    errors.push("manifest.version must be a positive integer");
  }
  if (typeof evidence.manifest?.keyId !== "string" || !evidence.manifest.keyId) {
    errors.push("manifest.keyId is required");
  }
  if (!allowedDistributions.has(evidence.verifier?.distribution)) {
    errors.push("verifier.distribution must be mdm, golden-image, or internal-package-manager");
  }
  if (evidence.verifier?.preinstalled !== true) {
    errors.push("verifier.preinstalled must be true");
  }
  if (evidence.verifier?.notFromReleaseUnderInstall !== true) {
    errors.push("verifier.notFromReleaseUnderInstall must be true");
  }
  if (typeof evidence.verifier?.path !== "string" || !evidence.verifier.path) {
    errors.push("verifier.path is required");
  }
  if (hasDownloadedCandidateMarker(evidence.verifier?.path)) {
    errors.push("verifier.path must not point at a download/tmp/candidate location");
  }
  if (evidence.trustAnchor?.preinstalled !== true) {
    errors.push("trustAnchor.preinstalled must be true");
  }
  if (typeof evidence.trustAnchor?.path !== "string" || !evidence.trustAnchor.path) {
    errors.push("trustAnchor.path is required");
  }
  if (!isPositiveInteger(evidence.trustAnchor?.minVersion)) {
    errors.push("trustAnchor.minVersion must be a positive integer");
  }
  if (evidence.strictEnv?.AI_REQUIRE_SIGNED_MANIFEST !== "1") {
    errors.push("strictEnv.AI_REQUIRE_SIGNED_MANIFEST must be \"1\"");
  }
  if (typeof evidence.strictEnv?.AI_MANIFEST_VERIFIER !== "string") {
    errors.push("strictEnv.AI_MANIFEST_VERIFIER is required");
  }
  if (typeof evidence.strictEnv?.AI_TERMINAL_ORG_TRUST_ANCHOR !== "string") {
    errors.push("strictEnv.AI_TERMINAL_ORG_TRUST_ANCHOR is required");
  }
  if (
    normalizePathLike(evidence.strictEnv?.AI_MANIFEST_VERIFIER) !==
    normalizePathLike(evidence.verifier?.path)
  ) {
    errors.push("strictEnv.AI_MANIFEST_VERIFIER must match verifier.path");
  }
  if (
    normalizePathLike(evidence.strictEnv?.AI_TERMINAL_ORG_TRUST_ANCHOR) !==
    normalizePathLike(evidence.trustAnchor?.path)
  ) {
    errors.push("strictEnv.AI_TERMINAL_ORG_TRUST_ANCHOR must match trustAnchor.path");
  }
  if (evidence.install?.exitCode !== 0) {
    errors.push("install.exitCode must be 0");
  }
  if (evidence.install?.usedJustDownloadedVerifier !== false) {
    errors.push("install.usedJustDownloadedVerifier must be false");
  }
  if (!isPositiveInteger(evidence.install?.recordedManifestVersion)) {
    errors.push("install.recordedManifestVersion must be a positive integer");
  }
  if (evidence.install?.recordedVersionFile !== ".ai-terminal-release-manifest-version") {
    errors.push("install.recordedVersionFile must be .ai-terminal-release-manifest-version");
  }
  if (evidence.downgradeGuard?.recordedVersionFilePresent !== true) {
    errors.push("downgradeGuard.recordedVersionFilePresent must be true");
  }
  if (evidence.downgradeGuard?.lowerVersionRejected !== true) {
    errors.push("downgradeGuard.lowerVersionRejected must be true");
  }

  const manifestVersion = evidence.manifest?.version;
  const recordedVersion = evidence.install?.recordedManifestVersion;
  const minVersion = evidence.trustAnchor?.minVersion;
  if (isPositiveInteger(manifestVersion) && isPositiveInteger(recordedVersion) && recordedVersion !== manifestVersion) {
    errors.push("install.recordedManifestVersion must equal manifest.version");
  }
  if (isPositiveInteger(manifestVersion) && isPositiveInteger(minVersion) && manifestVersion < minVersion) {
    errors.push("manifest.version must be >= trustAnchor.minVersion");
  }

  const prohibitedKeys = collectProhibitedEvidenceKeys(evidence);
  if (prohibitedKeys.length > 0) {
    errors.push(`evidence must not include private key, token, password, or secret fields: ${prohibitedKeys.join(", ")}`);
  }

  return errors;
}

async function main() {
  let evidence;
  try {
    evidence = JSON.parse(await readFile(inputPath, "utf8"));
  } catch (error) {
    if (error?.code !== "ENOENT") {
      throw error;
    }
    const blocked = {
      status: "blocked",
      generatedAt: new Date().toISOString(),
      objective: "Check external organization evidence for strict signed binary manifest bootstrap",
      inputPath,
      blockedItems: ["externalBootstrapEvidence"],
      nextActions: [
        "Run the strict installer on an organization-managed host with a preinstalled verifier bundle",
        "Record evidence.json using docs/releases/signed-binary-manifest-bootstrap-evidence.sample.json as the shape",
        "Re-run npm run check:release-manifest-bootstrap-evidence",
      ],
    };
    await mkdir(path.dirname(outputPath), { recursive: true });
    await writeFile(outputPath, `${JSON.stringify(blocked, null, 2)}\n`);
    console.log(`AI_RELEASE_MANIFEST_BOOTSTRAP_EVIDENCE_BLOCKED ${outputPath}`);
    process.exitCode = failOnBlocked ? 2 : 0;
    return;
  }

  const errors = validateReadyEvidence(evidence);
  const status = errors.length === 0 ? "ready" : "blocked";
  const result = {
    status,
    generatedAt: new Date().toISOString(),
    objective: "Check external organization evidence for strict signed binary manifest bootstrap",
    inputPath,
    releaseTag: evidence.releaseTag,
    manifestVersion: evidence.manifest?.version,
    verifierDistribution: evidence.verifier?.distribution,
    checks: {
      manifestAssetsPresent: errors.every((error) => !error.startsWith("releaseAssets.")),
      verifierPreinstalled: evidence.verifier?.preinstalled === true,
      verifierNotFromReleaseUnderInstall: evidence.verifier?.notFromReleaseUnderInstall === true,
      trustAnchorPreinstalled: evidence.trustAnchor?.preinstalled === true,
      strictModeEnabled: evidence.strictEnv?.AI_REQUIRE_SIGNED_MANIFEST === "1",
      installSucceeded: evidence.install?.exitCode === 0,
      selfVerificationProhibited: evidence.install?.usedJustDownloadedVerifier === false,
      downgradeGuardReady: evidence.downgradeGuard?.lowerVersionRejected === true,
      noPrivateEvidenceMaterial: collectProhibitedEvidenceKeys(evidence).length === 0,
    },
    blockedItems: errors,
  };

  await mkdir(path.dirname(outputPath), { recursive: true });
  await writeFile(outputPath, `${JSON.stringify(result, null, 2)}\n`);

  if (status === "ready") {
    assert.equal(errors.length, 0);
    console.log(`AI_RELEASE_MANIFEST_BOOTSTRAP_EVIDENCE_READY ${outputPath}`);
  } else {
    console.log(`AI_RELEASE_MANIFEST_BOOTSTRAP_EVIDENCE_BLOCKED ${outputPath}`);
    process.exitCode = failOnBlocked ? 2 : 0;
  }
}

await main();
