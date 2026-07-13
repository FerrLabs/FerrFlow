# Logging & observability

FerrFlow logs through the [`tracing`](https://docs.rs/tracing) crate. Every
diagnostic line — status, progress, warnings, errors — is a structured log
event, so you can control its verbosity, filter it by module, and ship it to a
log aggregator as JSON.

## stdout vs stderr

FerrFlow keeps a strict split between data and logs:

- **stdout** carries *data* — the `--json` output of `check` / `release` /
  `status` / `validate`, and the plain value printed by `version` and `tag`
  (a version like `1.4.2`, a tag name, …). This is the machine-readable output
  you capture in scripts, e.g. `V=$(ferrflow version)`.
- **stderr** carries *logs* — the human status report and every `tracing` event.

So you can redirect the two independently:

```bash
ferrflow check --json > result.json 2> run.log
```

`result.json` is clean JSON; `run.log` holds the diagnostics.

## Levels

| Level   | What it is                                                                    | Shown by default |
| ------- | ----------------------------------------------------------------------------- | ---------------- |
| `ERROR` | the command failed                                                            | yes              |
| `WARN`  | something non-fatal worth flagging (no changelog configured, no publishers …) | yes              |
| `INFO`  | normal status (headers, `✓ Updated …`, per-package result lines)              | yes              |
| `DEBUG` | verbose detail (changed-files list, per-package skip reasons)                 | only `--verbose` |
| `TRACE` | very fine-grained tracing                                                     | only via `RUST_LOG` |

## Controlling verbosity

- `--verbose` (`-v`) raises the default filter to `ferrflow=debug`, so `DEBUG`
  events show.
- `RUST_LOG` overrides everything with a per-module filter, using standard
  [`EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)
  syntax. It takes precedence over `--verbose`.

```bash
# debug just the git layer, info everywhere else
RUST_LOG=ferrflow=info,ferrflow::git=trace ferrflow release

# quiet everything except warnings and errors
RUST_LOG=ferrflow=warn ferrflow check
```

## Formats

`--log-format` selects how events are rendered:

- `human` (default) — one line per event, message only, colored when writing to
  a terminal. Visually identical to FerrFlow's classic output. Colors are
  dropped automatically when stderr is not a TTY (piped or redirected).
- `json` — one JSON object per line, ready for Datadog / Loki / CloudWatch
  ingestion.

### JSON event schema

```json
{"timestamp":"2026-07-13T12:08:54.709884Z","level":"INFO","fields":{"message":"✓ Updated CHANGELOG.md"},"target":"ferrflow::changelog"}
```

| Field            | Type                                                | Description                                                                                       |
| ---------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `timestamp`      | RFC 3339 string (UTC)                               | when the event was emitted                                                                       |
| `level`          | `TRACE` \| `DEBUG` \| `INFO` \| `WARN` \| `ERROR`   | severity                                                                                          |
| `fields.message` | string                                              | the log text (prefix glyphs like `✓` are part of the message; ANSI colors are absent in JSON)    |
| `target`         | string                                              | the emitting module path, e.g. `ferrflow::monorepo::check` — the key `RUST_LOG` filters on       |

Any structured key/value fields attached to an event appear alongside `message`
inside `fields`.

### Shipping to a log aggregator

Logs go to stderr, so forward that stream to your collector:

```bash
ferrflow release --log-format json 2> release.ndjson
```

Because `--json` data goes to stdout and logs go to stderr, you can capture the
machine result and the run log at the same time:

```bash
ferrflow release --json --log-format json 1> release.json 2> release.ndjson
```
