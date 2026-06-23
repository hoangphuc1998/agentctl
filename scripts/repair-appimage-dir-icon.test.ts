import { mkdtempSync, readlinkSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, describe, expect, it } from "vitest";

type RepairModule = {
  repairDirIconLink(appDir: string): { changed: boolean; target: string | null };
  replaceFile(source: string, destination: string, fsOps: ReplaceFileFsOps): void;
};

type ReplaceFileFsOps = {
  renameSync(source: string, destination: string): void;
  copyFileSync(source: string, destination: string): void;
  rmSync(path: string, options: { force: true }): void;
};

const tempDirs: string[] = [];

afterEach(() => {
  for (const dir of tempDirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

describe("repair-appimage-dir-icon", () => {
  it("rewrites an absolute DirIcon symlink to a relative root icon symlink", async () => {
    const { repairDirIconLink } = await loadRepairModule();
    const appDir = makeAppDir();
    const iconPath = join(appDir, "Agent Manager.png");
    const dirIconPath = join(appDir, ".DirIcon");
    writeFileSync(iconPath, "png");
    symlinkSync(iconPath, dirIconPath);

    const result = repairDirIconLink(appDir);

    expect(result).toEqual({ changed: true, target: "Agent Manager.png" });
    expect(readlinkSync(dirIconPath)).toBe("Agent Manager.png");
  });

  it("leaves an already-relative DirIcon symlink unchanged", async () => {
    const { repairDirIconLink } = await loadRepairModule();
    const appDir = makeAppDir();
    const dirIconPath = join(appDir, ".DirIcon");
    writeFileSync(join(appDir, "Agent Manager.png"), "png");
    symlinkSync("Agent Manager.png", dirIconPath);

    const result = repairDirIconLink(appDir);

    expect(result).toEqual({ changed: false, target: "Agent Manager.png" });
    expect(readlinkSync(dirIconPath)).toBe("Agent Manager.png");
  });

  it("copies and removes the temporary AppImage when rename crosses filesystems", async () => {
    const { replaceFile } = await loadRepairModule();
    const calls: string[] = [];
    const fsOps: ReplaceFileFsOps = {
      renameSync(source, destination) {
        calls.push(`rename:${source}:${destination}`);
        const error = new Error("cross-device link not permitted") as NodeJS.ErrnoException;
        error.code = "EXDEV";
        throw error;
      },
      copyFileSync(source, destination) {
        calls.push(`copy:${source}:${destination}`);
      },
      rmSync(path, options) {
        calls.push(`rm:${path}:${String(options.force)}`);
      }
    };

    replaceFile("/tmp/fixed.AppImage", "/repo/app.AppImage", fsOps);

    expect(calls).toEqual([
      "rename:/tmp/fixed.AppImage:/repo/app.AppImage",
      "copy:/tmp/fixed.AppImage:/repo/app.AppImage",
      "rm:/tmp/fixed.AppImage:true"
    ]);
  });
});

async function loadRepairModule() {
  return (await import(pathToFileURL(join(process.cwd(), "scripts/repair-appimage-dir-icon.mjs")).href)) as RepairModule;
}

function makeAppDir() {
  const dir = mkdtempSync(join(tmpdir(), "agent-manager-appdir-"));
  tempDirs.push(dir);
  return dir;
}
