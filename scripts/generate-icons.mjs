import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import zlib from "node:zlib";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const iconDir = join(root, "src-tauri", "icons");
mkdirSync(iconDir, { recursive: true });

const sizes = [
  ["32x32.png", 32],
  ["128x128.png", 128],
  ["128x128@2x.png", 256],
  ["icon.png", 512]
];

for (const [name, size] of sizes) {
  writeFileSync(join(iconDir, name), png(size, size));
}

function png(width, height) {
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) {
    const row = y * (width * 4 + 1);
    raw[row] = 0;
    for (let x = 0; x < width; x++) {
      const offset = row + 1 + x * 4;
      const border = x < width * 0.08 || y < height * 0.08 || x > width * 0.92 || y > height * 0.92;
      const terminal = x > width * 0.18 && x < width * 0.82 && y > height * 0.22 && y < height * 0.78;
      const prompt = terminal && x > width * 0.28 && x < width * 0.43 && y > height * 0.43 && y < height * 0.5;
      const cursor = terminal && x > width * 0.48 && x < width * 0.66 && y > height * 0.57 && y < height * 0.63;
      const accent = prompt || cursor;
      raw[offset] = accent ? 80 : border ? 38 : terminal ? 17 : 11;
      raw[offset + 1] = accent ? 216 : border ? 50 : terminal ? 24 : 15;
      raw[offset + 2] = accent ? 144 : border ? 65 : terminal ? 33 : 20;
      raw[offset + 3] = 255;
    }
  }

  return Buffer.concat([
    signature(),
    chunk("IHDR", ihdr(width, height)),
    chunk("IDAT", zlib.deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0))
  ]);
}

function signature() {
  return Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
}

function ihdr(width, height) {
  const data = Buffer.alloc(13);
  data.writeUInt32BE(width, 0);
  data.writeUInt32BE(height, 4);
  data[8] = 8;
  data[9] = 6;
  data[10] = 0;
  data[11] = 0;
  data[12] = 0;
  return data;
}

function chunk(type, data) {
  const typeBuffer = Buffer.from(type);
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const crcInput = Buffer.concat([typeBuffer, data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(crcInput), 0);
  return Buffer.concat([length, typeBuffer, data, crc]);
}

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

