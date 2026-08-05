# Homebrew Cask Release Notes

This repository can generate an Apple Silicon-only Homebrew Cask after a GitHub Release is published. The Cask uses the validated release candidate's `aio-coding-hub-macos-arm.zip` asset.

The release workflow does not generate or push a Cask. Updating a tap is a separate, manual publication step.

## Tap setup

Create or choose a tap repository. For example:

```bash
gh repo create KNaiFen/homebrew-aio-coding-hub --public
```

The tap repository should contain the generated file at:

```text
Casks/aio-coding-hub.rb
```

No Homebrew tap token or repository variable is required in the source repository because `.github/workflows/release.yml` does not publish to the tap.

## Generate after release

Download the published Apple Silicon ZIP and calculate its SHA-256:

```bash
RELEASE_TAG=aio-coding-hub-v0.60.49
RELEASE_DIR=/tmp/aio-coding-hub-homebrew
TAP_DIR=/absolute/path/to/homebrew-aio-coding-hub

mkdir -p "$RELEASE_DIR"
gh release download "$RELEASE_TAG" \
  --repo KNaiFen/aio-coding-hub \
  --pattern aio-coding-hub-macos-arm.zip \
  --dir "$RELEASE_DIR"

ARM_ZIP_SHA256="$(shasum -a 256 "$RELEASE_DIR/aio-coding-hub-macos-arm.zip" | awk '{print $1}')"

node scripts/support-matrix.mjs homebrew-cask \
  --tag "$RELEASE_TAG" \
  --repo KNaiFen/aio-coding-hub \
  --macos-arm-sha256 "$ARM_ZIP_SHA256" \
  --output "$TAP_DIR/Casks/aio-coding-hub.rb"
```

The formal Release does not include a macOS Intel desktop asset. Do not use the unsigned `dev-build` Intel artifact to populate this Cask.

Validate and then commit the generated file in the tap checkout:

```bash
brew style --cask "$TAP_DIR/Casks/aio-coding-hub.rb"
git -C "$TAP_DIR" add Casks/aio-coding-hub.rb
git -C "$TAP_DIR" commit -m "chore(cask): update AIO Coding Hub to ${RELEASE_TAG#aio-coding-hub-v}"
git -C "$TAP_DIR" push origin HEAD
```

## Release behavior

`.github/workflows/release.yml` promotes an immutable, previously validated release candidate. It verifies and publishes `aio-coding-hub-macos-arm.zip`, but it deliberately does not update an external tap. Generate and push the Cask only after that Release succeeds so its URL and checksum refer to a published asset.

See the [Homebrew Cask Cookbook](https://docs.brew.sh/Cask-Cookbook) for the current Cask syntax and validation rules.
