---
title: "Missing Homebrew tap step and absent pre-commit validation hooks"
date: "2026-03-11"
category: "setup-and-dx-issues"
tags:
  - homebrew-installation
  - pre-commit-hooks
  - developer-experience
  - ci-cd
  - lefthook
severity: medium
component: "README, CONTRIBUTING, developer workflow"
symptoms:
  - "Users following README could not find explicit brew tap step for Homebrew installation"
  - "Contributors had no local validation; CI caught issues only after push"
  - "No documented development setup guidance for contributors"
root_cause_summary: |
  Installation instructions used shorthand syntax (brew install org/repo/pkg) without
  documenting the required brew tap step. Pre-commit validation was enforced only in CI
  (6 checks) without local hooks, creating friction for contributors.
---

# Missing Homebrew tap step and absent pre-commit validation hooks

## Problem

### 1. Incomplete Homebrew Installation Instructions

README.md only had the shorthand form:

```bash
brew install hamsurang/velog-cli/velog-cli
```

This implicit tap syntax works but is non-standard. The explicit `brew tap` step was missing, causing confusion for users unfamiliar with Homebrew's shorthand resolution. Both English and Korean sections had the same issue.

### 2. No Pre-commit Hooks

CI pipeline runs 6 checks (fmt-check, clippy, test, typos, audit, ls-lint), but contributors had zero local pre-commit hooks. Issues were only discovered after pushing, wasting CI time and creating contributor friction.

## Root Cause

- **brew tap**: Documentation omitted the prerequisite step, relying on Homebrew's implicit tap resolution
- **Pre-commit hooks**: No git hooks framework was installed. The project had no `husky`, `lefthook`, or `pre-commit` configuration

## Solution

### 1. Explicit Homebrew Two-Step Installation

Changed both English and Korean README sections:

```bash
# Before
brew install hamsurang/velog-cli/velog-cli

# After
brew tap hamsurang/velog-cli
brew install velog-cli
```

### 2. lefthook Pre-commit Hooks

Created `lefthook.yml` with 4 parallel pre-commit hooks:

```yaml
pre-commit:
  parallel: true
  commands:
    fmt-check:
      run: cargo fmt-check
      fail_text: "Formatting error. Run `cargo fmt --all` to fix."
    clippy:
      run: cargo lint
      fail_text: "Clippy warning. Run `cargo lint` to check."
    typos:
      run: typos
      fail_text: "Typo found. Run `typos -w` to auto-fix."
    test:
      run: cargo test
      fail_text: "Test failed. Run `cargo test` to check."
```

**Tool choice**: lefthook (Go binary, zero dependencies) over husky (Node.js) or pre-commit (Python) — avoids adding unnecessary runtimes to a Rust project.

**Excluded from hooks**:
- `audit`: Network-dependent, runs independently of code changes
- `ls-lint`: Requires Node.js (npx), incompatible with zero-dependency goal

### 3. Documentation Updates

- **README.md**: Added "Development Setup" / "기여하기" sections (English + Korean) with lefthook and typos-cli install instructions
- **CONTRIBUTING.md**: Added lefthook tip and `--no-verify` escape hatch for WIP commits

## Files Changed

| File | Change |
|------|--------|
| `lefthook.yml` | Created — pre-commit hook configuration |
| `README.md` | Updated — brew tap 2-step, Contributing sections (EN + KO) |
| `CONTRIBUTING.md` | Updated — lefthook tip added |

## Prevention Strategies

### CI/Lefthook Parity

When adding a new CI check to `.github/workflows/ci.yml`:
1. Assess: "Should developers catch this locally?"
2. If yes, add to `lefthook.yml` with the same command
3. Update README Contributing section
4. Document CI-only exceptions (audit, ls-lint) with rationale

### Documentation Checklist

When modifying installation instructions:
- [ ] Test exact command sequence in a clean shell session
- [ ] Document all prerequisites before the install command
- [ ] Update both English and Korean sections
- [ ] Verify code blocks are identical across languages (only prose differs)

### When Updating lefthook.yml

- [ ] Verify CI equivalence in `.github/workflows/ci.yml`
- [ ] Test with `lefthook run pre-commit`
- [ ] Ensure `fail_text` provides clear fix instructions
- [ ] Measure commit time impact (target: <30s)

## Cross-References

- **Brainstorm**: [docs/brainstorms/2026-03-11-readme-brew-tap-and-lefthook-brainstorm.md](../brainstorms/2026-03-11-readme-brew-tap-and-lefthook-brainstorm.md)
- **Plan**: [docs/plans/2026-03-11-feat-readme-brew-tap-and-lefthook-plan.md](../plans/2026-03-11-feat-readme-brew-tap-and-lefthook-plan.md)
- **CI workflow**: `.github/workflows/ci.yml`
- **Cargo aliases**: `.cargo/config.toml` (fmt-check, lint)
- **Homebrew tap repo**: `hamsurang/homebrew-velog-cli`
