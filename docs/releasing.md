# Releasing StackPilot

StackPilot releases are created from semantic-version Git tags and published by GitHub Actions.

## Release contract

Use tags in this format only:

```text
vMAJOR.MINOR.PATCH
```

Examples:

```text
v0.1.1
v0.2.0
v1.0.0
```

Do not prefix release tags with `StackPilot-` or create a differently named GitHub release manually. The release workflow uses the Git tag as the source of truth.

The tag version must exactly match the version in `Cargo.toml`. For example, `version = "0.1.1"` must be released with tag `v0.1.1`.

## Normal release procedure

1. Update the package version in `Cargo.toml` and update `Cargo.lock` if Cargo changes it.
2. Merge the version change to `main` and confirm CI is green.
3. Create the release tag from the intended `main` commit:

   ```bash
   git checkout main
   git pull --ff-only
   git tag -a v0.1.1 -m "StackPilot v0.1.1"
   git push origin v0.1.1
   ```

4. The `Release` GitHub Actions workflow will automatically:
   - validate the tag against `Cargo.toml`;
   - build native release binaries;
   - package Windows, Linux, and macOS archives;
   - generate `SHA256SUMS`;
   - create the GitHub release;
   - upload all release assets;
   - verify that every expected asset is present.

## Expected release assets

A successful release must contain:

```text
stackpilot-linux-x86_64.tar.gz
stackpilot-macos-x86_64.tar.gz
stackpilot-macos-aarch64.tar.gz
stackpilot-windows-x86_64.zip
SHA256SUMS
```

Do not announce a release as installable until all five assets appear on the GitHub release page.

## Retrying a release

The publish step is idempotent. If the release already exists, the workflow uploads the generated assets with replacement enabled rather than failing because the release already exists.

For an existing valid `vX.Y.Z` tag, the workflow can also be run manually from **Actions → Release → Run workflow** and supplied with that existing tag. The workflow checks out the tag itself, so the tag must already exist in the repository.

## Installer smoke tests

After the release finishes, test the public installation paths before announcing it.

### Windows

```powershell
irm https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.ps1 | iex
stackpilot --help
stackpilot doctor
stackpilot recipes
```

To test a specific release:

```powershell
$env:STACKPILOT_VERSION = "0.1.1"
irm https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.ps1 | iex
```

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.sh | sh
stackpilot --help
stackpilot doctor
stackpilot recipes
```

To test a specific release:

```bash
STACKPILOT_VERSION=0.1.1 \
  sh -c "$(curl -fsSL https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.sh)"
```

## Failed or incomplete releases

If a release has no assets, do not treat the installer failure as a client-machine problem. Verify the release workflow and the release tag first.

A `404 Not Found` from an installer normally means the requested platform archive is missing from the GitHub release.

If a bad release was created with a non-standard tag such as `StackPilot-v0.1.0`, leave it as historical or mark it clearly as superseded, then publish the next corrected version using the standard `vX.Y.Z` tag convention.
