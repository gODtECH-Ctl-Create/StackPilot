# Releasing StackPilot

StackPilot releases are created from semantic-version Git tags and published by GitHub Actions. The public website is part of the release contract: version, release journey and verified product facts must stay aligned with the CLI.

## Release contract

Use tags in this format only:

```text
vMAJOR.MINOR.PATCH
```

Examples:

```text
v0.1.2
v0.2.0
v1.0.0
```

Do not prefix release tags with `StackPilot-` or create a differently named GitHub release manually. The release workflow uses the Git tag as the source of truth.

The tag version must exactly match the version in `Cargo.toml`. For example, `version = "0.1.2"` must be released with tag `v0.1.2`.

## Public website metadata

Release-facing website data lives in:

```text
website/src/data/site.ts
```

This is the single website source for the current public version, release journey and verified proof points shown across the landing page, navigation, install page and documentation. The website build checks this version against `Cargo.toml` so a stale public version is caught during CI instead of silently shipping.

Only publish statistics that are directly verifiable. Product facts such as supported golden paths, native release targets and completed smoke tests are appropriate. Do not invent download, user, project or adoption counts; add those only when a reliable source exists.

## Normal release procedure

1. Update the package version in `Cargo.toml` and update `Cargo.lock` if Cargo changes it.
2. Update `website/src/data/site.ts` with the same `currentVersion`, append the new release milestone, and refresh any proof point that materially changed.
3. Merge the release changes to `main` and confirm CI is green, including the website build.
4. Create the release tag from the intended `main` commit:

   ```bash
   git checkout main
   git pull --ff-only
   git tag -a v0.1.2 -m "StackPilot v0.1.2"
   git push origin v0.1.2
   ```

5. The `Release` GitHub Actions workflow will automatically:
   - validate the tag against `Cargo.toml`;
   - build native release binaries;
   - package Windows, Linux, and macOS archives;
   - bundle the `recipes` directory in every platform archive;
   - generate `SHA256SUMS`;
   - create the GitHub release;
   - upload all release assets;
   - verify that every expected asset is present.
6. Run the installer smoke tests below. If a newly claimed platform or workflow is verified, update the website proof points in the next documentation commit so the public story stays evidence-based.

## Expected release assets

A successful release must contain:

```text
stackpilot-linux-x86_64.tar.gz
stackpilot-macos-x86_64.tar.gz
stackpilot-macos-aarch64.tar.gz
stackpilot-windows-x86_64.zip
SHA256SUMS
```

Each platform archive must also contain the StackPilot recipe library, including `recipes/base/recipe.toml`. Do not announce a release as installable until all five assets appear on the GitHub release page and the installed CLI can resolve its bundled recipes outside the source repository.

## Retrying a release

The publish step is idempotent. If the release already exists, the workflow uploads the generated assets with replacement enabled rather than failing because the release already exists.

For an existing valid `vX.Y.Z` tag, the workflow can also be run manually from **Actions → Release → Run workflow** and supplied with that existing tag. The workflow checks out the tag itself, so the tag must already exist in the repository.

## Installer smoke tests

After the release finishes, test the public installation paths before announcing it. Run the CLI from a normal user directory that does not contain a local `recipes` folder; this confirms the installed recipe fallback works.

### Windows

```powershell
cd $HOME
irm https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.ps1 | iex
stackpilot --version
stackpilot --help
stackpilot doctor
stackpilot recipes
stackpilot plan smoke-test --recipe base --non-interactive
```

To test a specific release:

```powershell
$env:STACKPILOT_VERSION = "0.1.2"
irm https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.ps1 | iex
```

### Linux / macOS

```bash
cd "$HOME"
curl -fsSL https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.sh | sh
stackpilot --version
stackpilot --help
stackpilot doctor
stackpilot recipes
stackpilot plan smoke-test --recipe base --non-interactive
```

To test a specific release:

```bash
STACKPILOT_VERSION=0.1.2 \
  sh -c "$(curl -fsSL https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.sh)"
```

## Failed or incomplete releases

If a release has no assets, do not treat the installer failure as a client-machine problem. Verify the release workflow and the release tag first.

A `404 Not Found` from an installer normally means the requested platform archive is missing from the GitHub release. A `failed to read recipes directory recipes` error from an installed binary means the release archive or installer did not provide the bundled recipe library correctly.

If a bad release was created with a non-standard tag such as `StackPilot-v0.1.0`, leave it as historical or mark it clearly as superseded, then publish the next corrected version using the standard `vX.Y.Z` tag convention.
