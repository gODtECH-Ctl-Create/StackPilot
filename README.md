<a name="readme-top"></a>

<div align="center">

<img src="./docs/assets/stackpilot-hero.svg" alt="StackPilot animated project scaffolding hero" width="100%" />

<p>
  <a href="https://github.com/gODtECH-Ctl-Create/StackPilot/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/gODtECH-Ctl-Create/StackPilot/ci.yml?branch=main&style=for-the-badge&label=CI" alt="CI status" /></a>
  <img src="https://img.shields.io/badge/version-0.1.0-8d91ff?style=for-the-badge" alt="StackPilot version 0.1.0" />
  <img src="https://img.shields.io/badge/license-MIT-8bffb0?style=for-the-badge" alt="MIT license" />
  <img src="https://img.shields.io/badge/core-Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust core" />
  <img src="https://img.shields.io/badge/golden_paths-6-11172a?style=for-the-badge" alt="Six golden paths" />
</p>

### Opinionated project scaffolding for production-minded repositories.

<img src="https://readme-typing-svg.demolab.com?font=JetBrains+Mono&size=18&duration=2400&pause=750&color=8BFFB0&center=true&vCenter=true&width=1000&lines=Choose+a+language.+Get+the+golden+path.;Docker+%2B+CI+%2B+Terraform+without+the+setup+drift.;Rust+%7C+Go+%7C+TypeScript+%7C+Python+%7C+Java+%7C+C%23;Preview+the+plan.+Generate+the+repo.+Start+building." alt="Animated StackPilot capabilities" />

<p>
  <a href="https://godtech-ctl-create.github.io/StackPilot/">Website</a> ·
  <a href="#-quick-start">Quick start</a> ·
  <a href="#-golden-paths">Golden paths</a> ·
  <a href="#-architecture">Architecture</a> ·
  <a href="#-development">Development</a>
</p>

</div>

---

## ⚡ The 30-second version

**StackPilot** is a Rust-powered GitHub template and project scaffolding engine. Instead of asking you to choose from hundreds of framework combinations, it provides a deliberately opinionated **golden path** for each supported language and keeps generated repositories structurally consistent.

```text
LANGUAGE
   ↓
GOLDEN FRAMEWORK
   ↓
PRODUCTION DEFAULTS
   ↓
RECIPE + VALIDATION
   ↓
READY-TO-BUILD REPOSITORY
```

For deployable services, StackPilot defaults toward **PostgreSQL, AWS, Docker, CI, and Terraform**. Every generated backend service begins from the same contract: `/health`, port `3000`, container support, language-native CI, environment metadata, and optional Terraform infrastructure.

> **v0.1 focus:** backend services and APIs are the validated golden paths. Frontend, full-stack, workers, CLI apps, and libraries remain roadmap targets.

<a href="#readme-top">↑ back to top</a>

---

## 🧭 Golden paths

StackPilot recommends one production-minded framework per supported ecosystem.

| Language | Golden path | Typical output |
| --- | --- | --- |
| 🦀 Rust | **Axum** | Backend API / service |
| 🐹 Go | **Chi** | Backend API / service |
| 🟦 TypeScript | **NestJS** | Backend API / service |
| 🐍 Python | **FastAPI** | Backend API / service |
| ☕ Java | **Spring Boot** | Backend API / service |
| 🟣 C# | **ASP.NET Core** | Backend API / service |

<p align="center">
  <img src="https://skillicons.dev/icons?i=rust,go,ts,python,java,cs,docker,terraform,aws,postgres&perline=10" alt="StackPilot supported technology icons" />
</p>

The normal workflow recommends instead of overwhelming. Automation can still override supported infrastructure choices when required.

---

## 🚀 Quick start

### GitHub template flow

The lowest-friction path is GitHub Actions:

```text
01  Use this template
02  Open Actions → Configure StackPilot Template
03  Choose a language
04  Adjust infrastructure only when necessary
05  Run the workflow
06  Review the generated repository
```

StackPilot previews the plan, generates the selected golden path, commits the result, persists the choices in `.stackpilot.toml`, and removes the template-engine source files.

For local interactive setup after cloning:

```bash
stackpilot bootstrap
```

When running directly from source:

```bash
cargo run -- bootstrap
```

Keep the engine while testing bootstrap:

```bash
stackpilot bootstrap --keep-engine
```

### Create a separate project

Interactive:

```bash
stackpilot new
```

Named project:

```bash
stackpilot new payment-service --recipe base
```

Preview before writing anything:

