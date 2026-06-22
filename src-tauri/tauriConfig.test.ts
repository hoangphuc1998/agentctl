import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();

function readText(path: string) {
  return readFileSync(join(root, path), "utf8");
}

describe("Tauri package configuration", () => {
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
