---
title: Telemetry
description: What FerrFlow collects, how data is anonymized, and how to opt out.
---

FerrFlow collects anonymous usage telemetry to help improve the tool. This page explains exactly what is sent, how it is anonymized, and how to disable it.

## What is collected

Each time you run a command, FerrFlow may send a single event containing:

| Field           | Description                                                         |
| --------------- | ------------------------------------------------------------------- |
| `event_type`    | The action performed: `check`, `release`, `version_bump`, or `init` |
| `commits_count` | Number of commits since the last release                            |
| `repo_hash`     | A SHA-256 hash of your git remote URL (see below)                   |

Only fields relevant to the command are included. Empty fields are omitted.

## How data is anonymized

Your repository URL is **never sent in plain text**. FerrFlow computes a SHA-256 hash of the git remote URL and sends only the resulting hex digest. This lets us count unique repositories without knowing which repositories they are.

No source code, file names, commit messages, branch names, package names, version numbers, IP addresses, or personal information are ever collected or stored.

## Where data is sent

Events are sent as a POST request to `https://api.ferrflow.com/events`. The request is asynchronous and non-blocking — it never slows down your workflow. If the request fails, it is silently discarded.

## How to opt out

You can disable telemetry entirely using either an environment variable or your config file.

### Environment variable

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="Linux / macOS"><p class="ferr-tab__label">Linux / macOS</p><div class="ferr-tab__body"><pre><code class="language-bash">export FERRFLOW_ANONYMOUS_TELEMETRY=false
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Windows"><p class="ferr-tab__label">Windows</p><div class="ferr-tab__body"><pre><code class="language-powershell">$env:FERRFLOW_ANONYMOUS_TELEMETRY = &quot;false&quot;
</code></pre>
</div></div>
</div>

Accepted values to disable: `false`, `0`, `off`, `no` (case-insensitive).

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p><code>FERRFLOW_TELEMETRY=false</code> also works as a fallback.</p>
</div></aside>

### Config file

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;telemetry&quot;: false
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
telemetry = false
</code></pre>
</div></div>
</div>

Either method is sufficient to disable telemetry. If the config file disables it, the environment variable cannot re-enable it, and vice versa.
