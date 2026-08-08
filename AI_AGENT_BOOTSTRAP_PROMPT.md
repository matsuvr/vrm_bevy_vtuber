# AI_AGENT bootstrap prompt

Use the repository documents as binding implementation instructions.

Read, in this order:

1. `AGENTS.md`
2. `DESIGN.md`
3. `AI_AGENT_TASKS.md`
4. every accepted ADR under `docs/adr/`
5. `REFERENCES.md`

Implement **G0-01 only**.

The project is a full-Rust desktop VTuber application with these fixed boundaries:

- VRM 1.0 only;
- Windows and macOS only;
- Bevy 0.19.0;
- `bevy_vrm1` is the sole VRM runtime dependency in later tasks;
- camera, inference, tracking, avatar integration, and app orchestration remain separate crates;
- no unbounded frame queues;
- no native C/C++ inference runtime;
- no custom VRM loader, MToon, SpringBone, Node Constraint, or Expression runtime.

For G0-01:

- create only the workspace, empty crates, toolchain, workspace lint policy, formatting configuration, license-policy skeleton, root README, and Windows/macOS CI skeleton required by G0-01;
- use the exact directory and package names from `DESIGN.md`;
- do not add Bevy, `bevy_vrm1`, `nokhwa`, tract, camera code, model artifacts, UI, or VRM code yet;
- keep `vtuber-core` engine- and platform-independent;
- configure CI for Windows and macOS only;
- commit `Cargo.lock` if Cargo generates it;
- run every verification command listed in G0-01;
- report exact commands and results, including commands that cannot run in the current environment;
- do not commit, push, or open a pull request unless explicitly instructed.

Before finishing, compare the resulting directory tree and dependency graph with all G0-01 acceptance criteria. Do not continue to G0-02.
