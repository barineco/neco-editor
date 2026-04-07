//! Tree-based edit history with undo, redo, branching, and checkpointing.
//!
//! This crate provides [`EditHistory`], a text-editing history that records
//! edits as nodes in a [`neco_tree::CursoredTree`].  When the user undoes
//! and then makes a new edit, a new branch is created instead of discarding
//! the old timeline.
//!
//! Two recording modes are supported:
//!
//! - [`EntryKind::Reversible`]: stores forward and inverse [`TextPatch`]es.
//!   Inverse patches are derived automatically from the current text and the
//!   forward patches.
//! - [`EntryKind::Snapshot`]: stores a complete text snapshot.  Use this for
//!   operations where computing a delta is impractical (e.g. external file
//!   reload).
//!
//! Periodic checkpoints (full snapshots) are inserted automatically so that
//! [`jump_to`](EditHistory::jump_to) can reach distant nodes efficiently.

use neco_textpatch::{apply_patches, inverse_patches, TextPatch};
use neco_tree::{CursoredTree, PrunePolicy, Tree};

// ---------------------------------------------------------------------------
// EntryKind
// ---------------------------------------------------------------------------

/// Determines how an edit can be undone/redone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// Reversible via forward/inverse patches.
    Reversible,
    /// Restored from a full text snapshot.
    Snapshot,
}

// ---------------------------------------------------------------------------
// HistoryEntry
// ---------------------------------------------------------------------------

/// Data stored in each history node.
///
/// Fields are private.  Use the accessor methods to read them.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    label: String,
    kind: EntryKind,
    forward_patches: Option<Vec<TextPatch>>,
    inverse_patches: Option<Vec<TextPatch>>,
    snapshot: Option<String>,
    checkpoint: Option<String>,
}

impl HistoryEntry {
    /// Human-readable label for this edit.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// How this entry can be reversed.
    pub fn kind(&self) -> EntryKind {
        self.kind
    }

    /// Forward patches (present for [`EntryKind::Reversible`]).
    pub fn forward_patches(&self) -> Option<&[TextPatch]> {
        self.forward_patches.as_deref()
    }

    /// Inverse patches (present for [`EntryKind::Reversible`]).
    pub fn inverse_patches(&self) -> Option<&[TextPatch]> {
        self.inverse_patches.as_deref()
    }

    /// Full text snapshot (present for [`EntryKind::Snapshot`]).
    pub fn snapshot(&self) -> Option<&str> {
        self.snapshot.as_deref()
    }

    /// Checkpoint snapshot inserted for fast long-range jumps.
    pub fn checkpoint(&self) -> Option<&str> {
        self.checkpoint.as_deref()
    }
}

// ---------------------------------------------------------------------------
// UndoResult / RedoResult / JumpStep
// ---------------------------------------------------------------------------

/// Information returned by [`EditHistory::undo`].
#[derive(Debug, Clone)]
pub struct UndoResult {
    pub kind: EntryKind,
    pub inverse_patches: Option<Vec<TextPatch>>,
    pub snapshot: Option<String>,
    pub label: String,
}

/// Information returned by [`EditHistory::redo`].
#[derive(Debug, Clone)]
pub struct RedoResult {
    pub kind: EntryKind,
    pub forward_patches: Option<Vec<TextPatch>>,
    pub snapshot: Option<String>,
    pub label: String,
}

/// A single step in a [`EditHistory::jump_to`] path.
#[derive(Debug, Clone)]
pub enum JumpStep {
    /// Move toward the root (undo direction).
    Undo(UndoResult),
    /// Move toward a leaf (redo direction).
    Redo(RedoResult),
}

// ---------------------------------------------------------------------------
// EditHistory
// ---------------------------------------------------------------------------

/// Tree-structured edit history with cursor-based undo/redo.
///
/// The root node holds the initial text as a snapshot.  Each subsequent node
/// stores either reversible patches or a full snapshot.
///
/// `EditHistory` wraps a [`CursoredTree<HistoryEntry>`].  The cursor always
/// points to the *current* state.
#[derive(Debug, Clone)]
pub struct EditHistory {
    tree: CursoredTree<HistoryEntry>,
    checkpoint_interval: u32,
    edits_since_checkpoint: u32,
}

impl EditHistory {
    /// Create a new history with `initial_text` stored as the root snapshot.
    pub fn new(initial_text: &str) -> Self {
        let root_entry = HistoryEntry {
            label: String::new(),
            kind: EntryKind::Snapshot,
            forward_patches: None,
            inverse_patches: None,
            snapshot: Some(initial_text.to_string()),
            checkpoint: Some(initial_text.to_string()),
        };
        Self {
            tree: CursoredTree::new(root_entry),
            checkpoint_interval: 20,
            edits_since_checkpoint: 0,
        }
    }

