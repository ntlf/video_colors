import fs from "fs/promises";
// @ts-ignore
import trianglify from "trianglify";

async function main() {
  const input = process.argv[2];

  const data = await fs.readFile(input, "utf-8");
  const c = JSON.parse(data);

  const w = 3840;
  const h = 2160;
  const q = c.colors.length;

  const cellSize =
    Math.ceil(
      (q *
        Math.sqrt(
          (4 * h ** 2 + h * q * w - 8 * h * w + 4 * w ** 2) / (q - 16) ** 2
        ) -
        16 *
          Math.sqrt(
            (4 * h ** 2 + h * q * w - 8 * h * w + 4 * w ** 2) / (q - 16) ** 2
          ) +
        2 * h +
        2 * w) /
        (q - 16)
    ) * 1.5;

  const pattern = trianglify({
    height: h,
    width: w,
    cellSize: cellSize,
    xColors: c.colors,
  });

  pattern.polys
    // .sort((a, b) => a.centroid.x + a.centroid.y - (b.centroid.x + b.centroid.y))
    .forEach((p: any, i: number) => {
      p.color._rgb = [...(c.colors[i] || [0, 0, 0]), 1];
    });

  const canvas = pattern.toCanvas();

  let pngStream = canvas.createPNGStream();

  await fs.writeFile(input.replace(".json", ".png"), pngStream);
}

main();
