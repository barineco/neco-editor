/**
 * CompositionTracker callbacks.
 * All callbacks fire synchronously with no RAF, timeout, or microtask hop.
 */
export interface CompositionCallbacks {
  /** Pending text changed. Empty string means cleared. */
  onPending(text: string): void
  /** Committed text was produced. */
  onCommit(text: string): void
  /** Composition was explicitly cancelled. */
  onCancel(): void
}

/**
 * Pure state machine for IME composition sequences.
 * It has no dependency on DOM or rendering.
 */
export class CompositionTracker {
  #pending = ''
  #active = false
  readonly #cb: CompositionCallbacks

  constructor(callbacks: CompositionCallbacks) {
    this.#cb = callbacks
  }

  get pending(): string {
    return this.#pending
  }

  get isActive(): boolean {
    return this.#active
  }

  handleStart(): void {
    this.#pending = ''
    this.#active = true
    this.#cb.onPending('')
  }

  handleUpdate(text: string): void {
    if (!this.#active) return
    this.#pending = text
    this.#cb.onPending(text)
  }

  handleEnd(text: string): void {
    if (!this.#active) return
    this.#active = false
    this.#pending = ''
    this.#cb.onCommit(text)
  }

  cancel(): void {
    if (!this.#active) return
    this.#active = false
    this.#pending = ''
    this.#cb.onCancel()
  }
}
