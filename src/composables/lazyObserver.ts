/**
 * 全局共用的一个 IntersectionObserver。
 *
 * 之前每个 LazyImage 各建一个观察者，媒体库上千项时就是上千个观察者，
 * 光是创建和维护它们就能把低端机的主线程卡住 —— 这才是大列表卡顿的主要来源，
 * 不是 DOM 节点本身。共用一个实例后，这项开销基本归零。
 *
 * 每个元素只回调一次（图片一旦开始加载就不需要再观察了）。
 */

type Callback = () => void

const callbacks = new WeakMap<Element, Callback>()
let observer: IntersectionObserver | null = null

function getObserver(): IntersectionObserver {
  if (observer) return observer

  observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue
        const cb = callbacks.get(entry.target)
        if (!cb) continue
        // 先注销再回调：回调里可能同步改 DOM，避免重复触发
        callbacks.delete(entry.target)
        observer?.unobserve(entry.target)
        cb()
      }
    },
    // 提前一屏开始加载，滚动时不会看到空位
    { rootMargin: '400px 0px' },
  )
  return observer
}

export function observeOnce(el: Element, cb: Callback) {
  callbacks.set(el, cb)
  getObserver().observe(el)
}

export function unobserve(el: Element) {
  callbacks.delete(el)
  observer?.unobserve(el)
}
