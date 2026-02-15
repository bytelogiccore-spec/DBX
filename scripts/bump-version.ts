/**
 * DBX Version Bump Script
 *
 * Usage:
 *   npx tsx scripts/bump-version.ts 0.0.5-beta
 *   npx tsx scripts/bump-version.ts --check
 *
 * Updates version in all project files:
 *   - Cargo.toml (workspace)
 *   - core/dbx-{py,node,csharp}/Cargo.toml
 *   - core/dbx-ffi/Cargo.toml (dbx-core dep)
 *   - lang/python/pyproject.toml
 *   - lang/nodejs/package.json
 *   - lang/dotnet/DBX.Dotnet/DBX.Dotnet.csproj
 *   - docs/_config.yml
 */

import { readFileSync, writeFileSync, existsSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = resolve(__dirname, "..");

// ── Helpers ──────────────────────────────────────────────

function read(filePath: string): string {
    return readFileSync(resolve(ROOT, filePath), "utf-8");
}

function write(filePath: string, content: string): void {
    writeFileSync(resolve(ROOT, filePath), content, "utf-8");
}

function toPep440(version: string): string {
    return version.replace(/-beta/, "b0").replace(/-alpha/, "a0").replace(/-rc\.?(\d+)/, "rc$1");
}

function fromPep440(ver: string): string {
    return ver.replace(/b0$/, "-beta").replace(/a0$/, "-alpha").replace(/rc(\d+)$/, "-rc.$1");
}

interface ReplaceTarget {
    file: string;
    pattern: RegExp;
    replacement: (version: string) => string;
    label: string;
}

// ── Target Definitions ──────────────────────────────────

function getTargets(newVersion: string): ReplaceTarget[] {
    return [
        {
            file: "Cargo.toml",
            pattern: /^(version\s*=\s*)"[^"]*"/m,
            replacement: (v) => `$1"${v}"`,
            label: "Cargo.toml (workspace)",
        },
        ...["core/dbx-py", "core/dbx-node", "core/dbx-csharp"].map((dir) => ({
            file: `${dir}/Cargo.toml`,
            pattern: /^(version\s*=\s*)"[^"]*"/m,
            replacement: (v: string) => `$1"${v}"`,
            label: `${dir}/Cargo.toml (version)`,
        })),
        ...["core/dbx-py", "core/dbx-node", "core/dbx-csharp", "core/dbx-ffi"].map((dir) => ({
            file: `${dir}/Cargo.toml`,
            pattern: /(dbx-core\s*=\s*\{[^}]*version\s*=\s*)"[^"]*"/,
            replacement: (v: string) => `$1"${v}"`,
            label: `${dir}/Cargo.toml (dbx-core dep)`,
        })),
        {
            file: "lang/python/pyproject.toml",
            pattern: /^(version\s*=\s*)"[^"]*"/m,
            replacement: (v) => `$1"${toPep440(v)}"`,
            label: "lang/python/pyproject.toml",
        },
        {
            file: "lang/nodejs/package.json",
            pattern: /("version"\s*:\s*)"[^"]*"/,
            replacement: (v) => `$1"${v}"`,
            label: "lang/nodejs/package.json",
        },
        {
            file: "lang/dotnet/DBX.Dotnet/DBX.Dotnet.csproj",
            pattern: /(<Version>)[^<]*(<\/Version>)/,
            replacement: (v) => `$1${v}$2`,
            label: "lang/dotnet/DBX.Dotnet.csproj",
        },
        ...["dbx_version", "dbx_py_version", "dbx_node_version", "dbx_dotnet_version"].map((key) => ({
            file: "docs/_config.yml",
            pattern: new RegExp(`(${key}:\\s*)"[^"]*"`),
            replacement: (v: string) => `$1"${v}"`,
            label: `docs/_config.yml (${key})`,
        })),
    ];
}

// ── Commands ─────────────────────────────────────────────

