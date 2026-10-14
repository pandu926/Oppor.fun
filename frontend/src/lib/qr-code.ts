import { encodeQR, type QrOpts } from "qr";

// Cuer expects the symbol matrix without its quiet zone. Recent qr releases
// require a positive border; remove that padding before Cuer renders the SVG.
// RainbowKit supplies the visual quiet zone around the rendered symbol.
export function create(
  value: string,
  options: { errorCorrection?: QrOpts["ecc"]; version?: number } = {},
) {
  const grid = encodeQR(value, "raw", {
    border: 1,
    ecc: options.errorCorrection,
    scale: 1,
    version: options.version,
  })
    .slice(1, -1)
    .map((row) => row.slice(1, -1));
  return { edgeLength: grid.length, finderLength: 7, grid, value };
}
