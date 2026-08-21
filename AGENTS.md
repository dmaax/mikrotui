# Working on MikroTUI

Guidance for anyone — human or agent — changing this codebase. It records the reasoning
behind decisions that are easy to undo by accident, not general Rust advice.

MikroTUI is a read-only terminal UI for MikroTik RouterOS, published on crates.io. It
connects over SSH, verifies the router's host key, and sends only read commands.

---

## Invariants

These are the properties the project exists to hold. Changing one is a deliberate act with
a `BREAKING CHANGE:` footer and a rewritten README section — never a side effect.

**A credential is never sent to an unverified host key.** `SshHandler::check_server_key`
returning `false` aborts before authentication; that ordering is what makes the check
meaningful. `src/ssh/hostkey.rs` decides trust itself rather than delegating to russh's
`check_known_hosts_path`, because that function reports "host on file, but under a
different key algorithm" identically to "never seen this host" — which let an attacker
downgrade a trusted router back to first contact. Only `HostKeyIssue::Unknown` may ever be
resolved by accepting the key.

**Every command is checked before it is sent.** `src/ssh/guard.rs` refuses anything that is
not recognisably read-only. It is an allowlist because a denylist cannot win: RouterOS
accepts prefix abbreviations (`rem` for `remove`), the v7 slash form (`/ip/address/set`),
`;` chaining, and `[ ... ]` command substitution inside argument values. Every one of those
defeated an earlier denylist.

> The guard is a guard rail, not a permission boundary. It runs on the operator's machine,
> so it prevents mistakes; it does not contain someone trying to get past it. Say so
> wherever it is described. The real guarantee is a RouterOS account in the `read` group.

**No password is persisted by default.** Resolution order lives in `src/secrets.rs`. The
`enc:v1:` XOR field in `config.json` is retained for backward compatibility only, is
labelled obfuscation rather than encryption everywhere it appears, and warns on every use.
Do not use it for anything new. Unattended local storage cannot do better: a key the
program recomputes on its own is a key an attacker recomputes on its own.

**Files holding secrets are created at mode 0600**, not chmod-ed afterwards. See
`config::open_private`. The window between the two is the bug.

**The README does not claim more than the code does.** Three separate claims had to be
retracted — a Safe Mode that gated nothing, "encrypted" credentials that were XOR, and a
"Real-time CPU Gauge" with no auto-refresh. If a feature is a guard rail, or opt-in, or not
implemented, the README says that.

---

## Dependency policy

**RSA private keys are refused** (`src/ssh/identity.rs`). This is load-bearing, not
fussiness: CI ignores `RUSTSEC-2023-0071` — the Marvin timing sidechannel, which upstream
has never patched (`patched = []`) — on the stated grounds that MikroTUI holds no RSA
private key. Accepting an RSA identity makes that justification false. If you ever need
RSA, the ignore has to go and the advisory has to be handled on its merits.

**ssh-agent is not linked.** russh's agent client is the code covered by
`RUSTSEC-2026-0154`. Adding agent support reintroduces an advisory the project is currently
clear of.

**The `keyring` feature is off by default.** Its Linux backend needs a running Secret
Service, which headless machines lack, and enabling it unconditionally would break
`cargo install` there. It uses the pure-Rust zbus backend specifically so no `libdbus-1-dev`
or OpenSSL is needed to build. Keep it that way.

**Every `cargo audit` ignore carries a reason and an exit condition**, inline in
`.github/workflows/ci.yml`. An ignore without a stated way out is a permanent hole.

---

## Testing

**A regression test must fail without its fix. Check that it does.** Revert the fix, run the
test, confirm it fails, restore. This is not ceremony — it caught four tests during the
0.3–0.4 work that passed without exercising anything:

- a fabricated SSH key that did not parse, behind `else { return }`, so two host key tests
  silently skipped
- a revert that never applied, because rustfmt had reformatted the code the patch matched
- an assertion that passed through a different guard (`is_loading` masked the timer it was
  supposed to test)
- `if !keygen(...) { return; }` when `ssh-keygen` is absent

**Never skip silently.** A test that cannot run its subject must fail loudly. `expect` on
the missing tool, do not `return`.

