import { execFileSync } from "node:child_process";
import { mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const artifactDirectory = resolve(process.argv[2] ?? "artifacts/wasm");
const outputDirectory = resolve(process.argv[3] ?? "artifacts/bindings");
const specManifest = JSON.parse(await readFile("artifacts/specs/manifest.json", "utf8"));
const contracts = [
  ["evidence-registry", "evidence_registry.wasm"],
  ["policy-account-fixture", "policy_account_fixture.wasm"],
  ["web-auth-fixture", "web_auth_fixture.wasm"],
];

async function walk(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "dist" && entry.name !== "node_modules") {
        files.push(...(await walk(path)));
      }
    } else {
      files.push(path);
    }
  }
  return files;
}

await mkdir(outputDirectory, { recursive: true });
for (const [contractName, wasmName] of contracts) {
  const packageDirectory = join(outputDirectory, contractName);
  await rm(packageDirectory, { recursive: true, force: true });
  execFileSync(
    "stellar",
    [
      "contract",
      "bindings",
      "typescript",
      "--quiet",
      "--wasm",
      join(artifactDirectory, wasmName),
      "--output-dir",
      packageDirectory,
    ],
    { stdio: "inherit" },
  );

  const packagePath = join(packageDirectory, "package.json");
  const packageJson = JSON.parse(await readFile(packagePath, "utf8"));
  packageJson.name = `@rampspec/${contractName}`;
  packageJson.version = "0.1.0";
  packageJson.description = `Generated RampSpec ${contractName} Soroban client`;
  packageJson.license = "Apache-2.0";
  packageJson.files = ["dist", "README.md"];
  packageJson.scripts = {
    build: "tsc",
    check: "tsc --noEmit",
    test: "npm run build && node test/smoke.mjs",
  };
  packageJson.dependencies = {
    "@stellar/stellar-sdk": "16.0.1",
    buffer: "6.0.3",
  };
  packageJson.devDependencies = { typescript: "5.6.2" };
  packageJson.rampspec = {
    generator: "stellar-cli 27.1.0",
    specSha256: specManifest.contracts[contractName].jsonSha256,
    wasmSha256: specManifest.contracts[contractName].wasmSha256,
  };
  await writeFile(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`, "utf8");
  await writeFile(join(packageDirectory, ".gitignore"), "dist/\nnode_modules/\n", "utf8");

  const spec = JSON.parse(
    await readFile(join("artifacts/specs", specManifest.contracts[contractName].json), "utf8"),
  );
  const methods = spec
    .filter((entry) => entry.function_v0 && !entry.function_v0.name.startsWith("__"))
    .map((entry) => entry.function_v0.name)
    .sort();
  const smoke = `import assert from "node:assert/strict";\nimport { Client } from "../dist/index.js";\n\nconst client = new Client({\n  allowHttp: true,\n  contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",\n  networkPassphrase: "Standalone Network ; February 2017",\n  rpcUrl: "http://127.0.0.1:8000",\n});\nassert.deepEqual(Object.keys(client.fromJSON).sort(), ${JSON.stringify(methods)});\n`;
  await mkdir(join(packageDirectory, "test"), { recursive: true });
  await writeFile(join(packageDirectory, "test", "smoke.mjs"), smoke, "utf8");

  for (const file of await walk(packageDirectory)) {
    if (/\.(json|md|mjs|ts)$/.test(file) || file.endsWith(".gitignore")) {
      const content = await readFile(file, "utf8");
      await writeFile(file, content.replaceAll("\r\n", "\n"), "utf8");
    }
  }
  const npmArguments = [
    "install",
    "--package-lock-only",
    "--ignore-scripts",
    "--no-audit",
    "--no-fund",
  ];
  if (process.platform === "win32") {
    execFileSync(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm", ...npmArguments], {
      cwd: packageDirectory,
      stdio: "inherit",
    });
  } else {
    execFileSync("npm", npmArguments, { cwd: packageDirectory, stdio: "inherit" });
  }
}

process.stdout.write(`generated TypeScript bindings in ${outputDirectory}\n`);
