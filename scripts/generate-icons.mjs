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
  writeFileSync(join(iconDir, name), renderIcon(size));
}

function renderIcon(size) {
  const sampleScale = size <= 64 ? 8 : 4;
  const renderSize = size * sampleScale;
  const canvas = new Uint8ClampedArray(renderSize * renderSize * 4);
  const draw = renderer(canvas, renderSize);

  draw.roundedRect(42, 50, 428, 428, 105, [0, 0, 0, 72]);
  draw.roundedGradient(42, 34, 428, 428, 105, [18, 26, 35, 255], [6, 10, 14, 255]);
  draw.roundedRectStroke(42, 34, 428, 428, 105, 8, [45, 63, 82, 190]);

  draw.roundedRect(104, 151, 304, 238, 39, [0, 0, 0, 82]);
  draw.roundedGradient(102, 136, 308, 238, 35, [21, 29, 40, 255], [8, 12, 17, 255]);
  draw.roundedRectStroke(102, 136, 308, 238, 35, 7, [74, 96, 121, 210]);
  draw.line(126, 196, 386, 196, 6, [52, 71, 94, 205]);

  draw.circle(150, 166, 9, [94, 225, 140, 255]);
  draw.circle(180, 166, 9, [115, 183, 255, 190]);
  draw.circle(210, 166, 9, [149, 168, 188, 150]);

  draw.line(286, 242, 330, 242, 9, [115, 183, 255, 185]);
  draw.line(330, 242, 330, 291, 9, [115, 183, 255, 185]);
  draw.line(330, 291, 374, 291, 9, [115, 183, 255, 185]);
  draw.line(330, 242, 371, 214, 9, [115, 183, 255, 185]);
  draw.circle(286, 242, 15, [13, 19, 27, 255]);
  draw.circle(330, 242, 15, [13, 19, 27, 255]);
  draw.circle(374, 291, 15, [13, 19, 27, 255]);
  draw.circle(371, 214, 15, [13, 19, 27, 255]);
  draw.circle(286, 242, 9, [115, 183, 255, 245]);
  draw.circle(330, 242, 9, [115, 183, 255, 245]);
  draw.circle(374, 291, 9, [115, 183, 255, 245]);
  draw.circle(371, 214, 9, [115, 183, 255, 245]);

  draw.line(154, 256, 183, 281, 15, [94, 225, 140, 255]);
  draw.line(183, 281, 154, 306, 15, [94, 225, 140, 255]);
  draw.line(216, 307, 294, 307, 16, [94, 225, 140, 255]);

  draw.roundedRect(116, 361, 280, 12, 6, [94, 225, 140, 42]);

  return png(size, size, downsample(canvas, renderSize, size, sampleScale));
}

