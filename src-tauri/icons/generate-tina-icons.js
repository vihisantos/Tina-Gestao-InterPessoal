// Gera os ícones do aplicativo a partir da arte da Tina (rosa).
// Uso: node src-tauri/icons/generate-tina-icons.js
//
// A arte de origem é o PNG embutido em "public/tina person/tina (1).svg".
// Saída: PNGs em src-tauri/icons, um icon.ico multi-resolução e a cópia
// usada no cabeçalho do app (src-frontend/static/tina.png).

const fs = require('fs');
const path = require('path');
const { createCanvas, loadImage } = require('canvas');

const BASE_PATH = __dirname;
const SVG_PATH = path.join(BASE_PATH, '..', '..', 'public', 'tina person', 'tina (1).svg');
const STATIC_PATH = path.join(BASE_PATH, '..', '..', 'src-frontend', 'static');

const SIZES = [16, 24, 32, 48, 64, 128, 256];

// Área realmente ocupada pela personagem dentro da arte 500x500.
const CROP = { x: 91, y: 84, w: 312, h: 341 };

function readTinaPng() {
  const svg = fs.readFileSync(SVG_PATH, 'utf8');
  const match = svg.match(/base64,([A-Za-z0-9+/=]+)/);
  if (!match) throw new Error('PNG embutido não encontrado no SVG da Tina');
  return Buffer.from(match[1], 'base64');
}

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

function renderTile(tina, size) {
  const canvas = createCanvas(size, size);
  const ctx = canvas.getContext('2d');

  const inset = Math.max(0.5, size * 0.02);
  const radius = size * 0.225;

  const bg = ctx.createLinearGradient(0, 0, 0, size);
  bg.addColorStop(0, '#fff7fa');
  bg.addColorStop(1, '#ffe2ed');

  roundRect(ctx, inset, inset, size - inset * 2, size - inset * 2, radius);
  ctx.fillStyle = bg;
  ctx.fill();

  ctx.strokeStyle = '#ffc7db';
  ctx.lineWidth = Math.max(1, size * 0.015);
  ctx.stroke();

  const targetH = size * 0.7;
  const targetW = targetH * (CROP.w / CROP.h);

  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(
    tina,
    CROP.x,
    CROP.y,
    CROP.w,
    CROP.h,
    (size - targetW) / 2,
    (size - targetH) / 2 + size * 0.01,
    targetW,
    targetH
  );

  return canvas;
}

// Entrada ICO em DIB 32 bits (XOR + máscara AND), formato aceito pelo RC.exe.
function encodeIco(tina) {
  const images = SIZES.map((size) => {
    const ctx = renderTile(tina, size).getContext('2d');
    return { size, pixels: ctx.getImageData(0, 0, size, size).data };
  });

  const header = Buffer.alloc(6);
  header.writeUInt16LE(1, 2); // 1 = ICO
  header.writeUInt16LE(images.length, 4);

  let offset = 6 + images.length * 16;
  const entries = [];
  const payloads = [];

  for (const { size, pixels } of images) {
    const rowSize = size * 4;
    const xor = Buffer.alloc(rowSize * size);
    for (let y = 0; y < size; y++) {
      const sourceRow = (size - 1 - y) * rowSize; // DIB é bottom-up
      for (let x = 0; x < size; x++) {
        const i = sourceRow + x * 4;
        const o = y * rowSize + x * 4;
        xor[o] = pixels[i + 2];
        xor[o + 1] = pixels[i + 1];
        xor[o + 2] = pixels[i];
        xor[o + 3] = pixels[i + 3];
      }
    }

    const maskRowSize = Math.ceil(size / 32) * 4;
    const mask = Buffer.alloc(maskRowSize * size);
    for (let y = 0; y < size; y++) {
      const row = (size - 1 - y) * size;
      for (let x = 0; x < size; x++) {
        if (pixels[(row + x) * 4 + 3] < 16) {
          mask[y * maskRowSize + (x >> 3)] |= 0x80 >> (x & 7);
        }
      }
    }

    const info = Buffer.alloc(40);
    info.writeUInt32LE(40, 0);
    info.writeInt32LE(size, 4);
    info.writeInt32LE(size * 2, 8); // altura dobrada: imagem + máscara
    info.writeUInt16LE(1, 12);
    info.writeUInt16LE(32, 14);
    info.writeUInt32LE(xor.length + mask.length, 20);

    const payload = Buffer.concat([info, xor, mask]);

    const entry = Buffer.alloc(16);
    entry.writeUInt8(size === 256 ? 0 : size, 0);
    entry.writeUInt8(size === 256 ? 0 : size, 1);
    entry.writeUInt16LE(1, 4);
    entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(payload.length, 8);
    entry.writeUInt32LE(offset, 12);

    offset += payload.length;
    entries.push(entry);
    payloads.push(payload);
  }

  return Buffer.concat([header, ...entries, ...payloads]);
}

async function main() {
  const source = readTinaPng();
  const tina = await loadImage(source);

  fs.mkdirSync(STATIC_PATH, { recursive: true });
  fs.writeFileSync(path.join(STATIC_PATH, 'tina.png'), source);

  for (const size of SIZES) {
    const png = renderTile(tina, size).toBuffer('image/png');
    fs.writeFileSync(path.join(BASE_PATH, `${size}x${size}.png`), png);
  }

  fs.writeFileSync(
    path.join(BASE_PATH, '128x128@2x.png'),
    renderTile(tina, 256).toBuffer('image/png')
  );
  fs.writeFileSync(
    path.join(BASE_PATH, 'icon.png'),
    renderTile(tina, 512).toBuffer('image/png')
  );
  fs.writeFileSync(path.join(BASE_PATH, 'icon.ico'), encodeIco(tina));

  const icoSize = fs.statSync(path.join(BASE_PATH, 'icon.ico')).size;
  console.log(`Ícones gerados a partir da Tina. icon.ico: ${icoSize} bytes`);
}

main().catch((error) => {
  console.error('Falha ao gerar ícones:', error);
  process.exit(1);
});