```bash
stackpilot plan payment-service \
  --non-interactive \
  --kind "Backend API" \
  --language Go \
  --framework Auto \
  --database PostgreSQL \
  --cloud AWS \
  --docker true \
  --ci true \
  --terraform true
```

Then generate the same plan:

```bash
stackpilot new payment-service \
  --recipe base \
  --non-interactive \
  --kind "Backend API" \
  --language Go \
  --framework Auto \
  --database PostgreSQL \
  --cloud AWS \
  --docker true \
  --ci true \
  --terraform true
```

---

## 🌀 How StackPilot works

```mermaid
flowchart LR
    A[Project intent] --> B[StackPilot CLI]
    B --> C{Language}
    C -->|Rust| D[Axum]
    C -->|Go| E[Chi]
    C -->|TypeScript| F[NestJS]
    C -->|Python| G[FastAPI]
    C -->|Java| H[Spring Boot]
    C -->|C#| I[ASP.NET Core]

    D --> J[Recipe renderer]
    E --> J
    F --> J
    G --> J
    H --> J
    I --> J

    J --> K[Validation + safety]
    K --> L[Generated repository]
    L --> M[App]
    L --> N[Docker]
    L --> O[CI]
    L --> P[Terraform]
```

The engine stays independent from generated project languages. Recipes own stack-specific output; the Rust core owns selection, validation, rendering, repository operations, safety, and lifecycle behavior.

---

## ✨ What you get

| Area | StackPilot provides |
| --- | --- |
| **Scaffolding** | Native Rust CLI, template bootstrap, interactive and non-interactive project generation |
| **Opinionated defaults** | `Auto` framework resolution, PostgreSQL-first database profile, AWS-first cloud profile |
| **Delivery** | Docker policy, Compose support, language-native CI, optional Terraform foundation |
| **Recipes** | TOML manifests, MiniJinja rendering, compound conditional recipe files |
| **Safety** | Non-destructive planning, path-safe generation, transactional cleanup on failure |
| **Repository lifecycle** | Git initialization, `.stackpilot.toml` project profile, template-engine cleanup |
| **Quality** | Recipe diagnostics plus CI compatibility coverage across every golden path |
| **Distribution** | Cross-platform release workflow and one-command installer scripts |

Supported alternatives include **MySQL, MongoDB, SQLite, or no database**, plus **Azure, GCP, or no cloud** where the recipe supports them.

---

## 🧠 Architecture

```mermaid
graph TD
    U[Developer / GitHub Actions] --> CLI[StackPilot CLI]

    CLI --> SEL[Opinionated selector]
    CLI --> VAL[Validation + safety]
    CLI --> OPS[Repository operations]

    SEL --> REC[Recipe manifests]
    REC --> MJ[MiniJinja renderer]
    MJ --> OUT[Generated project]

    VAL --> OUT
    OPS --> OUT

    OUT --> APP[Application code]
    OUT --> DOC[Docker / Compose]
    OUT --> CI[Language-native CI]
    OUT --> INF[Terraform infrastructure]
    OUT --> CFG[.stackpilot.toml]
```

### Generated service contract

```text
/health              health endpoint
:3000                default application port
Docker               container-ready output
CI                   language-native validation/build pipeline
.stackpilot.toml      persisted project profile
Terraform            optional infrastructure foundation
```

---

## 🧰 Command map

| Command | Purpose |
| --- | --- |
| `stackpilot bootstrap` | Turn the current template repository into the selected project |
| `stackpilot new` | Generate a separate project |
| `stackpilot plan` | Preview exactly what would be generated without writing files |
| `stackpilot recipes` | Discover available recipes |
| `stackpilot doctor` | Run StackPilot diagnostics |

---

## 📦 Installation

Tagged releases are configured to publish native binaries for **Linux x86_64, macOS Intel, macOS Apple Silicon, and Windows x86_64**.

Linux / macOS installer:

```bash
curl -fsSL https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.ps1 | iex
```

If a tagged binary release is not yet available, run StackPilot from source:

```bash
git clone https://github.com/gODtECH-Ctl-Create/StackPilot.git
cd StackPilot
cargo run -- --help
```

---

## 🧪 Development

Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

StackPilot's CI also validates that every supported golden path can be generated and built consistently.

---

## 📄 License

StackPilot is available under the **MIT License**. See [`LICENSE`](./LICENSE).

<div align="center">

**Build from a golden path. Spend your time on the product.**

<a href="#readme-top">↑ back to top</a>

</div>
