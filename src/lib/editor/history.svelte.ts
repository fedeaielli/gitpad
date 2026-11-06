/// Monaco does not say whether undo or redo has anything to do, but every change
/// it makes carries an `isUndoing` / `isRedoing` flag, and the model's
/// alternative version id goes back to the value it had before the edit. Follow
/// the version for undo and count the undo steps for redo: a keystroke count
/// drifts as soon as Monaco groups one undo step out of several changes.
export const undoState = $state({ canUndo: false, canRedo: false })

let base = 0
let redoDepth = 0

/// A model keeps its own history: the arrows start disabled on every switch.
export function resetUndoState(alternativeVersionId: number) {
  base = alternativeVersionId
  redoDepth = 0
  undoState.canUndo = false
  undoState.canRedo = false
}

/// `id` is the model's alternative version id after the change.
export function trackChange(id: number, isUndoing: boolean, isRedoing: boolean) {
  if (isUndoing) {
    redoDepth += 1
    undoState.canUndo = id !== base
    undoState.canRedo = true
    return
  }
  if (isRedoing) {
    redoDepth = Math.max(0, redoDepth - 1)
    undoState.canUndo = id !== base
    undoState.canRedo = redoDepth > 0
    return
  }
  redoDepth = 0
  undoState.canUndo = true
  undoState.canRedo = false
}
