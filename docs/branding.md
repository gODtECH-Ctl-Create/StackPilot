# StackPilot CLI identity

StackPilot uses the shared **gODtECH CLI Identity** contract for its terminal identity.

The checked-in files under `identity/` are generated from the canonical `stackpilot` product profile in `gODtECH-Ctl-Create/gODtECH-CLI-Identity` after the scriptless-runtime fix merged in PR #6 (`4cc3bcfeb9090051b426a26c9e8c5d5aed401bd5`). StackPilot does not carry a second GODTECH banner renderer; Rust only selects the generated Unicode or ASCII asset and decides whether decoration is safe for the current invocation.

Behavior:

- interactive human-readable commands show the shared StackPilot identity
- non-TTY output stays decoration-free
- `--non-interactive`, `--help`, and `--version` stay decoration-free
- `STACKPILOT_NO_BANNER=1` explicitly suppresses the banner
- `STACKPILOT_ASCII=1` selects the generated ASCII fallback

The generated profile description is:

`Opinionated project scaffolding for production-minded repositories`
