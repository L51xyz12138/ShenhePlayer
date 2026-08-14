// 生成应用图标。纯 Node 手写 PNG/ICO 编码，不为几张图标引入 sharp 之类的重依赖。
//
// 设计按 Apple 的图标做法：
// - 外形是**超椭圆**（squircle）而不是普通圆角矩形。圆角矩形的直线段和圆弧
//   衔接处曲率突变，放大看会「顶」一下；超椭圆曲率连续，这是 Apple 图标
//   看起来「圆润」的真正原因。
// - 单一色系的垂直渐变，不用撞色。上浅下深，模拟光从上方来。
// - 顶部内缘一条极淡的高光，底部一层极淡的暗角 —— 让平面有厚度。
// - 字形是圆角三角形，边缘半径和外形的圆润度呼应，并且留足留白。
import { deflateSync } from 'node:zlib'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const OUT = join(dirname(fileURLToPath(import.meta.url)), '../src-tauri/icons')
mkdirSync(OUT, { recursive: true })

// ---------------------------------------------------------------- PNG 编码

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

function encodePng(size, rgba) {
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(size, 0)
  ihdr.writeUInt32BE(size, 4)
  ihdr[8] = 8 // bit depth
  ihdr[9] = 6 // RGBA
  ihdr[10] = 0
  ihdr[11] = 0
  ihdr[12] = 0

  // 每行前加一个 filter 字节（0 = None）
  const stride = size * 4
  const raw = Buffer.alloc(size * (stride + 1))
  for (let y = 0; y < size; y++) {
    raw[y * (stride + 1)] = 0
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride)
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

// ---------------------------------------------------------------- 几何

const clamp01 = (v) => (v < 0 ? 0 : v > 1 ? 1 : v)
const lerp = (a, b, t) => a + (b - a) * t
const smoothstep = (t) => t * t * (3 - 2 * t)

/**
 * 超椭圆的近似有符号距离。
 * 隐函数 f = (|x|/a)^n + (|y|/b)^n - 1 本身不是距离，除以梯度模长后
 * 就是一阶近似，足够做抗锯齿。n 越大越接近方形，Apple 图标大致在 5 附近。
 */
function squircleSD(px, py, cx, cy, half, n) {
  const x = (px - cx) / half
  const y = (py - cy) / half
  const ax = Math.abs(x)
  const ay = Math.abs(y)
  if (ax < 1e-6 && ay < 1e-6) return -half

  const f = Math.pow(ax, n) + Math.pow(ay, n) - 1
  // ∇f，注意换算回像素尺度
  const gx = (n * Math.pow(ax, n - 1)) / half
  const gy = (n * Math.pow(ay, n - 1)) / half
  const g = Math.hypot(gx, gy)
  return g < 1e-9 ? f : f / g
}

/** 三角形的有符号距离（负 = 内部） */
function triangleSD(px, py, a, b, c) {
  const edge = (p, q) => {
    const ex = q[0] - p[0]
    const ey = q[1] - p[1]
    const wx = px - p[0]
    const wy = py - p[1]
    const t = clamp01((wx * ex + wy * ey) / (ex * ex + ey * ey))
    return Math.hypot(wx - ex * t, wy - ey * t)
  }
  const side = (p, q) => (q[0] - p[0]) * (py - p[1]) - (q[1] - p[1]) * (px - p[0])
  const d = Math.min(edge(a, b), edge(b, c), edge(c, a))
  const s1 = side(a, b) > 0
  const s2 = side(b, c) > 0
  const s3 = side(c, a) > 0
  return s1 === s2 && s2 === s3 ? -d : d
}

// ---------------------------------------------------------------- 绘制

// 单色系蓝：上浅下深，模拟顶光
const TOP = [0x5c, 0xac, 0xff]
const BOTTOM = [0x00, 0x54, 0xd6]

function render(size) {
  // 小尺寸要更多采样才不毛边
  const SS = size <= 48 ? 6 : 4
  const buf = Buffer.alloc(size * size * 4)
  const S = size

  const pad = S * 0.045
  const half = S / 2 - pad
  const cx = S / 2
  const cy = S / 2

  // 播放三角：视觉重心比几何中心略靠右，所以整体右移一点点
  const triR = S * 0.055 // 圆角半径
  const ax = S * 0.405
  const bx = S * 0.735
  const top = S * 0.295
  const bot = S * 0.705
  // 顶点先按圆角半径内缩，再用 SDF 外扩回去，这样三角形整体大小不变
  const A = [ax + triR * 0.9, top + triR * 1.5]
  const B = [ax + triR * 0.9, bot - triR * 1.5]
  const C = [bx - triR * 1.2, (top + bot) / 2]

  for (let y = 0; y < S; y++) {
    for (let x = 0; x < S; x++) {
      let r = 0
      let g = 0
      let b = 0
      let a = 0

      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const px = x + (sx + 0.5) / SS
          const py = y + (sy + 0.5) / SS

          const d = squircleSD(px, py, cx, cy, half, 5)
          const cover = clamp01(0.5 - d)
          if (cover <= 0) continue

          // 垂直渐变
          const t = clamp01((py - (cy - half)) / (half * 2))
          let cr = lerp(TOP[0], BOTTOM[0], smoothstep(t))
          let cg = lerp(TOP[1], BOTTOM[1], smoothstep(t))
          let cb = lerp(TOP[2], BOTTOM[2], smoothstep(t))

          // 顶部内缘高光：只在离边缘很近的一圈里提亮，像光打在圆角上
          const rim = clamp01(1 - Math.abs(d) / (S * 0.03)) * clamp01(1 - t * 2.6)
          cr += (255 - cr) * rim * 0.34
          cg += (255 - cg) * rim * 0.34
          cb += (255 - cb) * rim * 0.34

          // 底部暗角，给一点厚度
          const vignette = clamp01((t - 0.62) / 0.38) * 0.12
          cr *= 1 - vignette
          cg *= 1 - vignette
          cb *= 1 - vignette

          // 圆角播放三角
          const tri = triangleSD(px, py, A, B, C) - triR
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

// ---------------------------------------------------------------- ICO 打包

/** Vista+ 的 ICO 允许直接内嵌 PNG */
function encodeIco(entries) {
  const header = Buffer.alloc(6)
  header.writeUInt16LE(0, 0)
  header.writeUInt16LE(1, 2)
  header.writeUInt16LE(entries.length, 4)

  const dir = Buffer.alloc(16 * entries.length)
  let offset = header.length + dir.length

  entries.forEach((e, idx) => {
    const o = idx * 16
    // 256 在这里要写 0
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

const pngFor = (size) => encodePng(size, render(size))

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

const ico = encodeIco([16, 20, 24, 32, 48, 64, 128, 256].map((size) => ({ size, png: pngFor(size) })))
writeFileSync(join(OUT, 'icon.ico'), ico)
console.log('  icon.ico (16/20/24/32/48/64/128/256)')
console.log('图标生成完成 ->', OUT)
