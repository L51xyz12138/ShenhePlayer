import { computed, onBeforeUnmount, onMounted, ref, type Ref } from 'vue'

/**
 * 等宽网格的窗口化渲染：只渲染可视区域附近的几行。
 *
 * 为什么需要：媒体库动辄几千项，全量渲染意味着几千个 DOM 节点，
 * 而且每张海报的懒加载都要挂一个 IntersectionObserver —— 低端机上
 * 光是建这些观察者就会卡住主线程。
 *
 * 做法是撑起等高的上下留白，把不可见的行整个从 DOM 里拿掉。
 * 行高不硬算（卡片里有几行文字、字号会随系统缩放），而是渲染出来之后
 * 量一次真实高度，量到了再用。
 */
export function useGridWindow<T>(
  items: Ref<T[]>,
  options: {
    /** 卡片最小宽度，和 CSS 里 minmax() 的第一个参数保持一致（rem） */
    minCardRem: number
    /** 列间距（rem） */
    gapXRem: number
    /** 行间距（rem） */
    gapYRem: number
    /** 行高估算值，量到真实值之前用（px） */
    estimatedRowPx: number
    /** 上下各多渲染几行，滚动时不会看到空白 */
    overscan?: number
  },
) {
  const gridEl = ref<HTMLElement>()
  const cols = ref(1)
  const rowHeight = ref(options.estimatedRowPx)
  const scrollTop = ref(0)
  const viewportH = ref(0)
  /** 网格在滚动容器内容里的纵向偏移 */
  const gridOffset = ref(0)

  const overscan = options.overscan ?? 2
  let scroller: HTMLElement | null = null
  let resizeObserver: ResizeObserver | undefined
  let measured = false

  const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16

  const totalRows = computed(() => Math.ceil(items.value.length / cols.value))

  const startRow = computed(() => {
    const top = scrollTop.value - gridOffset.value
    return Math.max(0, Math.floor(top / rowHeight.value) - overscan)
  })

  const endRow = computed(() => {
    const bottom = scrollTop.value - gridOffset.value + viewportH.value
    return Math.min(totalRows.value, Math.ceil(bottom / rowHeight.value) + overscan)
  })

  /** 找不到滚动容器时不做窗口化，宁可全量渲染也不能只显示两行 */
  const disabled = ref(false)

  const visibleItems = computed(() =>
    disabled.value
      ? items.value
      : items.value.slice(startRow.value * cols.value, endRow.value * cols.value),
  )

  /** 第一个被渲染的元素在整个列表里的下标，用于计算「首屏优先加载」 */
  const firstIndex = computed(() => (disabled.value ? 0 : startRow.value * cols.value))

  const padTop = computed(() => (disabled.value ? 0 : startRow.value * rowHeight.value))
  const padBottom = computed(() =>
    disabled.value ? 0 : Math.max(0, (totalRows.value - endRow.value) * rowHeight.value),
  )

  /** 往上找到真正带滚动条的祖先 */
  function findScroller(el: HTMLElement): HTMLElement | null {
    let node: HTMLElement | null = el.parentElement
    while (node) {
      const oy = getComputedStyle(node).overflowY
      if (oy === 'auto' || oy === 'scroll') return node
      node = node.parentElement
    }
    return null
  }

  function measureColumns() {
    const el = gridEl.value
    if (!el) return
    const unit = rem()
    const width = el.clientWidth
    const gapX = options.gapXRem * unit
    const min = options.minCardRem * unit
    // 和 CSS repeat(auto-fill, minmax(min, 1fr)) 的算法保持一致
    cols.value = Math.max(1, Math.floor((width + gapX) / (min + gapX)))
  }

  /** 真实行高只能等内容渲染出来才知道 */
  function measureRowHeight() {
    const el = gridEl.value
    const first = el?.firstElementChild as HTMLElement | null
    if (!first || !first.offsetHeight) return
    rowHeight.value = first.offsetHeight + options.gapYRem * rem()
    measured = true
  }

  function readGeometry() {
    const el = gridEl.value
    if (!el || !scroller) return
    scrollTop.value = scroller.scrollTop
    viewportH.value = scroller.clientHeight
    gridOffset.value =
      el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop
    // 数据是异步来的，挂载时还没有卡片可量，这里补一次
    if (!measured) measureRowHeight()
  }

  function onScroll() {
    if (!scroller) return
    scrollTop.value = scroller.scrollTop
    // 行高只量一次；量早了（还没渲染）会拿到 0
    if (!measured) measureRowHeight()
  }

  onMounted(() => {
    const el = gridEl.value
    if (!el) return

    scroller = findScroller(el)
    if (!scroller) {
      disabled.value = true
      return
    }

    measureColumns()
    readGeometry()

    scroller?.addEventListener('scroll', onScroll, { passive: true })

    resizeObserver = new ResizeObserver(() => {
      measureColumns()
      // 宽度一变卡片就变宽，行高得重新量
      measured = false
      measureRowHeight()
      readGeometry()
    })
    resizeObserver.observe(el)
    if (scroller) resizeObserver.observe(scroller)

    requestAnimationFrame(() => {
      measureRowHeight()
      readGeometry()
    })
  })

  onBeforeUnmount(() => {
    scroller?.removeEventListener('scroll', onScroll)
    resizeObserver?.disconnect()
  })

  return { gridEl, visibleItems, firstIndex, padTop, padBottom, cols, rowHeight, readGeometry }
}
