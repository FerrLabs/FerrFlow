const fs = require("fs");
const path = require("path");

const version = process.env.FERRFLOW_NEW_VERSION;
if (!version) {
  console.error("FERRFLOW_NEW_VERSION is not set");
  process.exit(1);
}

const pkgPath = path.join(__dirname, "..", "npm", "package.json");
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));

for (const dep of Object.keys(pkg.optionalDependencies || {})) {
  pkg.optionalDependencies[dep] = version;
}

fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
