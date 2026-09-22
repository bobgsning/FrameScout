# Contributing to FrameScout

Thanks for stopping by! FrameScout is a small open-source project, and any help is appreciated — whether it's fixing a typo, reporting a bug, or suggesting an idea.

Please read our [AI_POLICY.md](AI_POLICY.md) to understand how AI tools are used in this project.

## How to Contribute

### Report a Bug

Open an [issue](https://github.com/bobgsning/FrameScout/issues) and include:

- What you were doing when the bug occurred
- What you expected to happen vs. what actually happened
- Your OS version and any relevant error messages

That's it. No template required — just describe the problem clearly.

### Suggest a Feature

Open an issue and tell us:

- What problem you're trying to solve
- How you imagine it could work

Even rough ideas are welcome. We'll figure out the details together.

### Submit Code

1. Fork the repo
2. Create a branch: `git checkout -b your-feature-name`
3. Make your changes
4. Test them manually (automated tests are planned but not yet available)
5. If you modified `src/proto/search.proto`, regenerate the Python bindings:

> **Prerequisite**: Install the Protocol Buffers compiler (`protoc`) from [https://github.com/protocolbuffers/protobuf/releases](https://github.com/protocolbuffers/protobuf/releases) and ensure it's in your PATH.

   ```bash
   protoc -I=./src/proto --python_out=./src/inference-worker ./src/proto/search.proto
   ```

   The generated `search_pb2.py` is tracked in the repository, so this step is only needed when the proto schema changes.
6. Push and open a Pull Request

Don't worry too much about commit message conventions. Just write something clear. If AI tools helped you, feel free to mention it in the PR description — but it's not required.
If you’d like, consider using [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) (e.g., feat:, fix:, docs:). It helps generate changelogs automatically.

## Code Style

Try to match the existing style of the code you're modifying. If you're unsure, just do your best — We'll clean things up before merging.

- **Rust**: `rustfmt` is preferred but not enforced
- **Python**: Follow basic PEP 8 if possible
- **Vue/TypeScript**: Use `<script setup lang="ts">` like the existing components

## What We Need Help With

These are the areas where contributions would be most valuable right now:

- **Bug fixes** — Always welcome
- **Documentation** — Improving README, fixing typos, adding examples
- **Cross-platform support** — macOS and Linux builds are planned but untested
- **Small UI improvements** — Better layouts, tooltips, responsive tweaks

## What's Not Ready Yet

For large architectural changes or new ML models, please open an issue first to discuss — we'd love to hear your ideas!

## Code of Conduct

Be kind. Be constructive. Assume good intentions.

## Questions?

Open an issue or email <bobgsning@outlook.com>.

> By submitting a pull request, you agree that your contributions will be licensed under the Apache 2.0 license.