    /// Record a reversible edit.
    ///
    /// `current_text` is the text *before* applying `forward`.  Inverse
    /// patches are computed automatically via
    /// [`neco_textpatch::inverse_patches`].
    ///
    /// Returns the id of the new history node.
    pub fn push_edit(&mut self, label: &str, current_text: &str, forward: Vec<TextPatch>) -> u64 {
        let inverse = inverse_patches(current_text, &forward);
        let checkpoint = self.maybe_checkpoint(current_text, &forward);

        let entry = HistoryEntry {
            label: label.to_string(),
            kind: EntryKind::Reversible,
            forward_patches: Some(forward),
            inverse_patches: Some(inverse),
            snapshot: None,
            checkpoint,
        };
        self.tree.push(entry)
    }

    /// Record a snapshot-based edit.
    ///
    /// Use this when computing a delta is impractical (e.g. external file
    /// reload).
    ///
    /// Returns the id of the new history node.
    pub fn push_snapshot(&mut self, label: &str, full_text: String) -> u64 {
        self.edits_since_checkpoint = 0;
        let entry = HistoryEntry {
            label: label.to_string(),
            kind: EntryKind::Snapshot,
            forward_patches: None,
            inverse_patches: None,
            snapshot: Some(full_text.clone()),
            checkpoint: Some(full_text),
        };
        self.tree.push(entry)
    }

    /// Undo: return the information needed to reverse the current edit, then
    /// move the cursor to the parent.
    ///
    /// Returns `None` when at the root (nothing to undo).
    pub fn undo(&mut self) -> Option<UndoResult> {
        if !self.tree.has_parent() {
            return None;
        }
        let entry = self.tree.current().value();
        let result = UndoResult {
            kind: entry.kind(),
            inverse_patches: entry.inverse_patches.clone(),
            snapshot: self.find_parent_snapshot(),
            label: entry.label.clone(),
        };
        self.tree.go_parent();
        Some(result)
    }

    /// Redo: move the cursor to the last (newest) child and return the
    /// information needed to replay that edit.
    ///
    /// Returns `None` when there are no children.
    pub fn redo(&mut self) -> Option<RedoResult> {
        if !self.tree.has_children() {
            return None;
        }
        self.tree.go_child_last();
        let entry = self.tree.current().value();
        Some(RedoResult {
            kind: entry.kind(),
            forward_patches: entry.forward_patches.clone(),
            snapshot: entry.snapshot.clone(),
            label: entry.label.clone(),
        })
    }

    /// Jump to an arbitrary node, returning the sequence of undo/redo steps
    /// along the path.
    ///
    /// The path goes from the current node up to the lowest common ancestor
    /// (LCA), then down to the target.  Returns `None` when `id` does not
    /// exist.
    pub fn jump_to(&mut self, id: u64) -> Option<Vec<JumpStep>> {
        let current_path = self.tree.find_path_to(self.tree.current_id())?;
        let target_path = self.tree.find_path_to(id)?;

        // Find LCA depth (length of shared prefix).
        let lca_depth = current_path
            .iter()
            .zip(target_path.iter())
            .take_while(|(a, b)| a == b)
            .count();

        let mut steps = Vec::new();

        // Undo from current up to LCA.
        let undo_count = current_path.len() - lca_depth;
        for _ in 0..undo_count {
            if let Some(result) = self.undo() {
                steps.push(JumpStep::Undo(result));
            }
        }

        // Redo from LCA down to target.
        let redo_indices = &target_path[lca_depth..];
        for &child_index in redo_indices {
            if !self.tree.has_children() {
                break;
            }
            self.tree.go_child(child_index);
            let entry = self.tree.current().value();
            steps.push(JumpStep::Redo(RedoResult {
                kind: entry.kind(),
                forward_patches: entry.forward_patches.clone(),
                snapshot: entry.snapshot.clone(),
                label: entry.label.clone(),
            }));
        }

        Some(steps)
    }

    /// `true` when undo is possible (not at root).
    pub fn can_undo(&self) -> bool {
        self.tree.has_parent()
    }

    /// `true` when redo is possible (current node has children).
    pub fn can_redo(&self) -> bool {
        self.tree.has_children()
    }

    /// Id of the current history node.
    pub fn current_id(&self) -> u64 {
        self.tree.current_id()
    }

    /// Label of the current history node.
    pub fn current_label(&self) -> &str {
        self.tree.current().value().label()
    }

