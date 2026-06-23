import { spawnSync } from "node:child_process";
import {
  createReadStream,
  createWriteStream,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  copyFileSync,
  readdirSync,
  readlinkSync,
  renameSync,
  rmSync,
  statSync,
  symlinkSync,
  unlinkSync
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { basename, dirname, isAbsolute, join, resolve } from "node:path";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));

export function repairDirIconLink(appDir) {
  const dirIconPath = join(appDir, ".DirIcon");
  let dirIconStat;

  try {
    dirIconStat = lstatSync(dirIconPath);
  } catch (error) {
    if (error.code === "ENOENT") {
      return { changed: false, target: null };
    }
    throw error;
  }

  if (!dirIconStat.isSymbolicLink()) {
    return { changed: false, target: null };
  }

  const currentTarget = readlinkSync(dirIconPath);
  if (!isAbsolute(currentTarget)) {
    return { changed: false, target: currentTarget };
  }

  const relativeTarget = basename(currentTarget);
  if (!existsSync(join(appDir, relativeTarget))) {
    throw new Error(`Cannot repair .DirIcon because ${relativeTarget} is missing from ${appDir}`);
  }

  unlinkSync(dirIconPath);
  symlinkSync(relativeTarget, dirIconPath);

  return { changed: true, target: relativeTarget };
}

export async function repairBuiltAppImages({ rootDir = repoRoot, log = console.log } = {}) {
  const appImageDir = join(rootDir, "target", "release", "bundle", "appimage");
  if (!existsSync(appImageDir)) {
    log("No AppImage bundle directory found; skipping .DirIcon repair.");
    return [];
  }

  const appImages = readdirSync(appImageDir)
    .filter((name) => name.endsWith(".AppImage"))
    .map((name) => join(appImageDir, name))
    .filter((path) => statSync(path).isFile());

  const results = [];
  for (const appImage of appImages) {
    results.push(await repairAppImage(appImage, { log }));
  }

  return results;
}

async function repairAppImage(appImage, { log }) {
  const tempDir = mkdtempSync(join(tmpdir(), "agent-manager-appimage-"));

  try {
    runCommand(appImage, ["--appimage-extract"], { cwd: tempDir });
    const appDir = join(tempDir, "squashfs-root");
    const dirIcon = repairDirIconLink(appDir);

    if (!dirIcon.changed) {
      log(`AppImage .DirIcon already portable: ${basename(appImage)}`);
      return { path: appImage, repaired: false, dirIconTarget: dirIcon.target };
    }

    const runtimePath = join(tempDir, "runtime-x86_64");
    await extractRuntime(appImage, runtimePath);

    const appImageTool = findAppImageTool(tempDir);
    const fixedAppImage = join(tempDir, basename(appImage));
    runCommand(appImageTool, ["--runtime-file", runtimePath, appDir, fixedAppImage], { cwd: tempDir });

    replaceFile(fixedAppImage, appImage);
    log(`Repaired AppImage .DirIcon: ${basename(appImage)} -> ${dirIcon.target}`);

    return { path: appImage, repaired: true, dirIconTarget: dirIcon.target };
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
}

export function replaceFile(source, destination, fsOps = { renameSync, copyFileSync, rmSync }) {
  try {
    fsOps.renameSync(source, destination);
  } catch (error) {
    if (error.code !== "EXDEV") {
      throw error;
    }

    fsOps.copyFileSync(source, destination);
    fsOps.rmSync(source, { force: true });
  }
}

async function extractRuntime(appImage, outputPath) {
  const offsetResult = runCommand(appImage, ["--appimage-offset"], { cwd: dirname(appImage) });
  const offset = Number.parseInt(offsetResult.stdout.trim(), 10);
  if (!Number.isFinite(offset) || offset <= 0) {
    throw new Error(`Could not determine AppImage runtime offset for ${appImage}`);
  }

  await pipeline(createReadStream(appImage, { start: 0, end: offset - 1 }), createWriteStream(outputPath));
}

function findAppImageTool(tempDir) {
  if (process.env.APPIMAGETOOL) {
    return process.env.APPIMAGETOOL;
  }

  const pathTool = spawnSync("appimagetool", ["--help"], { encoding: "utf8" });
  if (pathTool.status === 0) {
    return "appimagetool";
  }

  const cachedPlugin = join(homedir(), ".cache", "tauri", "linuxdeploy-plugin-appimage.AppImage");
  if (!existsSync(cachedPlugin)) {
    throw new Error("appimagetool was not found on PATH and Tauri's cached linuxdeploy AppImage plugin is missing.");
  }

  const pluginDir = join(tempDir, "linuxdeploy-plugin-appimage");
  mkdirSync(pluginDir);
  runCommand(cachedPlugin, ["--appimage-extract"], { cwd: pluginDir });

  const extractedTool = join(pluginDir, "squashfs-root", "usr", "bin", "appimagetool");
  if (!existsSync(extractedTool)) {
    throw new Error(`Tauri's cached AppImage plugin did not contain appimagetool at ${extractedTool}`);
  }

  return extractedTool;
}

function runCommand(command, args, { cwd }) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"]
  });

  if (result.status !== 0) {
    throw new Error(
      [`Command failed: ${command} ${args.join(" ")}`, result.stdout.trim(), result.stderr.trim()]
        .filter(Boolean)
        .join("\n")
    );
  }

  return { stdout: result.stdout, stderr: result.stderr };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  repairBuiltAppImages().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