**Test behaviour, not implementation, where the behaviour is what broke.** The table
scrolling work renders through `ratatui::backend::TestBackend` and asserts on the buffer,
because the defect was "row 199 is unreachable", not "the offset field is wrong".

Run before pushing — this is exactly what CI runs:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo test --all-targets --features keyring
```

---

## CI

`ci.yml` has three jobs, and one of them exists for a reason worth stating: **cross-build**
compiles macOS and Windows because each platform selects a different credential store
behind `cfg()`, and a Linux-only pipeline never compiles them. It has already caught two
defects that were impossible to reproduce locally — a missing `keychain` feature on the
Apple store, and a `#[cfg(unix)]` placed inside a function body, which left an unused
parameter on Windows.

If a cross-platform failure needs local reproduction, a full cross-compile pulls C
dependencies that may not build. Extracting the affected function into a standalone file
and running `rustc --target x86_64-pc-windows-gnu --emit=metadata -D warnings` on it is
usually enough.

---

## Workflow

Branch, PR, merge commit. Never push to `main`, and never squash: release-plz derives both
the version and the changelog from individual commits, and squashing collapses the
`BREAKING CHANGE:` footers that decide the version.

**Conventional commits, with the body carrying the reasoning.** Commit messages here are
the durable record of *why*; the changelog only ever shows the summary line.

**Versioning is release-plz's job.** Note the 0.x rule, which is not the 1.x rule:

| Commit | 1.x and later | **0.x (here)** |
| :--- | :--- | :--- |
| `BREAKING CHANGE` | major | **minor** |
| `feat` | minor | **patch** |
| `fix` | patch | patch |

Before merging a release PR, check the version it proposes. A breaking change that shows up
as a patch bump means the footer was not parsed, and merging would ship a silent break.

**Do not hand-write `## [Unreleased]` in CHANGELOG.md.** `changelog_update = true` means
release-plz owns that file; hand-written entries end up duplicated.

Merging a release PR **publishes to crates.io and is irreversible** — a version can be
yanked but never removed. Treat it as a separate decision from merging the work.

After publishing, verify from the outside rather than trusting the job log: download the
crate from crates.io and confirm the change is actually in it.

```bash
curl -sL "https://crates.io/api/v1/crates/mikrotui/<version>/download" -o m.crate
tar xzf m.crate && grep -r "<the thing you changed>" mikrotui-<version>/src/
```

---

## Code conventions

English throughout — comments, errors, UI, commit messages.

**Comments explain why, not what.** Prefer noting the failure a piece of code prevents over
restating it. `rustfmt` clean, `clippy -D warnings` clean; both are enforced.

`src/ssh/` holds everything that talks to a router; `src/ui/views/` holds one file per tab,
all sharing `render_scrollable_table`. Selection is read through `App::selection_in(len)`
and nowhere else — the highlighted row and the row acted on diverged once already, and a
single accessor is what stops it recurring.

---

## Manual verification

Some things cannot be unit tested and should be checked by hand when touched.

**A local sshd** covers the host key and authentication paths. For key auth, an isolated
instance on a high port avoids touching a real `~/.ssh/authorized_keys`:

```bash
ssh-keygen -q -t ed25519 -N '' -f "$D/hostkey"
ssh-keygen -q -t ed25519 -N '' -f "$D/client"
cp "$D/client.pub" "$D/authorized_keys"
printf 'Port 2299\nListenAddress 127.0.0.1\nHostKey %s/hostkey\nAuthorizedKeysFile %s/authorized_keys\nPasswordAuthentication no\nStrictModes no\nUsePAM no\n' "$D" "$D" > "$D/sshd_config"
/usr/sbin/sshd -f "$D/sshd_config" -E "$D/sshd.log"
```

Authenticating against a Linux host and getting `bash: /system: No such file or directory`
back is a *success*: the session opened and ran the command.

**The TUI itself** renders under a pty. Note that ratatui only redraws changed cells, so
counting occurrences in captured output does not tell you how often something was set:

```bash
timeout 6 script -qec "stty rows 40 cols 200; ./target/debug/mikrotui --demo" out.log </dev/null
sed -e 's/\x1b\[[0-9;?]*[a-zA-Z]//g' out.log | tr -d '\r' | grep -aoE "Router:[^│]*"
```

`--demo` needs no router. Use it for anything that is not specifically about SSH.