    /// Entry data of the current history node.
    pub fn current_entry(&self) -> &HistoryEntry {
        self.tree.current().value()
    }

    /// Set the checkpoint interval (number of edits between automatic
    /// snapshots).  Default is 20.
    pub fn set_checkpoint_interval(&mut self, interval: u32) {
        self.checkpoint_interval = interval;
    }

    /// Prune old branches from the history tree.
    pub fn prune(&mut self, policy: PrunePolicy) {
        self.tree.prune(policy);
    }

    /// Read-only access to the underlying tree (e.g. for visualization).
    pub fn tree(&self) -> &Tree<HistoryEntry> {
        self.tree.tree()
    }

    /// Read-only access to the underlying cursored tree.
    pub fn cursored_tree(&self) -> &CursoredTree<HistoryEntry> {
        &self.tree
    }

    // -- private ------------------------------------------------------------

    fn maybe_checkpoint(&mut self, current_text: &str, forward: &[TextPatch]) -> Option<String> {
        self.edits_since_checkpoint += 1;
        if self.edits_since_checkpoint >= self.checkpoint_interval {
            self.edits_since_checkpoint = 0;
            apply_patches(current_text, forward).ok()
        } else {
            None
        }
    }

    fn find_parent_snapshot(&self) -> Option<String> {
        // For Snapshot entries, the parent's snapshot (or checkpoint)
        // is the text to restore.  Walk up to find the nearest snapshot
        // or checkpoint.
        let current_path = self.tree.cursor_path();
        if current_path.is_empty() {
            return None;
        }
        let parent_path = &current_path[..current_path.len() - 1];
        self.resolve_snapshot_at_path(parent_path)
    }

