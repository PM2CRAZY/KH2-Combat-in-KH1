# Working with several sessions

Up to four Claude chats can work on this repo at once: **one lead and at most three helpers**.
This page is the protocol that stops them colliding.

## The pieces

- **Integration copy:** the repo on your PC (e.g. `C:\source\kh2-combat`).
  - Its `main` is the only shared truth.
  - Every session works in its own cloud copy and delivers to it by git bundle.
- **Board:** `local/board/` in that repo. It is ignored by git, so it never conflicts.
  - `claims.md`: who is editing what (see Claims).
  - `log.md`: append-only notes for sessions that aren't running right now.
  - `main.lock/`: a directory that exists only while someone is merging into `main`.
- **Live messages:** sessions on the same PC can message each other by name. `ListAgents`
  shows who is running; send with `SendMessage`. Use these for "I need X from your lane" and
  "merged, please rebase".

## Lanes

Each helper owns one lane. Only the owner edits a lane's files.

| Lane | Owns |
|---|---|
| **core** | `crates/character-core`, `crates/combat-core`, `crates/map-collision`, `crates/kh2-camera` |
| **data** | `crates/kh2-formats`, `crates/kh2-data`, `crates/kh2-bdx`, `crates/kh2-magic`, `crates/kh2-combat-cli` |
| **play** | `crates/kh2-bevy` (the playground) |
| **lead** (this first session) | Ghidra/RE notes (it has the Ghidra setup), the `docs/*-model.md` files, and the shared files: `Cargo.toml`, `Cargo.lock`, `README.md`, `AGENTS.md`, `docs/ROADMAP.md`, `CREDITS.md`, `NOTICE`, `.gitignore` |

**Needing a change in someone else's lane.** Message the owner and say what you need: an API,
a field, a fix. If the owner isn't running, claim the file on the board first (below), keep
the change small, and say so in `log.md`.

**New crate or dependency.** Ask the lead. It edits `Cargo.toml`, and only the lead commits
`Cargo.lock`. Helpers leave `Cargo.lock` out of their commits; it is regenerated after merging.

## Claims

Before editing any file outside your lane (or a shared file), add a line to
`local/board/claims.md`:

```
<session name> | <lane> | <file or crate> | <what for> | <date time>
```

- Remove the line when you deliver.
- If a line names your file, message that session and wait for an answer. A claim older than
  12 hours with no running session may be taken over; note it in `log.md`.

## Delivering to `main`

Small, frequent deliveries. Each one builds and passes `cargo test`.

1. Bring your copy up to date with the PC's `main` (bundle it on the PC, fetch it here) and
   rebase your commits onto it. Run `cargo build` and `cargo test`.
2. **Take the lock:** `mkdir local/board/main.lock`. If it fails, someone is merging: wait
   and retry. `mkdir` is atomic.
3. Copy your bundle in, then `git fetch <bundle> +HEAD:refs/heads/lane/<lane>` and
   `git merge --ff-only lane/<lane>`.
   - If the fast-forward fails, `main` moved: release the lock and go back to step 1.
4. **Release:** `rmdir local/board/main.lock`.
   - Append a line to `log.md`: what changed and whether the others need to rebase.
   - Message the running sessions.
5. Never force-push, rewrite `main`, or merge with conflicts on the PC.
6. Only the session that took the lock releases it. In a one-line script, release inside the
   branch that took it (`if mkdir local/board/main.lock; then ...; rmdir local/board/main.lock;
   fi`): `mkdir ... && ...; rmdir ...` deletes another session's lock when `mkdir` fails
   (claude-8a's, during its merge on 2026-10-11).

## Cost

- **Helpers start lean:** read only your lane's crates and this page, not the whole repo.
- **Subagents:** at most 3 per session, and only for fan-out searches or Ghidra digs. Give
  them a cheaper model when the job is mechanical.
- **Playground screenshots under Xvfb are slow** (about 4 frames a second). Keep scenes to
  about 200 frames, and run several in one shell call.
- **Extracted game files** are in `local/re` on the PC. Stage only the ones you need. Its
  `.mdlx` files came from a modded install and are not the originals (the forms carry Sora's
  collision table): for a model, extract it again from the Steam install with the CLI's
  `extract` (`local/re-fresh` has the player and form models, `docs/drive-model.md`).
