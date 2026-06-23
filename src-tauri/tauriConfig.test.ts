import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();

function readText(path: string) {
  return readFileSync(join(root, path), "utf8");
}

describe("Tauri package configuration", () => {
  it("repairs AppImage DirIcon metadata after AppImage bundle builds", () => {
    const packageJson = JSON.parse(readText("package.json"));
    const repairCommand = "node scripts/repair-appimage-dir-icon.mjs";

    expect(packageJson.scripts["tauri:build"]).toContain(`--bundles appimage,deb && ${repairCommand}`);
    expect(packageJson.scripts["tauri:build:appimage"]).toContain(`--bundles appimage && ${repairCommand}`);
    expect(packageJson.scripts["tauri:build:deb"]).not.toContain(repairCommand);
  });

  it("refreshes Linux desktop icon caches after Debian install and remove", () => {
    const config = JSON.parse(readText("src-tauri/tauri.conf.json"));
    const debConfig = config.bundle.linux.deb;

    expect(debConfig.postInstallScript).toBe("../scripts/deb-postinst.sh");
    expect(debConfig.postRemoveScript).toBe("../scripts/deb-postrm.sh");

    const postInstallScript = readText("scripts/deb-postinst.sh");
    const postRemoveScript = readText("scripts/deb-postrm.sh");

    for (const script of [postInstallScript, postRemoveScript]) {
      expect(script).toContain("gtk-update-icon-cache");
      expect(script).toContain("update-desktop-database");
      expect(script).toContain("/usr/share/icons/hicolor");
      expect(script).toContain("/usr/share/applications");
    }
  });
});