function check(): void {
    console.log("\n📋 Current versions across project:\n");

    const checks = [
        { label: "Cargo workspace", file: "Cargo.toml", pattern: /^version\s*=\s*"([^"]*)"/m },
        { label: "dbx-py crate", file: "core/dbx-py/Cargo.toml", pattern: /^version\s*=\s*"([^"]*)"/m },
        { label: "dbx-node crate", file: "core/dbx-node/Cargo.toml", pattern: /^version\s*=\s*"([^"]*)"/m },
        { label: "dbx-csharp crate", file: "core/dbx-csharp/Cargo.toml", pattern: /^version\s*=\s*"([^"]*)"/m },
        { label: "Python (pyproject)", file: "lang/python/pyproject.toml", pattern: /^version\s*=\s*"([^"]*)"/m },
        { label: "Node.js (package)", file: "lang/nodejs/package.json", pattern: /"version"\s*:\s*"([^"]*)"/ },
        { label: ".NET (csproj)", file: "lang/dotnet/DBX.Dotnet/DBX.Dotnet.csproj", pattern: /<Version>([^<]*)</ },
        { label: "Docs (config)", file: "docs/_config.yml", pattern: /dbx_version:\s*"([^"]*)"/ },
    ];

    const versions = new Set<string>();

    for (const { label, file, pattern } of checks) {
        const fullPath = resolve(ROOT, file);
        if (!existsSync(fullPath)) {
            console.log(`  ⚠️  ${label.padEnd(22)} — file not found`);
            continue;
        }
        const match = read(file).match(pattern);
        const ver = match?.[1] ?? "???";
        const normalized = fromPep440(ver);
        versions.add(normalized);
        const suffix = ver !== normalized ? ` (raw: ${ver})` : "";
        console.log(`  ${label.padEnd(22)} ${normalized}${suffix}`);
    }

    console.log();
    if (versions.size === 1) {
        console.log(`✅ All versions are in sync: ${[...versions][0]}\n`);
    } else {
        console.log(`⚠️  Version mismatch detected! Found ${versions.size} different versions.\n`);
        process.exitCode = 1;
    }
}

function bump(newVersion: string): void {
    console.log(`\n🚀 Bumping all versions to ${newVersion}\n`);

    const targets = getTargets(newVersion);
    let updated = 0;
    let skipped = 0;

    for (const target of targets) {
        const fullPath = resolve(ROOT, target.file);
        if (!existsSync(fullPath)) {
            console.log(`  ⏭️  ${target.label} — file not found, skipping`);
            skipped++;
            continue;
        }

        const content = read(target.file);
        const replaced = content.replace(target.pattern, target.replacement(newVersion));

        if (content === replaced) {
            console.log(`  ✔️  ${target.label} — already up to date`);
        } else {
            write(target.file, replaced);
            console.log(`  ✅ ${target.label} — updated`);
            updated++;
        }
    }

    console.log(`\n📊 Results: ${updated} updated, ${skipped} skipped`);
    console.log(`\n💡 Next steps:`);
    console.log(`   1. Update CHANGELOG.md with new version header`);
    console.log(`   2. cargo check -p dbx-core`);
    console.log(`   3. git add -A && git commit -m "chore: bump version to ${newVersion}"`);
    console.log();
}

// ── Entry Point ──────────────────────────────────────────

const args = process.argv.slice(2);

if (args.length === 0 || args.includes("--help") || args.includes("-h")) {
    console.log(`
  Usage:
    npx tsx scripts/bump-version.ts <version>   Bump all versions
    npx tsx scripts/bump-version.ts --check     Show current versions

  Examples:
    npx tsx scripts/bump-version.ts 0.0.5-beta
    npx tsx scripts/bump-version.ts 1.0.0
  `);
} else if (args.includes("--check")) {
    check();
} else {
    const version = args[0];
    if (!/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(version)) {
        console.error(`❌ Invalid version format: "${version}". Expected: x.y.z or x.y.z-prerelease`);
        process.exit(1);
    }
    bump(version);
}
