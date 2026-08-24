# Sandbox template

`Dockerfile` extends the Claude Code sandbox image with the Rust toolchain, the
CLI tools `taskfile.yml` drives, Task and vim, so a fresh sandbox needs no
setup.

Build it and load it into the sbx runtime, which does not share the host image
store:

```bash
docker build -t sbx-cmt:v1 sandbox/
docker image save sbx-cmt:v1 -o /tmp/sbx-cmt-v1.tar
sbx template load /tmp/sbx-cmt-v1.tar
```

Run a sandbox from it:

```bash
sbx run --name cmt --template sbx-cmt:v1 claude
```

The agent argument must match the base image variant, so it stays `claude`.
Bump the tag on every rebuild: the runtime caches templates by tag and reusing
one can serve the older image.
