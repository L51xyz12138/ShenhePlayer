import { invoke } from '@tauri-apps/api/core'
import type {
  AppInfo,
  BaseItem,
  DetectedPlayer,
  HomeData,
  ItemsQuery,
  PlaybackTarget,
  PlayerSettings,
  PlayerSnapshot,
  QueryResult,
  ServerSummary,
  SessionInfo,
  Settings,
  SystemInfo,
  UpdateInfo,
  UiSettings,
} from '@/types'

export * from './images'

// ---------------------------------------------------------------- 登录

export const connectServer = (url: string, allowInvalidCerts = false) =>
  invoke<SystemInfo>('connect_server', { url, allowInvalidCerts })

export const login = (
  url: string,
  username: string,
  password: string,
  allowInvalidCerts = false,
) => invoke<SessionInfo>('login', { url, username, password, allowInvalidCerts })

export const restoreSession = () => invoke<SessionInfo | null>('restore_session')
export const logout = () => invoke<void>('logout')
/** 断开但保留凭据，下次可直接切回来 */
export const disconnect = () => invoke<void>('disconnect')
export const savedServers = () => invoke<ServerSummary[]>('saved_servers')
export const forgetServer = (serverId: string) => invoke<void>('forget_server', { serverId })
export const switchServer = (serverId: string) =>
  invoke<SessionInfo | null>('switch_server', { serverId })

// ---------------------------------------------------------------- 媒体库

export const getViews = () => invoke<BaseItem[]>('get_views')
export const getHome = () => invoke<HomeData>('get_home')
export const getItems = (query: ItemsQuery) =>
  invoke<QueryResult<BaseItem>>('get_items', { query })
export const getItem = (itemId: string) => invoke<BaseItem>('get_item', { itemId })
export const getSeasons = (seriesId: string) => invoke<BaseItem[]>('get_seasons', { seriesId })
export const getEpisodes = (seriesId: string, seasonId?: string) =>
  invoke<BaseItem[]>('get_episodes', { seriesId, seasonId })
export const getSimilar = (itemId: string, limit = 12) =>
  invoke<BaseItem[]>('get_similar', { itemId, limit })
export const search = (term: string, limit = 40) => invoke<BaseItem[]>('search', { term, limit })
export const setFavorite = (itemId: string, favorite: boolean) =>
  invoke<void>('set_favorite', { itemId, favorite })
export const setPlayed = (itemId: string, played: boolean) =>
  invoke<void>('set_played', { itemId, played })

// ---------------------------------------------------------------- 播放

export const preparePlayback = (itemId: string, resume: boolean, mediaSourceId?: string) =>
  invoke<PlaybackTarget>('prepare_playback', { itemId, mediaSourceId, resume })

export const currentTarget = () => invoke<PlaybackTarget | null>('current_target')
export const startInternal = () => invoke<void>('start_internal')
export const playTestPattern = () => invoke<void>('play_test_pattern')
export const startExternal = () => invoke<void>('start_external')
export const stopPlayback = () => invoke<void>('stop_playback')

export const playerSetPause = (paused: boolean) => invoke<void>('player_set_pause', { paused })
export const playerSeek = (position: number) => invoke<void>('player_seek', { position })
export const playerSeekRelative = (delta: number) =>
  invoke<void>('player_seek_relative', { delta })
export const playerSetVolume = (volume: number) => invoke<void>('player_set_volume', { volume })
export const playerSetMuted = (muted: boolean) => invoke<void>('player_set_muted', { muted })
export const playerSetSpeed = (speed: number) => invoke<void>('player_set_speed', { speed })
export const playerSetTrack = (kind: 'audio' | 'sub', id: number) =>
  invoke<void>('player_set_track', { kind, id })
export const playerSnapshot = () => invoke<PlayerSnapshot>('player_snapshot')

export const listExternalPlayers = () => invoke<DetectedPlayer[]>('list_external_players')
export const reportExternalProgress = (position: number, finished: boolean) =>
  invoke<void>('report_external_progress', { position, finished })

// ---------------------------------------------------------------- 系统

export const getSettings = () => invoke<Settings>('get_settings')
export const updatePlayerSettings = (player: PlayerSettings) =>
  invoke<void>('update_player_settings', { player })
export const updateUiSettings = (ui: UiSettings) => invoke<void>('update_ui_settings', { ui })
export const appInfo = () => invoke<AppInfo>('app_info')
export const setFullscreen = (fullscreen: boolean) =>
  invoke<void>('set_fullscreen', { fullscreen })
export const isFullscreen = () => invoke<boolean>('is_fullscreen')

// 控制条在 overlay 窗口里，这些操作要落到 player 窗口上
export const playerMinimize = () => invoke<void>('player_minimize')
export const playerToggleMaximize = () => invoke<void>('player_toggle_maximize')
export const playerIsMaximized = () => invoke<boolean>('player_is_maximized')
export const playerStartDrag = () => invoke<void>('player_start_drag')

// ---------------------------------------------------------------- 更新

export const checkUpdate = () => invoke<UpdateInfo>('check_update')
export const openReleasePage = (url: string) => invoke<void>('open_release_page', { url })
/** 下载安装包到临时目录，返回文件路径。进度通过 update:progress 事件推送。 */
export const downloadUpdate = () => invoke<string>('download_update')
/** 启动安装程序，本进程随即退出 */
export const installUpdate = (path: string) => invoke<void>('install_update', { path })