function renderer(canvas, size) {
  const unit = size / 512;
  const toPx = (value) => Math.round(value * unit);

  function pixel(x, y, rgba) {
    if (x < 0 || y < 0 || x >= size || y >= size) return;
    const offset = (y * size + x) * 4;
    const alpha = rgba[3] / 255;
    const inverse = 1 - alpha;
    canvas[offset] = Math.round(rgba[0] * alpha + canvas[offset] * inverse);
    canvas[offset + 1] = Math.round(rgba[1] * alpha + canvas[offset + 1] * inverse);
    canvas[offset + 2] = Math.round(rgba[2] * alpha + canvas[offset + 2] * inverse);
    canvas[offset + 3] = Math.round(rgba[3] + canvas[offset + 3] * inverse);
  }

  function roundedRect(x, y, width, height, radius, rgba) {
    fillRoundedRect(x, y, width, height, radius, () => rgba);
  }

  function roundedGradient(x, y, width, height, radius, top, bottom) {
    fillRoundedRect(x, y, width, height, radius, (py) => {
      const t = clamp((py / unit - y) / height, 0, 1);
      return [
        lerp(top[0], bottom[0], t),
        lerp(top[1], bottom[1], t),
        lerp(top[2], bottom[2], t),
        lerp(top[3], bottom[3], t)
      ];
    });
  }

  function roundedRectStroke(x, y, width, height, radius, strokeWidth, rgba) {
    const outer = rectPixels(x, y, width, height, radius);
    const inner = rectPixels(
      x + strokeWidth,
      y + strokeWidth,
      width - strokeWidth * 2,
      height - strokeWidth * 2,
      Math.max(0, radius - strokeWidth)
    );

    for (let py = outer.top; py < outer.bottom; py++) {
      for (let px = outer.left; px < outer.right; px++) {
        if (insideRoundedRect(px, py, outer) && !insideRoundedRect(px, py, inner)) {
          pixel(px, py, rgba);
        }
      }
    }
  }

  function fillRoundedRect(x, y, width, height, radius, colorAt) {
    const rect = rectPixels(x, y, width, height, radius);

    for (let py = rect.top; py < rect.bottom; py++) {
      for (let px = rect.left; px < rect.right; px++) {
        if (insideRoundedRect(px, py, rect)) {
          pixel(px, py, colorAt(py));
        }
      }
    }
  }

  function rectPixels(x, y, width, height, radius) {
    return {
      left: toPx(x),
      top: toPx(y),
      right: toPx(x + width),
      bottom: toPx(y + height),
      radius: toPx(radius)
    };
  }

  function insideRoundedRect(px, py, rect) {
    if (px < rect.left || py < rect.top || px >= rect.right || py >= rect.bottom) return false;
    const dx = Math.max(rect.left + rect.radius - px, 0, px - (rect.right - rect.radius));
    const dy = Math.max(rect.top + rect.radius - py, 0, py - (rect.bottom - rect.radius));
    return dx * dx + dy * dy <= rect.radius * rect.radius;
  }

  function line(x1, y1, x2, y2, width, rgba) {
    const ax = x1 * unit;
    const ay = y1 * unit;
    const bx = x2 * unit;
    const by = y2 * unit;
    const half = (width * unit) / 2;
    const minX = Math.floor(Math.min(ax, bx) - half);
    const maxX = Math.ceil(Math.max(ax, bx) + half);
    const minY = Math.floor(Math.min(ay, by) - half);
    const maxY = Math.ceil(Math.max(ay, by) + half);

    for (let py = minY; py <= maxY; py++) {
      for (let px = minX; px <= maxX; px++) {
        if (distanceToSegment(px, py, ax, ay, bx, by) <= half) {
          pixel(px, py, rgba);
        }
      }
    }
  }

  function circle(cx, cy, radius, rgba) {
    const centerX = cx * unit;
    const centerY = cy * unit;
    const r = radius * unit;
    const minX = Math.floor(centerX - r);
    const maxX = Math.ceil(centerX + r);
    const minY = Math.floor(centerY - r);
    const maxY = Math.ceil(centerY + r);

    for (let py = minY; py <= maxY; py++) {
      for (let px = minX; px <= maxX; px++) {
        const dx = px - centerX;
        const dy = py - centerY;
        if (dx * dx + dy * dy <= r * r) {
          pixel(px, py, rgba);
        }
      }
    }
  }

  return { circle, line, roundedGradient, roundedRect, roundedRectStroke };
}

function downsample(canvas, renderSize, outputSize, scale) {
  const pixels = Buffer.alloc(outputSize * outputSize * 4);
  const area = scale * scale;

  for (let y = 0; y < outputSize; y++) {
    for (let x = 0; x < outputSize; x++) {
      let red = 0;
      let green = 0;
      let blue = 0;
      let alpha = 0;

      for (let sy = 0; sy < scale; sy++) {
        for (let sx = 0; sx < scale; sx++) {
          const offset = ((y * scale + sy) * renderSize + x * scale + sx) * 4;
          red += canvas[offset];
          green += canvas[offset + 1];
          blue += canvas[offset + 2];
          alpha += canvas[offset + 3];
        }
      }

      const target = (y * outputSize + x) * 4;
      pixels[target] = Math.round(red / area);
      pixels[target + 1] = Math.round(green / area);
      pixels[target + 2] = Math.round(blue / area);
      pixels[target + 3] = Math.round(alpha / area);
    }
  }

  return pixels;
}

function png(width, height, pixels) {
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) {
    const row = y * (width * 4 + 1);
    raw[row] = 0;
    pixels.copy(raw, row + 1, y * width * 4, (y + 1) * width * 4);
  }

  return Buffer.concat([
    signature(),
    chunk("IHDR", ihdr(width, height)),
    chunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
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

function distanceToSegment(px, py, ax, ay, bx, by) {
  const dx = bx - ax;
  const dy = by - ay;
  if (dx === 0 && dy === 0) return Math.hypot(px - ax, py - ay);
  const t = clamp(((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy), 0, 1);
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
}

function lerp(from, to, amount) {
  return Math.round(from + (to - from) * amount);
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}