    fn resolve_snapshot_at_path(&self, path: &[usize]) -> Option<String> {
        // Walk from root along path, looking for the most recent snapshot
        // or checkpoint.
        let mut node = self.tree.root();
        let entry = node.value();
        let mut latest = entry.snapshot.as_ref().or(entry.checkpoint.as_ref());

        for &index in path {
            node = node.children().get(index)?;
            let e = node.value();
            if e.snapshot.is_some() {
                latest = e.snapshot.as_ref();
            } else if e.checkpoint.is_some() {
                latest = e.checkpoint.as_ref();
            }
        }
        latest.cloned()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use neco_textpatch::apply_patches;

    fn make_insert(offset: usize, text: &str) -> Vec<TextPatch> {
        vec![TextPatch::insert(offset, text)]
    }

    fn make_delete(start: usize, end: usize) -> Vec<TextPatch> {
        vec![TextPatch::delete(start, end).unwrap()]
    }

    fn make_replace(start: usize, end: usize, text: &str) -> Vec<TextPatch> {
        vec![TextPatch::replace(start, end, text).unwrap()]
    }

    // -- basic construction -------------------------------------------------

    #[test]
    fn new_creates_root_with_initial_snapshot() {
        let h = EditHistory::new("hello");
        assert_eq!(h.current_id(), 0);
        assert!(!h.can_undo());
        assert!(!h.can_redo());

        let root = h.tree().root().value();
        assert_eq!(root.kind(), EntryKind::Snapshot);
        assert_eq!(root.snapshot(), Some("hello"));
        assert_eq!(root.checkpoint(), Some("hello"));
    }

    // -- push_edit ----------------------------------------------------------

    #[test]
    fn push_edit_records_forward_and_inverse() {
        let mut h = EditHistory::new("abc");
        let text = "abc";
        let forward = make_replace(1, 2, "B");
        let id = h.push_edit("replace b", text, forward);

        assert_eq!(h.current_id(), id);
        assert!(h.can_undo());

        let entry = h.cursored_tree().current().value();
        assert_eq!(entry.kind(), EntryKind::Reversible);
        assert_eq!(entry.label(), "replace b");

        // Forward: replace [1..2) with "B"
        let fwd = entry.forward_patches().unwrap();
        assert_eq!(fwd.len(), 1);
        assert_eq!(fwd[0].replacement(), "B");

        // Inverse: replace [1..2) with "b"
        let inv = entry.inverse_patches().unwrap();
        assert_eq!(inv.len(), 1);
        assert_eq!(inv[0].replacement(), "b");
    }

    #[test]
    fn push_edit_inverse_roundtrip() {
        let original = "hello world";
        let mut h = EditHistory::new(original);

        let forward = make_replace(6, 11, "rust");
        h.push_edit("change world", original, forward.clone());

        let modified = apply_patches(original, &forward).unwrap();
        assert_eq!(modified, "hello rust");

        let entry = h.cursored_tree().current().value();
        let inv = entry.inverse_patches().unwrap();
        let restored = apply_patches(&modified, inv).unwrap();
        assert_eq!(restored, original);
    }

    // -- push_snapshot ------------------------------------------------------

    #[test]
    fn push_snapshot_stores_full_text() {
        let mut h = EditHistory::new("old");
        let id = h.push_snapshot("reload", "new content".to_string());

        assert_eq!(h.current_id(), id);
        let entry = h.cursored_tree().current().value();
        assert_eq!(entry.kind(), EntryKind::Snapshot);
        assert_eq!(entry.snapshot(), Some("new content"));
    }

    // -- undo / redo --------------------------------------------------------

    #[test]
    fn undo_returns_inverse_and_moves_to_parent() {
        let mut h = EditHistory::new("abc");
        h.push_edit("insert", "abc", make_insert(3, "d"));

        assert!(h.can_undo());
        let result = h.undo().unwrap();
        assert_eq!(result.kind, EntryKind::Reversible);
        assert_eq!(result.label, "insert");
        assert!(result.inverse_patches.is_some());
        assert_eq!(h.current_id(), 0);
        assert!(!h.can_undo());
    }

    #[test]
    fn undo_at_root_returns_none() {
        let mut h = EditHistory::new("text");
        assert!(h.undo().is_none());
    }

    #[test]
    fn redo_returns_forward_and_moves_to_child() {
        let mut h = EditHistory::new("abc");
        h.push_edit("insert", "abc", make_insert(3, "d"));
        h.undo();

        assert!(h.can_redo());
        let result = h.redo().unwrap();
        assert_eq!(result.kind, EntryKind::Reversible);
        assert_eq!(result.label, "insert");
        assert!(result.forward_patches.is_some());
        assert!(!h.can_redo());
    }

    #[test]
    fn redo_at_leaf_returns_none() {
        let mut h = EditHistory::new("text");
        h.push_edit("edit", "text", make_insert(4, "!"));
        assert!(h.redo().is_none());
    }

    // -- branching ----------------------------------------------------------

    #[test]
    fn undo_then_push_creates_new_branch() {
        let mut h = EditHistory::new("abc");
        let id1 = h.push_edit("first", "abc", make_insert(3, "1"));
        h.undo(); // back to root
        let id2 = h.push_edit("second", "abc", make_insert(3, "2"));

        assert_ne!(id1, id2);
        // Root should now have 2 children (two branches).
        assert_eq!(h.tree().root().child_count(), 2);
        assert_eq!(h.current_id(), id2);
    }

    // -- jump_to ------------------------------------------------------------

    #[test]
    fn jump_to_returns_undo_redo_steps() {
        let mut h = EditHistory::new("abc");
        let a = h.push_edit("a", "abc", make_insert(3, "d"));
        let _a1 = h.push_edit("a1", "abcd", make_insert(4, "e"));

        // Jump back to root
        let steps = h.jump_to(0).unwrap();
        assert_eq!(steps.len(), 2);
        assert!(matches!(steps[0], JumpStep::Undo(_)));
        assert!(matches!(steps[1], JumpStep::Undo(_)));
        assert_eq!(h.current_id(), 0);

        // Jump forward to a
        let steps = h.jump_to(a).unwrap();
        assert_eq!(steps.len(), 1);
        assert!(matches!(steps[0], JumpStep::Redo(_)));
        assert_eq!(h.current_id(), a);
    }

    #[test]
    fn jump_to_nonexistent_returns_none() {
        let mut h = EditHistory::new("abc");
        assert!(h.jump_to(999).is_none());
    }

    #[test]
    fn jump_to_current_returns_empty_steps() {
        let mut h = EditHistory::new("abc");
        let a = h.push_edit("a", "abc", make_insert(3, "d"));
        let steps = h.jump_to(a).unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn jump_to_through_snapshot_node() {
        let mut h = EditHistory::new("abc");
        h.push_edit("edit", "abc", make_insert(3, "d"));
        let snap_id = h.push_snapshot("reload", "xyz".to_string());
        h.push_edit("after reload", "xyz", make_insert(3, "!"));

        // Jump back to root, passing through snapshot node
        let steps = h.jump_to(0).unwrap();
        assert_eq!(steps.len(), 3);
        assert!(matches!(steps[0], JumpStep::Undo(_)));
        assert!(matches!(steps[1], JumpStep::Undo(_)));
        assert!(matches!(steps[2], JumpStep::Undo(_)));
        assert_eq!(h.current_id(), 0);

        // Jump forward to snapshot node
        let steps = h.jump_to(snap_id).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(h.current_id(), snap_id);
    }

    // -- checkpoint ---------------------------------------------------------

    #[test]
    fn checkpoint_is_inserted_at_interval() {
        let mut h = EditHistory::new("x");
        h.set_checkpoint_interval(3);

        let mut text = "x".to_string();
        for i in 0..5 {
            let ch = char::from(b'a' + i);
            let forward = make_insert(text.len(), &ch.to_string());
            h.push_edit(&format!("add {ch}"), &text, forward.clone());
            text = apply_patches(&text, &forward).unwrap();
        }

        // After 3 edits (interval=3), checkpoint should be set on edit #3.
        let node_3 = h.tree().find(3).unwrap().value();
        assert!(
            node_3.checkpoint().is_some(),
            "edit #3 should have a checkpoint"
        );

        // Edit #1, #2, #4 should not have checkpoints.
        let node_1 = h.tree().find(1).unwrap().value();
        assert!(node_1.checkpoint().is_none());
        let node_2 = h.tree().find(2).unwrap().value();
        assert!(node_2.checkpoint().is_none());
    }

    // -- can_undo / can_redo ------------------------------------------------

    #[test]
    fn can_undo_can_redo_reflect_cursor_position() {
        let mut h = EditHistory::new("abc");
        assert!(!h.can_undo());
        assert!(!h.can_redo());

        h.push_edit("edit", "abc", make_insert(3, "d"));
        assert!(h.can_undo());
        assert!(!h.can_redo());

        h.undo();
        assert!(!h.can_undo());
        assert!(h.can_redo());
    }

    // -- prune --------------------------------------------------------------

    #[test]
    fn prune_removes_old_branches() {
        let mut h = EditHistory::new("abc");
        h.push_edit("a", "abc", make_insert(3, "1"));
        h.undo();
        h.push_edit("b", "abc", make_insert(3, "2"));
        h.undo();
        h.push_edit("c", "abc", make_insert(3, "3"));

        // Root has 3 branches.
        assert_eq!(h.tree().root().child_count(), 3);

        h.prune(PrunePolicy::KeepLastN(1));
        // Only the newest branch survives.
        assert_eq!(h.tree().root().child_count(), 1);
    }

    // -- snapshot undo ------------------------------------------------------

    #[test]
    fn undo_snapshot_entry_provides_parent_snapshot() {
        let mut h = EditHistory::new("original");
        h.push_snapshot("reload", "reloaded".to_string());

        let result = h.undo().unwrap();
        assert_eq!(result.kind, EntryKind::Snapshot);
        assert_eq!(result.snapshot.as_deref(), Some("original"));
    }

    // -- multiple edits roundtrip -------------------------------------------

    #[test]
    fn multiple_edits_undo_redo_roundtrip() {
        let mut h = EditHistory::new("hello");
        let mut text = "hello".to_string();

        // Edit 1: insert " world"
        let fwd1 = make_insert(5, " world");
        h.push_edit("add world", &text, fwd1.clone());
        text = apply_patches(&text, &fwd1).unwrap();
        assert_eq!(text, "hello world");

        // Edit 2: replace "world" with "rust"
        let fwd2 = make_replace(6, 11, "rust");
        h.push_edit("change lang", &text, fwd2.clone());
        text = apply_patches(&text, &fwd2).unwrap();
        assert_eq!(text, "hello rust");

        // Undo edit 2
        let u2 = h.undo().unwrap();
        text = apply_patches(&text, u2.inverse_patches.as_ref().unwrap()).unwrap();
        assert_eq!(text, "hello world");

        // Undo edit 1
        let u1 = h.undo().unwrap();
        text = apply_patches(&text, u1.inverse_patches.as_ref().unwrap()).unwrap();
        assert_eq!(text, "hello");

        // Redo edit 1
        let r1 = h.redo().unwrap();
        text = apply_patches(&text, r1.forward_patches.as_ref().unwrap()).unwrap();
        assert_eq!(text, "hello world");

        // Redo edit 2
        let r2 = h.redo().unwrap();
        text = apply_patches(&text, r2.forward_patches.as_ref().unwrap()).unwrap();
        assert_eq!(text, "hello rust");
    }

    // -- delete roundtrip ---------------------------------------------------

    #[test]
    fn delete_undo_restores_text() {
        let mut h = EditHistory::new("abcdef");
        let fwd = make_delete(2, 4);
        h.push_edit("delete cd", "abcdef", fwd.clone());

        let modified = apply_patches("abcdef", &fwd).unwrap();
        assert_eq!(modified, "abef");

        let u = h.undo().unwrap();
        let restored = apply_patches(&modified, u.inverse_patches.as_ref().unwrap()).unwrap();
        assert_eq!(restored, "abcdef");
    }
}
