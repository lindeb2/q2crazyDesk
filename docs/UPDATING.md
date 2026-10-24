# Updating upstream code

How to merge a new RustDesk release and how to update vendored third-party code. Read `docs/UPSTREAM.md` first: its patch list is the set of local edits that can conflict.

## Merging a new RustDesk release

1. **Fetch upstream.**

   ```bash
   git fetch upstream --tags
   git tag --list "1.*" --sort=-v:refname | head
   ```

   Pick the newest stable tag (no `-` suffix), for example `1.5.1`.

2. **Read what changed** in the areas q2crazyDesk touches:

   ```bash
   OLD=1.5.0   # the tag recorded in docs/UPSTREAM.md
   NEW=1.5.1
   git log --oneline $OLD..$NEW | wc -l
   git diff --stat $OLD..$NEW -- src/server/connection.rs src/client src/flutter_ffi.rs src/platform flutter/lib libs/hbb_common
   git diff $OLD..$NEW -- .github/workflows/flutter-build.yml .github/workflows/bridge.yml
   ```

   The last command shows toolchain changes (Rust, Flutter, vcpkg commit, NDK, LLVM, action versions) that must be ported to `.github/workflows/q2-ci.yml`.

3. **Merge on a branch.**

   ```bash
   git checkout -b upstream-$NEW main
   git merge --no-ff $NEW -m "Merge RustDesk $NEW"
   git submodule update --init --recursive
   ```

4. **Resolve conflicts with these rules.**

   - `.github/workflows/*` files deleted in patch U-1 come back as modify/delete conflicts: keep them deleted with `git rm <file>`.
   - `README.md`: keep ours (`git checkout --ours README.md`).
   - Every other conflict is in a file on the patch list: re-apply the q2crazyDesk change on top of the new upstream code. Keep the hook as small as before.
   - If upstream moved or renamed code a hook depends on, update the hook and the patch list.

5. **Port toolchain changes** found in step 2 to `q2-ci.yml` and `docs/DECISIONS.md` (D-003). If the vcpkg commit changed, the `vcpkg.json` baseline came with the merge; the CI cache key includes the commit, so the cache refreshes by itself. Update the local tools on `win1` the same way.

6. **Build and test.**

   ```bash
   cargo test --manifest-path q2/Cargo.toml --workspace
   python build.py --flutter --hwcodec --vram    # local Windows build on win1
   ```

   Push the branch, open a pull request, and wait for `q2crazyDesk CI` to be green on all three targets. Run the smoke subset of `docs/MANUAL_TESTS.md` (the tests marked "smoke").

7. **Record the new version** in `docs/UPSTREAM.md` (tag, commit, submodule commit) and merge the pull request.

## Updating Lan Mouse (git subtree)

Lan Mouse lives in `third_party/lan-mouse` as a squashed `git subtree`.

```bash
git subtree pull --prefix third_party/lan-mouse https://github.com/feschber/lan-mouse.git <tag-or-commit> --squash
```

Then re-check the local modifications listed for Lan Mouse in `docs/UPSTREAM.md`, update the recorded commit, and run the q2 tests.

## Updating ported code

Ported files (from LittleBigMouse, Mouse Without Borders, DeskPad or VirtualDisplayKit, and others) carry a header naming the source repository, commit, path and license. To update one:

1. Diff the upstream file between the recorded commit and the new one.
2. Port the relevant changes by hand.
3. Update the commit in the file header and in `docs/UPSTREAM.md`.

## Updating prebuilt binaries

Prebuilt binaries (for example the signed Virtual-Display-Driver release) are downloaded by CI at a pinned version and verified against a SHA-256 checksum stored in the workflow. To update: change the version and the checksum together, and record both in `docs/UPSTREAM.md`.
