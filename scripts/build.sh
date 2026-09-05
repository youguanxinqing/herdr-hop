#!/usr/bin/env sh
# Build from source. Prebuilt release assets can replace this once CI publishes them;
# until then every install compiles with cargo.
set -eu
cargo build --release
mkdir -p bin
# Stage via rename onto a fresh inode: overwriting bin/herdr-hop in place poisons the vnode's
# cached code signature on macOS, and execs from it are then killed with SIGKILL (Code Signature
# Invalid) until the cache recovers. Inherited from herdr-flash; see that repo's
# docs/bugs/build-overwrite-codesign-kill.md.
cp target/release/herdr-hop bin/herdr-hop.tmp
mv -f bin/herdr-hop.tmp bin/herdr-hop
