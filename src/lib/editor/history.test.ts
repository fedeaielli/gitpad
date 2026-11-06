import { beforeEach, describe, expect, it } from 'vitest'
import { resetUndoState, trackChange, undoState } from './history.svelte'

describe('undo availability', () => {
  beforeEach(() => resetUndoState(3))

  it('starts with both arrows disabled', () => {
    expect(undoState.canUndo).toBe(false)
    expect(undoState.canRedo).toBe(false)
  })

  it('undoes back to the version the tab opened on', () => {
    trackChange(4, false, false)
    expect(undoState).toMatchObject({ canUndo: true, canRedo: false })

    trackChange(3, true, false)
    expect(undoState).toMatchObject({ canUndo: false, canRedo: true })

    trackChange(4, false, true)
    expect(undoState).toMatchObject({ canUndo: true, canRedo: false })
  })

  it('keeps undo available when the history goes deeper than the last step', () => {
    trackChange(4, false, false)
    trackChange(5, false, false)
    trackChange(4, true, false)
    expect(undoState).toMatchObject({ canUndo: true, canRedo: true })
  })

  it('turns redo off after the last undone step is redone', () => {
    trackChange(4, false, false)
    trackChange(3, true, false)
    trackChange(4, false, true)
    expect(undoState).toMatchObject({ canUndo: true, canRedo: false })
  })

  it('drops redo as soon as the user types after undoing', () => {
    trackChange(4, false, false)
    trackChange(3, true, false)
    trackChange(9, false, false)
    expect(undoState).toMatchObject({ canUndo: true, canRedo: false })
  })
})
