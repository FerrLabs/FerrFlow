import { ChangeDetectionStrategy, Component } from '@angular/core';
import { LegalLayoutComponent } from '../components/legal-layout.component';

@Component({
  selector: 'flr-security-en',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LegalLayoutComponent],
  template: `
    <flr-legal-layout
      lang="en"
      title="Security — FerrFlow"
      description="How to report a vulnerability in FerrFlow, the CLI threat model, and FerrFlow's security posture as an open-source tool."
      canonical="https://ferrflow.com/security/"
      lastUpdated="2026-05-14"
      [draft]="true"
      enHref="/security/"
      frHref="/fr/security/"
    >
      <p>
        FerrFlow is an open-source CLI. It runs entirely on your machine or inside your CI runner —
        your source code, your commits, your tags. Nothing leaves your infrastructure, nothing
        phones home. This page tells you how the binary supply chain is protected, where to report a
        vulnerability, and what is in scope.
      </p>

      <h2>Posture</h2>
      <p>
        FerrFlow has no servers to protect. There is no FerrFlow account, no database, no API
        endpoint handling user data. The CLI reads your git history, edits version strings in files
        you control, pushes tags through your existing git credentials, and exits. The companion
        site <strong>ferrflow.com</strong> is a static documentation surface — see the
        <a href="/privacy">privacy policy</a> for what little it logs.
      </p>
      <p>
        For the broader FerrLabs platform posture (encryption, identity, network isolation, audit
        logging, sub-processor management), see
        <a href="https://ferrlabs.com/security">ferrlabs.com/security</a>. FerrFlow inherits very
        little from that posture in practice — it's a CLI, not a SaaS — but the canonical page is the
        source of truth for FerrLabs-wide controls.
      </p>

      <h2>Threat model</h2>
      <p>
        The primary risk surface is the <strong>binary supply chain</strong>. A malicious FerrFlow
        build in a CI runner could read repository contents, push tags, or publish releases under
        the project's credentials. Mitigations:
      </p>
      <ul>
        <li>
          Every release is built in <strong>GitHub Actions</strong> from the public
          <a href="https://github.com/FerrLabs/FerrFlow">FerrLabs/FerrFlow</a> repository, with build
          provenance attestations (SLSA L3 target).
        </li>
        <li>
          Binaries and container images are <strong>signed</strong>; checksums and signatures are
          published alongside every release at
          <a href="https://github.com/FerrLabs/FerrFlow/releases">github.com/FerrLabs/FerrFlow/releases</a>.
        </li>
        <li>
          The self-host bundle <code>ghcr.io/ferrlabs/ferrflow-selfhost</code> is distributed only
          via GHCR, with image signatures verifiable via <code>cosign</code>.
        </li>
        <li>
          The JSON Schema at <code>ferrflow.com/schema/ferrflow.json</code> is served from the same
          static origin as this site and is byte-identical to <code>schema/ferrflow.json</code> in
          the source repo.
        </li>
      </ul>

      <h2>Reporting a vulnerability</h2>
      <p>Two channels — pick whichever fits your workflow. Both are monitored.</p>
      <ul>
        <li>
          <strong>Email</strong>: <a href="mailto:security@ferrlabs.com">security&#64;ferrlabs.com</a>.
          PGP key fingerprint and policy at
          <a href="https://ferrlabs.com/.well-known/security.txt">ferrlabs.com/.well-known/security.txt</a>.
        </li>
        <li>
          <strong>GitHub private security advisory</strong>:
          <a href="https://github.com/FerrLabs/FerrFlow/security/advisories/new">file a private advisory</a>
          directly on the repo. This is the preferred channel if you can already reproduce the issue
          against a tagged release.
        </li>
      </ul>
      <p>
        We follow <strong>coordinated disclosure with a 90-day default</strong>. We acknowledge
        reports within 3 business days, agree on a disclosure timeline, and credit reporters in the
        published advisory unless they ask to remain anonymous. Please do not open a public GitHub
        issue for vulnerabilities.
      </p>

      <h2>Scope</h2>
      <p><strong>In scope:</strong></p>
      <ul>
        <li>
          The <a href="https://github.com/FerrLabs/FerrFlow">FerrLabs/FerrFlow</a> CLI source and the
          binaries it produces.
        </li>
        <li>The published container images at <code>ghcr.io/ferrlabs/ferrflow*</code>.</li>
        <li>The JSON Schema served at <code>ferrflow.com/schema/ferrflow.json</code>.</li>
        <li>This marketing site (<code>ferrflow.com</code>).</li>
      </ul>
      <p><strong>Out of scope:</strong></p>
      <ul>
        <li>
          Third-party file formats FerrFlow reads or updates (<code>Cargo.toml</code>,
          <code>package.json</code>, <code>pyproject.toml</code>, <code>Chart.yaml</code>,
          <code>mix.exs</code>, <code>pubspec.yaml</code>, gemspecs, etc.) — vulnerabilities in those
          ecosystems belong upstream.
        </li>
        <li>
          Conventional Commits parsing bugs that are not security issues (those are normal bug
          reports).
        </li>
        <li>Denial of service from passing a pathological input to the CLI on your own machine.</li>
      </ul>

      <h2>No customer data</h2>
      <p>
        <strong>FerrFlow does not collect, transmit, or store any user data.</strong> No telemetry,
        no error reporting, no usage analytics, no phone-home. The CLI's only network activity is the
        git operations you explicitly invoke (fetching, pushing tags, creating GitHub releases
        through your own token) and HTTP calls to package registries that <em>you</em> configured.
      </p>
      <p>
        The companion site ferrflow.com uses no cookies and no third-party trackers; see the
        <a href="/cookies">cookies page</a> and <a href="/privacy">privacy policy</a> for the full
        detail.
      </p>

      <h2>Contact</h2>
      <p>
        <a href="mailto:security@ferrlabs.com">security&#64;ferrlabs.com</a> ·
        <a href="https://github.com/FerrLabs/FerrFlow/security/advisories/new">GitHub advisory</a> ·
        <a href="https://ferrlabs.com/security">FerrLabs platform security →</a>
      </p>

      <p style="margin-top:48px;font-size:13px;">
        French version: <a href="/fr/security">Sécurité</a>.
      </p>
    </flr-legal-layout>
  `,
})
export default class SecurityEnPage {}
