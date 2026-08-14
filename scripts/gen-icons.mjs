// 生成应用图标：圆角渐变方块 + 播放三角。
// 纯 Node 手写 PNG/ICO 编码，避免为了几张图标引入 sharp 之类的重依赖。
import { deflateSync } from 'node:zlib'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const OUT = join(dirname(fileURLToPath(import.meta.url)), '../src-tauri/icons')
mkdirSync(OUT, { recursive: true })

// ---------- PNG 编码 ----------
const CRC_TABLE = (() => {
  const t = new Int32Array(256)
  for (let n = 0; n < 256; n++) {
    let c = n
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    t[n] = c
  }
  return t
})()

function crc32(buf) {
  let c = -1
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8)
  return (c ^ -1) >>> 0
}

function chunk(type, data) {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length)
  const typeBuf = Buffer.from(type, 'ascii')
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])))
  return Buffer.concat([len, typeBuf, data, crc])
}

function encodePng(width, height, rgba) {
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width, 0)
  ihdr.writeUInt32BE(height, 4)
  ihdr[8] = 8 // bit depth
  ihdr[9] = 6 // RGBA
  ihdr[10] = 0
  ihdr[11] = 0
  ihdr[12] = 0

  // 每行前加一个 filter 字节（0 = None）
  const raw = Buffer.alloc(height * (width * 4 + 1))
  for (let y = 0; y < height; y++) {
    raw[y * (width * 4 + 1)] = 0
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4)
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

// ---------- 图形 ----------
const lerp = (a, b, t) => a + (b - a) * t
const clamp01 = (v) => (v < 0 ? 0 : v > 1 ? 1 : v)

/** 圆角矩形的有符号距离场，用于抗锯齿 */
function roundedRectSD(px, py, cx, cy, hw, hh, r) {
  const qx = Math.abs(px - cx) - (hw - r)
  const qy = Math.abs(py - cy) - (hh - r)
  const ox = Math.max(qx, 0)
  const oy = Math.max(qy, 0)
  return Math.hypot(ox, oy) + Math.min(Math.max(qx, qy), 0) - r
}

/** 点在三角形内的有符号距离（负 = 内部） */
function triangleSD(px, py, a, b, c) {
  const edge = (p, q) => {
    const ex = q[0] - p[0]
    const ey = q[1] - p[1]
    const wx = px - p[0]
    const wy = py - p[1]
    const t = clamp01((wx * ex + wy * ey) / (ex * ex + ey * ey))
    return Math.hypot(wx - ex * t, wy - ey * t)
  }
  const sign = (p, q) => (q[0] - p[0]) * (py - p[1]) - (q[1] - p[1]) * (px - p[0])
  const d = Math.min(edge(a, b), edge(b, c), edge(c, a))
  const s1 = sign(a, b) > 0
  const s2 = sign(b, c) > 0
  const s3 = sign(c, a) > 0
  const inside = s1 === s2 && s2 === s3
  return inside ? -d : d
}

function render(size) {
  const SS = 3 // 3x3 超采样抗锯齿
  const buf = Buffer.alloc(size * size * 4)
  const S = size

  for (let y = 0; y < S; y++) {
    for (let x = 0; x < S; x++) {
      let r = 0, g = 0, b = 0, a = 0

      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const px = x + (sx + 0.5) / SS
          const py = y + (sy + 0.5) / SS

          // 外圈圆角方块，留 6% 边距
          const pad = S * 0.06
          const d = roundedRectSD(px, py, S / 2, S / 2, S / 2 - pad, S / 2 - pad, S * 0.235)
          const cover = clamp01(0.5 - d)
          if (cover <= 0) continue

          // 左上 -> 右下 的蓝紫渐变
          const t = clamp01((px / S) * 0.6 + (py / S) * 0.4)
          let cr = lerp(0x3d, 0x8b, t)
          let cg = lerp(0x6f, 0x5c, t)
          let cb = lerp(0xff, 0xf0, t)

          // 顶部高光，让图标有立体感
          const gloss = clamp01(1 - py / (S * 0.55)) * 0.16
          cr += (255 - cr) * gloss
          cg += (255 - cg) * gloss
          cb += (255 - cb) * gloss

          // 播放三角（略微右移以视觉居中）
          const tri = triangleSD(
            px, py,
            [S * 0.395, S * 0.305],
            [S * 0.395, S * 0.695],
            [S * 0.735, S * 0.5],
          )
          const triCover = clamp01(0.5 - tri)
          if (triCover > 0) {
            cr = lerp(cr, 255, triCover)
            cg = lerp(cg, 255, triCover)
            cb = lerp(cb, 255, triCover)
          }

          r += cr * cover
          g += cg * cover
          b += cb * cover
          a += cover
        }
      }

      const n = SS * SS
      const alpha = a / n
      const i = (y * S + x) * 4
      if (alpha > 0.0001) {
        buf[i] = Math.round(r / a)
        buf[i + 1] = Math.round(g / a)
        buf[i + 2] = Math.round(b / a)
        buf[i + 3] = Math.round(alpha * 255)
      }
    }
  }
  return buf
}

// ---------- ICO 打包（Vista+ 支持直接内嵌 PNG） ----------
function encodeIco(entries) {
  const header = Buffer.alloc(6)
  header.writeUInt16LE(0, 0)
  header.writeUInt16LE(1, 2)
  header.writeUInt16LE(entries.length, 4)

  const dir = Buffer.alloc(16 * entries.length)
  let offset = header.length + dir.length

  entries.forEach((e, idx) => {
    const o = idx * 16
    dir[o] = e.size >= 256 ? 0 : e.size
    dir[o + 1] = e.size >= 256 ? 0 : e.size
    dir[o + 2] = 0
    dir[o + 3] = 0
    dir.writeUInt16LE(1, o + 4)
    dir.writeUInt16LE(32, o + 6)
    dir.writeUInt32LE(e.png.length, o + 8)
    dir.writeUInt32LE(offset, o + 12)
    offset += e.png.length
  })

  return Buffer.concat([header, dir, ...entries.map((e) => e.png)])
}

const pngFor = (size) => encodePng(size, size, render(size))

const files = {
  '32x32.png': 32,
  '128x128.png': 128,
  '128x128@2x.png': 256,
  'icon.png': 512,
}

for (const [name, size] of Object.entries(files)) {
  writeFileSync(join(OUT, name), pngFor(size))
  console.log(`  ${name} (${size}x${size})`)
}

const ico = encodeIco([16, 32, 48, 64, 128, 256].map((size) => ({ size, png: pngFor(size) })))
writeFileSync(join(OUT, 'icon.ico'), ico)
console.log(`  icon.ico (16/32/48/64/128/256)`)
console.log('图标生成完成 ->', OUT)
