const fs = require("fs");
const path = require("path");

const root = path.join(__dirname, "..");
const target = path.join(root, "docs", "site", "data");

const schemaSource = path.join(root, "schema");
const schemaTarget = path.join(target, "schema");

fs.mkdirSync(schemaTarget, { recursive: true });

// The schema is the source of truth for the CLI being released, so the copy
// shipped alongside the pages is that exact file rather than whatever main
// happens to hold when a site build runs.
const schemas = fs
  .readdirSync(schemaSource)
  .filter((name) => name.endsWith(".json"));
for (const name of schemas) {
  fs.copyFileSync(path.join(schemaSource, name), path.join(schemaTarget, name));
}

// Written by the release job before ferrflow runs, from the hyperfine-baseline
// artifact of the benchmark matrix this commit already waited on. Absent on a
// dry run, on a workflow_dispatch release, and on any release that skipped the
// benchmarks: the previously published numbers stay in place.
const bench = process.env.FERRFLOW_BENCHMARK_JSON;
let embedded = false;

if (bench && fs.existsSync(bench)) {
  // A truncated or malformed artifact must not abort the release. postBump
  // runs once the version files are already bumped, and `.ferrflow` sets no
  // onFailure, so an uncaught throw here stops the run mid-release.
  try {
    const parsed = JSON.parse(fs.readFileSync(bench, "utf8"));
    parsed.ferrflow_version = `ferrflow ${process.env.FERRFLOW_NEW_VERSION}`;
    fs.writeFileSync(
      path.join(target, "benchmarks.json"),
      JSON.stringify(parsed, null, 2) + "\n",
    );
    console.log(`embedded benchmarks for ${parsed.ferrflow_version}`);
    embedded = true;
  } catch (err) {
    console.log(`benchmark artifact unusable (${err.message})`);
  }
}

if (!embedded) {
  console.log("no usable benchmark artifact, keeping the published numbers");
}
