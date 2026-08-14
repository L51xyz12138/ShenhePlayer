import type { BaseItem } from '@/types'

const TICKS_PER_SECOND = 10_000_000

export const ticksToSeconds = (ticks?: number | null) => (ticks ?? 0) / TICKS_PER_SECOND

/** 秒 -> 1:23:45 / 23:45 */
export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) seconds = 0
  const total = Math.floor(seconds)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  const pad = (n: number) => String(n).padStart(2, '0')
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`
}

/** 时长的中文口语表达：2 小时 15 分 */
export function formatDuration(ticks?: number | null): string {
  const seconds = ticksToSeconds(ticks)
  if (seconds <= 0) return ''
  const h = Math.floor(seconds / 3600)
  const m = Math.round((seconds % 3600) / 60)
  if (h > 0) return m > 0 ? `${h} 小时 ${m} 分` : `${h} 小时`
  return `${m} 分钟`
}

export function formatBytes(bytes?: number | null): string {
  if (!bytes) return ''
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let v = bytes
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  return `${v.toFixed(v >= 10 || i === 0 ? 0 : 1)} ${units[i]}`
}

export function formatBitrate(bps?: number | null): string {
  if (!bps) return ''
  return bps >= 1_000_000 ? `${(bps / 1_000_000).toFixed(1)} Mbps` : `${Math.round(bps / 1000)} Kbps`
}

/** 已看进度 0~1 */
export function playedFraction(item: BaseItem): number {
  const ud = item.userData
  if (!ud) return 0
  if (ud.played) return 1
  const pos = ticksToSeconds(ud.playbackPositionTicks)
  const total = ticksToSeconds(item.runTimeTicks)
  if (total <= 0 || pos <= 0) return 0
  return Math.min(pos / total, 1)
}

export function isResumable(item: BaseItem): boolean {
  const f = playedFraction(item)
  return f > 0.005 && f < 0.98
}

/** 剧集显示成 S01E05 */
export function episodeLabel(item: BaseItem): string {
  const s = item.parentIndexNumber
  const e = item.indexNumber
  if (s == null && e == null) return ''
  const pad = (n: number) => String(n).padStart(2, '0')
  if (s != null && e != null) return `S${pad(s)}E${pad(e)}`
  return e != null ? `第 ${e} 集` : `第 ${s} 季`
}

/** 卡片标题：剧集显示剧名，其它显示自身名字 */
export function displayTitle(item: BaseItem): string {
  if (item.type === 'Episode' && item.seriesName) return item.seriesName
  return item.name
}

/** 卡片副标题 */
export function displaySubtitle(item: BaseItem): string {
  switch (item.type) {
    case 'Episode': {
      const label = episodeLabel(item)
      return label ? `${label} · ${item.name}` : item.name
    }
    case 'Series':
      return item.productionYear ? `${item.productionYear}` : ''
    case 'Season':
      return item.childCount ? `${item.childCount} 集` : ''
    default:
      return item.productionYear ? `${item.productionYear}` : ''
  }
}

export function formatDate(input?: string | null): string {
  if (!input) return ''
  const d = new Date(input)
  if (Number.isNaN(d.getTime())) return ''
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(
    d.getDate(),
  ).padStart(2, '0')}`
}

export function formatRating(v?: number | null): string {
  return v == null ? '' : v.toFixed(1)
}

/** 中文类型名 */
export function typeLabel(type: string): string {
  const map: Record<string, string> = {
    Movie: '电影',
    Series: '剧集',
    Season: '季',
    Episode: '集',
    BoxSet: '合集',
    MusicAlbum: '专辑',
    Audio: '音乐',
    Person: '人物',
    Video: '视频',
    Folder: '文件夹',
  }
  return map[type] ?? type
}
