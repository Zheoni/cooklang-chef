const path = require("path");
const fs = require("fs");

const mappings = JSON.parse(fs.readFileSync("./package.json")).frontendMappings;

for (const item of mappings.copy) {
  let src, target;
  if (typeof item === "object") {
    src = item.src;
    target = item.target;
  } else {
    src = item;
    target = path.posix.basename(item);
  }

  const input = path.join(__dirname, "node_modules", src);
  const output = path.join(__dirname, mappings.target, target);

  fs.copyFileSync(input, output);

  console.log(output);
}

const licenses = [];
for (const [key, lic] of Object.entries(mappings.licenses)) {
  const file = path.join(__dirname, "node_modules", lic);
  const content = fs.readFileSync(file);
  licenses.push(key + "\n");
  licenses.push(content);
  licenses.push("\n----------\n");
}
const license_output = path.join(__dirname, mappings.target, "LICENSES");
fs.writeFileSync(license_output, licenses.join("\n"));
