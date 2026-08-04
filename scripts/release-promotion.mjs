const CHECKSUM_FILE = "SHA256SUMS.txt";

function toAssetNameSet(assetNames, label) {
  if (!Array.isArray(assetNames)) {
    throw new Error(`${label} asset names must be an array`);
  }

  const names = new Set();
  for (const name of assetNames) {
    if (typeof name !== "string" || name.length === 0 || name.includes("/") || names.has(name)) {
      throw new Error(`${label} contains an invalid or duplicate asset name`);
    }
    names.add(name);
  }
  if (!names.has(CHECKSUM_FILE)) {
    throw new Error(`${label} is missing ${CHECKSUM_FILE}`);
  }
  return names;
}

function parseChecksums(manifestText, label) {
  if (typeof manifestText !== "string") {
    throw new Error(`${label} checksum manifest must be text`);
  }

  const checksums = new Map();
  for (const line of manifestText.split(/\r?\n/)) {
    if (line.length === 0) continue;
    const match = /^([a-fA-F0-9]{64}) [ *](.+)$/.exec(line);
    if (!match || match[2] === CHECKSUM_FILE || match[2].includes("/") || checksums.has(match[2])) {
      throw new Error(`${label} contains an invalid or duplicate checksum entry`);
    }
    checksums.set(match[2], match[1].toLowerCase());
  }
  if (checksums.size === 0) {
    throw new Error(`${label} has no checksum entries`);
  }
  return checksums;
}

function sameSet(left, right) {
  return left.size === right.size && [...left].every((value) => right.has(value));
}

function manifestMatchesAssets(checksums, assetNames, label) {
  const assetFiles = new Set([...assetNames].filter((name) => name !== CHECKSUM_FILE));
  if (!sameSet(new Set(checksums.keys()), assetFiles)) {
    throw new Error(`${label} checksum entries do not match its asset names`);
  }
}

function sameChecksums(left, right) {
  return (
    left.size === right.size && [...left].every(([name, checksum]) => right.get(name) === checksum)
  );
}

export function validateReleaseAssets({ assetNames, manifestText, label }) {
  const names = toAssetNameSet(assetNames, label);
  const checksums = parseChecksums(manifestText, label);
  manifestMatchesAssets(checksums, names, label);
  return { checksums, names };
}

export function selectReleaseCandidate(runs, sourceSha) {
  if (!Array.isArray(runs) || typeof sourceSha !== "string" || sourceSha.length === 0) {
    throw new Error("Release candidate inputs are invalid");
  }
  const candidates = [];
  for (const run of runs) {
    if (
      !Number.isInteger(run?.id) ||
      !Number.isInteger(run.runAttempt) ||
      !Array.isArray(run.artifacts)
    ) {
      throw new Error("Release candidate run is invalid");
    }
    const expected = `release-candidate-${sourceSha}-${run.id}-${run.runAttempt}`;
    for (const artifact of run.artifacts) {
      if (
        artifact?.name === expected &&
        artifact.expired === false &&
        Number.isInteger(artifact.id)
      ) {
        candidates.push({ artifactId: artifact.id, artifactName: artifact.name, runId: run.id });
      }
    }
  }
  if (candidates.length !== 1) {
    throw new Error(`Expected exactly one unexpired release candidate, found ${candidates.length}`);
  }
  return candidates[0];
}

export function assessExistingRelease({
  candidateAssetNames,
  candidateManifestText,
  existingAssetNames,
  existingManifestText,
}) {
  const candidate = validateReleaseAssets({
    assetNames: candidateAssetNames,
    manifestText: candidateManifestText,
    label: "Candidate release",
  });
  const existing = validateReleaseAssets({
    assetNames: existingAssetNames,
    manifestText: existingManifestText,
    label: "Existing release",
  });
  if (!sameSet(candidate.names, existing.names)) {
    return { equivalent: false, reason: "asset names differ" };
  }

  if (!sameChecksums(candidate.checksums, existing.checksums)) {
    return { equivalent: false, reason: "asset checksums differ" };
  }
  return { equivalent: true };
}
