const fs = require("fs");
const path = require("path");

const root = path.join(__dirname, "..");
const target = path.join(root, "docs", "site", "data");

fs.mkdirSync(target, { recursive: true });

// The schema is the source of truth for the CLI being released, so the copy
// shipped alongside the pages is that exact file rather than whatever main
// happens to hold when a site build runs.
fs.copyFileSync(
  path.join(root, "schema", "ferrflow.json"),
  path.join(target, "schema.json"),
);

// Written by the release job before ferrflow runs, from the hyperfine-baseline
// artifact of the benchmark matrix this commit already waited on. Absent on a
// dry run and on any release that skipped the benchmarks, which is not an
// error: the previously published numbers stay in place.
const bench = process.env.FERRFLOW_BENCHMARK_JSON;
if (bench && fs.existsSync(bench)) {
  const parsed = JSON.parse(fs.readFileSync(bench, "utf8"));
  parsed.ferrflow_version = `ferrflow ${process.env.FERRFLOW_NEW_VERSION}`;
  fs.writeFileSync(
    path.join(target, "benchmarks.json"),
    JSON.stringify(parsed, null, 2) + "\n",
  );
  console.log(`embedded benchmarks for ${parsed.ferrflow_version}`);
} else {
  console.log("no benchmark artifact in this run, keeping the published numbers");
}
