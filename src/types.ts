// 与 Rust 端 serde 序列化结果一一对应

export interface UserData {
  playbackPositionTicks?: number | null
  playCount: number
  isFavorite: boolean
  played: boolean
  playedPercentage?: number | null
  unplayedItemCount?: number | null
  lastPlayedDate?: string | null
}

export interface NameGuidPair {
  name: string
  id?: string | null
}

export interface Person {
  name: string
  id?: string | null
  role?: string | null
  type?: string | null
  primaryImageTag?: string | null
}

export interface MediaStream {
  codec?: string | null
  language?: string | null
  displayTitle?: string | null
  title?: string | null
  type: string
  index: number
  isDefault: boolean
  isForced: boolean
  isExternal: boolean
  height?: number | null
  width?: number | null
  bitRate?: number | null
  channels?: number | null
  channelLayout?: string | null
  videoRange?: string | null
}

export interface MediaSource {
  id: string
  name?: string | null
  path?: string | null
  container?: string | null
  size?: number | null
  bitrate?: number | null
  runTimeTicks?: number | null
  supportsDirectPlay: boolean
  supportsDirectStream: boolean
  supportsTranscoding: boolean
  mediaStreams: MediaStream[]
}

export interface BaseItem {
  id: string
  name: string
  originalTitle?: string | null
  type: string
  mediaType?: string | null
  collectionType?: string | null
  overview?: string | null
  taglines: string[]
  productionYear?: number | null
  premiereDate?: string | null
  officialRating?: string | null
  communityRating?: number | null
  criticRating?: number | null
  runTimeTicks?: number | null
  indexNumber?: number | null
  parentIndexNumber?: number | null
  childCount?: number | null
  status?: string | null
  path?: string | null
  container?: string | null

  seriesId?: string | null
  seriesName?: string | null
  seasonId?: string | null
  seasonName?: string | null
  parentId?: string | null

  imageTags: Record<string, string>
  backdropImageTags: string[]
  parentBackdropItemId?: string | null
  parentBackdropImageTags: string[]
  parentThumbItemId?: string | null
  parentThumbImageTag?: string | null
  parentPrimaryImageItemId?: string | null
  parentPrimaryImageTag?: string | null
  seriesPrimaryImageTag?: string | null

  genres: string[]
  studios: NameGuidPair[]
  people: Person[]
  userData?: UserData | null
  mediaSources: MediaSource[]
  mediaStreams: MediaStream[]
  providerIds: Record<string, string>
  isFolder: boolean
}

export interface QueryResult<T> {
  items: T[]
  totalRecordCount: number
}

export interface HomeSection {
  id: string
  title: string
  kind: 'resume' | 'nextup' | 'latest'
  parentId?: string | null
  items: BaseItem[]
}

export interface HomeData {
  views: BaseItem[]
  sections: HomeSection[]
  hero: BaseItem[]
}

export interface SessionInfo {
  serverId: string
  serverName: string
  serverUrl: string
  userId: string
  userName: string
  token: string
  avatarUrl?: string | null
}

export interface SystemInfo {
  serverName: string
  version: string
  id: string
  operatingSystem?: string | null
}

export interface ExternalSubtitle {
  index: number
  title: string
  language?: string | null
  url: string
  isDefault: boolean
  codec?: string | null
}

export interface PlaybackTarget {
  itemId: string
  mediaSourceId: string
  playSessionId: string
  url: string
  isDirect: boolean
  title: string
  subTitle?: string | null
  startPosition: number
  duration: number
  container?: string | null
  size?: number | null
  bitrate?: number | null
  audioStreams: MediaStream[]
  subtitleStreams: MediaStream[]
  videoStream?: MediaStream | null
  externalSubtitles: ExternalSubtitle[]
  defaultAudioIndex?: number | null
  defaultSubtitleIndex?: number | null
  backdropUrl?: string | null
}

export interface TrackInfo {
  id: number
  type: string
  title: string
  lang: string
  codec: string
  selected: boolean
  external: boolean
  default: boolean
  forced: boolean
  detail: string
}

export interface PlayerSnapshot {
  active: boolean
  paused: boolean
  idle: boolean
  buffering: boolean
  seeking: boolean
  position: number
  duration: number
  cacheTime: number
  volume: number
  muted: boolean
  speed: number
  width: number
  height: number
  fps: number
  hwdec: string
  videoCodec: string
  audioCodec: string
  tracks: TrackInfo[]
  audioTrack: number
  subTrack: number
  fileLoaded: boolean
}

export type QualityPreset = 'performance' | 'balanced' | 'quality'

export interface PlayerSettings {
  mode: 'internal' | 'external'
  mpvPath: string
  externalPath: string
  externalKind: string
  externalArgs: string
  quality: QualityPreset
  hwdec: string
  gpuNext: boolean
  volume: number
  subFontSize: number
  skipIntroSeconds: number
  autoNext: boolean
  fullscreenOnPlay: boolean
  maxBitrate: number
}

export type Theme = 'system' | 'light' | 'dark'

export interface UiSettings {
  theme: Theme
  accent: string
  gridSize: number
  showBackdrop: boolean
  reduceMotion: boolean
}

/** 服务器列表项。后端不下发 token，只给状态。 */
export interface ServerSummary {
  id: string
  name: string
  url: string
  username: string
  userId: string
  allowInvalidCerts: boolean
  lastUsed: number
  hasToken: boolean
  isActive: boolean
}

export interface ServerProfile {
  id: string
  name: string
  url: string
  username: string
  userId: string
  token: string
  allowInvalidCerts: boolean
  lastUsed: number
}

export interface Settings {
  deviceId: string
  servers: ServerProfile[]
  activeServer: string
  player: PlayerSettings
  ui: UiSettings
}

export interface DetectedPlayer {
  kind: string
  name: string
  path: string
}

export interface UpdateInfo {
  current: string
  latest: string
  available: boolean
  noRelease: boolean
  notes: string
  url: string
  /** Release 的 tag（带 v 前缀） */
  tag: string
}

export interface DownloadProgress {
  downloaded: number
  /** 服务器给的总长度，未知时为 0 */
  total: number
}

export interface AppInfo {
  version: string
  mpvPath?: string | null
  mpvAvailable: boolean
  configPath: string
}

export interface ItemsQuery {
  ParentId?: string
  IncludeItemTypes?: string
  ExcludeItemTypes?: string
  Recursive?: boolean
  SortBy?: string
  SortOrder?: string
  StartIndex?: number
  Limit?: number
  SearchTerm?: string
  Filters?: string
  Genres?: string
  Years?: string
  IsPlayed?: boolean
  Fields?: string
  EnableImageTypes?: string
  ImageTypeLimit?: number
  NameStartsWith?: string
}
