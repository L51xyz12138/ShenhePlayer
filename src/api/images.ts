import type { BaseItem } from '@/types'

/**
 * Emby 图片接口必须带 api_key，所以前端要知道服务器地址和 token。
 * 用模块级变量而不是 store，避免 store 之间的循环依赖，取值也更快。
 */
let ctx = { baseUrl: '', token: '' }

export function setImageContext(baseUrl: string, token: string) {
  ctx = { baseUrl: baseUrl.replace(/\/+$/, ''), token }
}

interface ImageOptions {
  maxHeight?: number
  maxWidth?: number
  quality?: number
}

function build(itemId: string, type: string, tag: string | null | undefined, opts: ImageOptions = {}) {
  if (!ctx.baseUrl) return ''
  const params = new URLSearchParams()
  params.set('quality', String(opts.quality ?? 90))
  if (tag) params.set('tag', tag)
  if (opts.maxHeight) params.set('maxHeight', String(opts.maxHeight))
  if (opts.maxWidth) params.set('maxWidth', String(opts.maxWidth))
  if (ctx.token) params.set('api_key', ctx.token)
  return `${ctx.baseUrl}/emby/Items/${itemId}/Images/${type}?${params}`
}

/** 竖版海报 2:3。剧集会回退到剧集所属剧的海报。 */
export function posterUrl(item: BaseItem, maxHeight = 480): string {
  if (item.imageTags?.Primary) {
    return build(item.id, 'Primary', item.imageTags.Primary, { maxHeight })
  }
  if (item.seriesId && item.seriesPrimaryImageTag) {
    return build(item.seriesId, 'Primary', item.seriesPrimaryImageTag, { maxHeight })
  }
  if (item.parentPrimaryImageItemId && item.parentPrimaryImageTag) {
    return build(item.parentPrimaryImageItemId, 'Primary', item.parentPrimaryImageTag, { maxHeight })
  }
  return ''
}

/** 横版背景图 16:9 */
export function backdropUrl(item: BaseItem, maxWidth = 1600): string {
  if (item.backdropImageTags?.length) {
    return build(item.id, 'Backdrop', item.backdropImageTags[0], { maxWidth })
  }
  if (item.parentBackdropItemId && item.parentBackdropImageTags?.length) {
    return build(item.parentBackdropItemId, 'Backdrop', item.parentBackdropImageTags[0], { maxWidth })
  }
  return ''
}

/** 横版缩略图，剧集列表用。没有 Thumb 就退回背景图或海报。 */
export function thumbUrl(item: BaseItem, maxWidth = 480): string {
  if (item.imageTags?.Thumb) {
    return build(item.id, 'Thumb', item.imageTags.Thumb, { maxWidth })
  }
  if (item.imageTags?.Primary && item.type === 'Episode') {
    return build(item.id, 'Primary', item.imageTags.Primary, { maxWidth })
  }
  if (item.parentThumbItemId && item.parentThumbImageTag) {
    return build(item.parentThumbItemId, 'Thumb', item.parentThumbImageTag, { maxWidth })
  }
  return backdropUrl(item, maxWidth) || posterUrl(item, Math.round(maxWidth * 1.5))
}

/** 片名 Logo（透明 PNG），详情页大图上叠加用 */
export function logoUrl(item: BaseItem, maxWidth = 520): string {
  if (item.imageTags?.Logo) {
    return build(item.id, 'Logo', item.imageTags.Logo, { maxWidth })
  }
  return ''
}

export function personUrl(personId: string, tag: string | null | undefined, maxHeight = 240): string {
  if (!tag) return ''
  return build(personId, 'Primary', tag, { maxHeight })
}
