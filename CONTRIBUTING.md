# Contributing to primer-scout

Thanks for contributing.

## What We Are Optimizing For

- Correctness on biological sequence matching semantics
- Predictable performance and reproducible benchmarks
- Stable CLI behavior for automation and CI pipelines

## Local Setup

1. Install Rust stable.
2. Clone the repository.
3. Run:

```bash
cargo build
cargo test --all-targets --all-features
```

## Development Workflow

1. Create a branch from `main`.
2. Implement your change with tests.
3. Run quality checks:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

4. If performance-sensitive code changed, run:

```bash
./scripts/run_benchmark.sh
cargo bench --bench engine
```

5. Update docs where behavior or CLI flags changed.

## Contribution Guidelines

- Keep pull requests focused and small.
- Add or update tests for parser/engine/CLI behavior.
- Do not introduce unverifiable performance claims.
- Do not break output contracts without an explicit migration note.

## Testing Expectations

Minimum for most PRs:
- Unit tests for new behavior
- No clippy warnings
- No formatting diffs

For matching-engine changes:
- Add test coverage for mismatch/strand/IUPAC behavior
- Include benchmark deltas when relevant

## Commit Message Suggestions

Use clear, scoped messages:
- `engine: tighten mismatch short-circuit`
- `cli: add --summary json example`
- `docs: clarify coordinate convention`

## Pull Request Checklist

- [ ] Change is clearly scoped
- [ ] Tests added/updated
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes with `-D warnings`
- [ ] `cargo test` passes
- [ ] Docs/README updated if needed
- [ ] Benchmark evidence attached (if perf claims)

## Reporting Security Issues

Do not open a public issue for sensitive vulnerabilities.
Share details privately with maintainers first.

## Code of Conduct

Be respectful and constructive in all interactions.
