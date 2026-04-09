//! WebAssembly bindings for neco-editor.
//!
//! Two-layer architecture:
//! - Layer 1: internal pure-Rust functions (testable without WASM)
//! - Layer 2: public `#[wasm_bindgen]` wrappers with JS marshalling

use js_sys::{Array, Object, Reflect};
use neco_editor::neco_decor::DecorationSet;
use neco_editor::neco_history::EditHistory;
use neco_editor::neco_textpatch::{TextPatch, TextPatchError};
use neco_editor::neco_textview::{RangeChange, Selection, Utf16Mapping};
use neco_editor::neco_wrap::{LayoutMode, LineLayoutPolicy, WidthPolicy, WrapMap, WrapPolicy};
use neco_editor::{EditorBuffer, IndentStyle};
use neco_editor_search::{SearchError, SearchMatch, SearchQuery};
use neco_editor_viewport::{self, Rect, ViewportError, ViewportLayout, ViewportMetrics};
use neco_syntax_textmate::{GrammarSet, SyntaxHighlighter, TokenKind, TokenSpan};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct WasmErrorData {
    domain: &'static str,
    code: &'static str,
    message: String,
}

// ---------------------------------------------------------------------------
// JS helpers
// ---------------------------------------------------------------------------

fn set_prop(target: &Object, key: &str, value: JsValue) {
    Reflect::set(target, &JsValue::from_str(key), &value).expect("js object property set");
}

fn error_to_js_value(error: WasmErrorData) -> JsValue {
    let object = Object::new();
    set_prop(&object, "domain", JsValue::from_str(error.domain));
    set_prop(&object, "code", JsValue::from_str(error.code));
    set_prop(&object, "message", JsValue::from_str(&error.message));
    object.into()
}

fn rect_to_js_value(rect: &Rect) -> JsValue {
    let object = Object::new();
    set_prop(&object, "x", JsValue::from_f64(rect.x));
    set_prop(&object, "y", JsValue::from_f64(rect.y));
    set_prop(&object, "width", JsValue::from_f64(rect.width));
    set_prop(&object, "height", JsValue::from_f64(rect.height));
    object.into()
}

fn layout_mode_str(mode: LayoutMode) -> &'static str {
    match mode {
        LayoutMode::HorizontalLtr => "horizontal-ltr",
        LayoutMode::VerticalRl => "vertical-rl",
        LayoutMode::VerticalLr => "vertical-lr",
    }
}

fn visual_line_frame_to_js_value(frame: &neco_editor_viewport::VisualLineFrame) -> JsValue {
    let object = Object::new();
    set_prop(
        &object,
        "logicalLine",
        JsValue::from_f64(f64::from(frame.logical_line())),
    );
    set_prop(
        &object,
        "visualLine",
        JsValue::from_f64(f64::from(frame.visual_line())),
    );
    set_prop(
        &object,
        "inlineAdvance",
        JsValue::from_f64(f64::from(frame.inline_advance())),
    );
    set_prop(
        &object,
        "blockAdvance",
        JsValue::from_f64(f64::from(frame.block_advance())),
    );
    set_prop(
        &object,
        "layoutMode",
        JsValue::from_str(layout_mode_str(frame.layout_mode())),
    );
    object.into()
}

fn token_kind_str(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Keyword => "keyword",
        TokenKind::Type => "type",
        TokenKind::Function => "function",
        TokenKind::String => "string",
        TokenKind::Number => "number",
        TokenKind::Comment => "comment",
        TokenKind::Operator => "operator",
        TokenKind::Punctuation => "punctuation",
        TokenKind::Variable => "variable",
        TokenKind::Constant => "constant",
        TokenKind::Tag => "tag",
        TokenKind::Attribute => "attribute",
        TokenKind::Escape => "escape",
        TokenKind::Plain => "plain",
        _ => "plain",
    }
}

fn token_span_to_js_value(span: &TokenSpan) -> JsValue {
    let object = Object::new();
    set_prop(&object, "start", JsValue::from_f64(span.range.start as f64));
    set_prop(&object, "end", JsValue::from_f64(span.range.end as f64));
    set_prop(
        &object,
        "kind",
        JsValue::from_str(token_kind_str(span.kind)),
    );
    object.into()
}

fn search_match_to_js_value(m: &SearchMatch) -> JsValue {
    let object = Object::new();
    set_prop(
        &object,
        "start",
        JsValue::from_f64(m.range().start() as f64),
    );
    set_prop(&object, "end", JsValue::from_f64(m.range().end() as f64));
    set_prop(&object, "line", JsValue::from_f64(f64::from(m.line())));
    set_prop(&object, "column", JsValue::from_f64(f64::from(m.column())));
    object.into()
}

fn range_change_to_js_value(rc: &RangeChange) -> JsValue {
    let object = Object::new();
    set_prop(&object, "start", JsValue::from_f64(rc.start() as f64));
    set_prop(&object, "oldEnd", JsValue::from_f64(rc.old_end() as f64));
    set_prop(&object, "newEnd", JsValue::from_f64(rc.new_end() as f64));
    object.into()
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

fn map_textpatch_error(e: TextPatchError) -> WasmErrorData {
    WasmErrorData {
        domain: "editor",
        code: "patch_failed",
        message: format!("{e:?}"),
    }
}

fn map_viewport_error(e: ViewportError) -> WasmErrorData {
    WasmErrorData {
        domain: "viewport",
        code: "viewport_error",
        message: e.to_string(),
    }
}

fn map_search_error(e: SearchError) -> WasmErrorData {
    WasmErrorData {
        domain: "search",
        code: "search_error",
        message: e.to_string(),
    }
}

// ---------------------------------------------------------------------------
// EditorHandle
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct EditorHandle {
    buffer: EditorBuffer,
    decorations: DecorationSet,
    line_layout_policy: LineLayoutPolicy,
    width_policy: WidthPolicy,
    wrap_map: WrapMap,
    wrap_policy: WrapPolicy,
    history: EditHistory,
    highlighter: Option<SyntaxHighlighter>,
    viewport_metrics: ViewportMetrics,
    search_matches: Vec<SearchMatch>,
    edit_count: u64,
    saved_edit_count: u64,
    /// Last logical line for which the highlighter state is valid.
    /// 0 means the highlighter must start from the beginning.
    highlight_valid_through: u32,
    read_only: bool,
}

#[wasm_bindgen]
impl EditorHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(
        text: &str,
        language: &str,
        line_height: f64,
        char_width: f64,
        tab_width: u32,
    ) -> EditorHandle {
        let buffer = EditorBuffer::new(text.to_string());
        let line_layout_policy = LineLayoutPolicy::horizontal_ltr();
        let width_policy = WidthPolicy::cjk_grid(tab_width);
        let wrap_policy = WrapPolicy::code_with_width_policy(width_policy);
        let wrap_map = WrapMap::new(text.split('\n'), u32::MAX, &wrap_policy);
        let history = EditHistory::new(text);
        let decorations = DecorationSet::new();
        let grammar_set = GrammarSet::default_set();
        let highlighter = SyntaxHighlighter::new(&grammar_set, language);
        let viewport_metrics = ViewportMetrics {
            line_height,
            char_width,
            tab_width,
        };

        EditorHandle {
            buffer,
            decorations,
            line_layout_policy,
            width_policy,
            wrap_map,
            wrap_policy,
            history,
            highlighter,
            viewport_metrics,
            search_matches: Vec::new(),
            edit_count: 0,
            saved_edit_count: 0,
            highlight_valid_through: 0,
            read_only: false,
        }
    }

    // -- State --------------------------------------------------------------

    #[wasm_bindgen(js_name = getText)]
    pub fn get_text(&self) -> String {
        self.buffer.text().to_string()
    }

    #[wasm_bindgen(js_name = getTextByteLength)]
    pub fn get_text_byte_length(&self) -> f64 {
        self.buffer.text().len() as f64
    }

    #[wasm_bindgen(js_name = isDirty)]
    pub fn is_dirty(&self) -> bool {
        self.edit_count != self.saved_edit_count
    }

    /// Mark the current state as clean (e.g. after saving).
    #[wasm_bindgen(js_name = markClean)]
    pub fn mark_clean(&mut self) {
        self.saved_edit_count = self.edit_count;
    }

    #[wasm_bindgen(js_name = detectIndent)]
    pub fn detect_indent(&self, sample_lines: u32) -> JsValue {
        let style = self.buffer.detect_indent(sample_lines as usize);
        detect_indent_to_js_value(style)
    }

    // -- Editing ------------------------------------------------------------

    #[wasm_bindgen(js_name = applyEdit)]
    pub fn apply_edit(
        &mut self,
        start: u32,
        end: u32,
        new_text: &str,
        label: &str,
    ) -> Result<JsValue, JsValue> {
        apply_edit_value(self, start, end, new_text, label)
            .map(|changes| {
                let arr = Array::new();
                for rc in &changes {
                    arr.push(&range_change_to_js_value(rc));
                }
                arr.into()
            })
            .map_err(error_to_js_value)
    }

    #[wasm_bindgen]
    pub fn undo(&mut self) -> Result<JsValue, JsValue> {
        undo_value(self)
            .map(undo_redo_result_to_js)
            .map_err(error_to_js_value)
    }

    #[wasm_bindgen]
    pub fn redo(&mut self) -> Result<JsValue, JsValue> {
        redo_value(self)
            .map(undo_redo_result_to_js)
            .map_err(error_to_js_value)
    }

    // -- Display ------------------------------------------------------------

    #[wasm_bindgen(js_name = getVisibleLines)]
    pub fn get_visible_lines(&mut self, scroll_top: f64, height: f64) -> JsValue {
        get_visible_lines_value(self, scroll_top, height)
    }

    #[wasm_bindgen(js_name = getCaretRect)]
    pub fn get_caret_rect(&self, offset: u32) -> Result<JsValue, JsValue> {
        get_caret_rect_value(self, offset)
            .map(|rect| rect_to_js_value(&rect))
            .map_err(error_to_js_value)
    }

    #[wasm_bindgen(js_name = getSelectionRects)]
    pub fn get_selection_rects(&self, anchor: u32, head: u32) -> Result<JsValue, JsValue> {
        get_selection_rects_value(self, anchor, head)
            .map(|rects| {
                let arr = Array::new();
                for r in &rects {
                    arr.push(&rect_to_js_value(r));
                }
                arr.into()
            })
            .map_err(error_to_js_value)
    }

    #[wasm_bindgen(js_name = getVisualLineFrame)]
    pub fn get_visual_line_frame(&self, visual_line: u32) -> Result<JsValue, JsValue> {
        get_visual_line_frame_value(self, visual_line).map_err(error_to_js_value)
    }

    #[wasm_bindgen(js_name = byteOffsetToUtf16)]
    pub fn byte_offset_to_utf16(&self, offset: u32) -> Result<f64, JsValue> {
        byte_offset_to_utf16_value(self, offset)
            .map(|value| value as f64)
            .map_err(error_to_js_value)
    }

    #[wasm_bindgen(js_name = utf16OffsetToByte)]
    pub fn utf16_offset_to_byte(&self, offset: u32) -> Result<f64, JsValue> {
        utf16_offset_to_byte_value(self, offset)
            .map(|value| value as f64)
            .map_err(error_to_js_value)
    }

    #[wasm_bindgen(js_name = hitTest)]
    pub fn hit_test(&self, x: f64, y: f64, scroll_top: f64) -> f64 {
        hit_test_value(self, x, y, scroll_top) as f64
    }

    #[wasm_bindgen(js_name = tokenizeLine)]
    pub fn tokenize_line(&mut self, line: &str) -> JsValue {
        let spans = tokenize_line_value(self, line);
        let arr = Array::new();
        for span in &spans {
            arr.push(&token_span_to_js_value(span));
        }
        arr.into()
    }

    // -- Search -------------------------------------------------------------

    #[wasm_bindgen]
    pub fn search(
        &mut self,
        pattern: &str,
        is_regex: bool,
        case_sensitive: bool,
        whole_word: bool,
    ) -> Result<JsValue, JsValue> {
        search_value(self, pattern, is_regex, case_sensitive, whole_word)
            .map_err(error_to_js_value)?;
        Ok(self.get_search_matches())
    }

    #[wasm_bindgen(js_name = getSearchMatches)]
    pub fn get_search_matches(&self) -> JsValue {
        let arr = Array::new();
        for m in &self.search_matches {
            arr.push(&search_match_to_js_value(m));
        }
        arr.into()
    }

    // -- Viewport -----------------------------------------------------------

    #[wasm_bindgen(js_name = updateMetrics)]
    pub fn update_metrics(&mut self, line_height: f64, char_width: f64, tab_width: u32) {
        self.viewport_metrics = ViewportMetrics {
            line_height,
            char_width,
            tab_width,
        };
        self.width_policy = WidthPolicy::cjk_grid(tab_width);
        self.wrap_policy = WrapPolicy::code_with_width_policy(self.width_policy);
        self.wrap_map = WrapMap::new(
            self.buffer.text().split('\n'),
            self.wrap_map.max_width(),
            &self.wrap_policy,
        );
    }

    #[wasm_bindgen(js_name = getGutterWidth)]
    pub fn get_gutter_width(&self) -> f64 {
        neco_editor_viewport::gutter_width(
            self.buffer.line_index().line_count(),
            &self.viewport_metrics,
        )
    }

    #[wasm_bindgen(js_name = scrollToReveal)]
    pub fn scroll_to_reveal(
        &self,
        offset: u32,
        scroll_top: f64,
        container_height: f64,
    ) -> Result<JsValue, JsValue> {
        scroll_to_reveal_value(self, offset, scroll_top, container_height)
            .map(|opt| match opt {
                Some(v) => JsValue::from_f64(v),
                None => JsValue::NULL,
            })
            .map_err(error_to_js_value)
    }

    // -- Read-only -------------------------------------------------------------

    #[wasm_bindgen(js_name = setReadOnly)]
    pub fn set_read_only(&mut self, value: bool) {
        self.read_only = value;
    }

    #[wasm_bindgen(js_name = isReadOnly)]
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    // -- Auto-indent -----------------------------------------------------------

    /// Returns the leading whitespace of the line containing `offset`.
    #[wasm_bindgen(js_name = autoIndent)]
    pub fn auto_indent(&self, offset: u32) -> String {
        neco_editor::auto_indent(
            self.buffer.text(),
            self.buffer.line_index(),
            offset as usize,
        )
    }

    // -- Auto close bracket ----------------------------------------------------

    /// Returns the closing bracket/quote char code for an opening one, or null.
    #[wasm_bindgen(js_name = autoCloseBracket)]
    pub fn auto_close_bracket(&self, ch: u32) -> JsValue {
        let c = match char::from_u32(ch) {
            Some(c) => c,
            None => return JsValue::NULL,
        };
        match neco_editor::auto_close_bracket(c) {
            Some(close) => JsValue::from_f64(close as u32 as f64),
            None => JsValue::NULL,
        }
    }

    // -- Paste indent adjustment -----------------------------------------------

    /// Adjusts indentation of pasted text relative to the target line.
    /// Current implementation: returns input unchanged (future extension point).
    #[wasm_bindgen(js_name = adjustPasteIndent)]
    pub fn adjust_paste_indent(&self, text: &str, _offset: u32) -> String {
        text.to_string()
    }

    // -- Bracket matching ------------------------------------------------------

    /// Finds the matching bracket at `offset`. Returns `{open, close}` or null.
    #[wasm_bindgen(js_name = findMatchingBracket)]
    pub fn find_matching_bracket(&self, offset: u32) -> JsValue {
        match neco_editor::find_matching_bracket(self.buffer.text(), offset as usize) {
            Some(pair) => {
                let object = Object::new();
                set_prop(&object, "open", JsValue::from_f64(pair.open() as f64));
                set_prop(&object, "close", JsValue::from_f64(pair.close() as f64));
                object.into()
            }
            None => JsValue::NULL,
        }
    }
}

// ---------------------------------------------------------------------------
// Layer 1: pure Rust functions
// ---------------------------------------------------------------------------

fn detect_indent_to_js_value(style: IndentStyle) -> JsValue {
    let object = Object::new();
    match style {
        IndentStyle::Tabs => {
            set_prop(&object, "style", JsValue::from_str("tabs"));
        }
        IndentStyle::Spaces(width) => {
            set_prop(&object, "style", JsValue::from_str("spaces"));
            set_prop(&object, "width", JsValue::from_f64(f64::from(width)));
        }
    }
    object.into()
}

fn apply_edit_value(
    handle: &mut EditorHandle,
    start: u32,
    end: u32,
    new_text: &str,
    label: &str,
) -> Result<Vec<RangeChange>, WasmErrorData> {
    if handle.read_only {
        return Err(WasmErrorData {
            domain: "editor",
            code: "read_only",
            message: "buffer is read-only".to_string(),
        });
    }

    let patch = if start == end && !new_text.is_empty() {
        TextPatch::insert(start as usize, new_text)
    } else if new_text.is_empty() {
        TextPatch::delete(start as usize, end as usize).map_err(map_textpatch_error)?
    } else {
        TextPatch::replace(start as usize, end as usize, new_text).map_err(map_textpatch_error)?
    };
    let patches = vec![patch];

    // Validate patches against text before applying. This prevents panics in
    // inverse_patches (called by history.push_edit) when offsets are out of bounds.
    neco_editor::neco_textpatch::validate_patches(handle.buffer.text(), &patches)
        .map_err(map_textpatch_error)?;

    // Apply with subsystems (history, decorations, wrap map).
    handle
        .buffer
        .apply_patches_with(
            &patches,
            &mut handle.decorations,
            Some(&mut handle.wrap_map),
            Some(&handle.wrap_policy),
            Some(&mut handle.history),
            Some(label),
        )
        .map_err(map_textpatch_error)?;

    // Reconstruct range changes from patches for the caller.
    let range_changes = patches
        .iter()
        .map(|p| {
            let new_end = p.start() + p.replacement().len();
            RangeChange::new(p.start(), p.end(), new_end)
        })
        .collect();

    handle.edit_count += 1;
    handle.highlight_valid_through = 0;

    Ok(range_changes)
}

/// Result of undo/redo: None if nothing to undo/redo, Some(label) otherwise.
fn undo_value(handle: &mut EditorHandle) -> Result<Option<String>, WasmErrorData> {
    let undo_result = match handle.history.undo() {
        Some(r) => r,
        None => return Ok(None),
    };

    match undo_result.kind {
        neco_editor::neco_history::EntryKind::Reversible => {
            if let Some(inverse) = &undo_result.inverse_patches {
                handle
                    .buffer
                    .apply_patches(inverse)
                    .map_err(map_textpatch_error)?;
                handle.wrap_map = WrapMap::new(
                    handle.buffer.text().split('\n'),
                    handle.wrap_map.max_width(),
                    &handle.wrap_policy,
                );
            }
        }
        neco_editor::neco_history::EntryKind::Snapshot => {
            if let Some(snapshot) = &undo_result.snapshot {
                handle.buffer = EditorBuffer::new(snapshot.clone());
                handle.wrap_map = WrapMap::new(
                    handle.buffer.text().split('\n'),
                    handle.wrap_map.max_width(),
                    &handle.wrap_policy,
                );
            }
        }
    }

    handle.edit_count += 1;
    handle.highlight_valid_through = 0;

    Ok(Some(undo_result.label))
}

/// Result of redo: None if nothing to redo, Some(label) otherwise.
fn redo_value(handle: &mut EditorHandle) -> Result<Option<String>, WasmErrorData> {
    let redo_result = match handle.history.redo() {
        Some(r) => r,
        None => return Ok(None),
    };

    match redo_result.kind {
        neco_editor::neco_history::EntryKind::Reversible => {
            if let Some(forward) = &redo_result.forward_patches {
                handle
                    .buffer
                    .apply_patches(forward)
                    .map_err(map_textpatch_error)?;
                handle.wrap_map = WrapMap::new(
                    handle.buffer.text().split('\n'),
                    handle.wrap_map.max_width(),
                    &handle.wrap_policy,
                );
            }
        }
        neco_editor::neco_history::EntryKind::Snapshot => {
            if let Some(snapshot) = &redo_result.snapshot {
                handle.buffer = EditorBuffer::new(snapshot.clone());
                handle.wrap_map = WrapMap::new(
                    handle.buffer.text().split('\n'),
                    handle.wrap_map.max_width(),
                    &handle.wrap_policy,
                );
            }
        }
    }

    handle.edit_count += 1;
    handle.highlight_valid_through = 0;

    Ok(Some(redo_result.label))
}

fn undo_redo_result_to_js(label: Option<String>) -> JsValue {
    match label {
        Some(l) => {
            let object = Object::new();
            set_prop(&object, "label", JsValue::from_str(&l));
            object.into()
        }
        None => JsValue::NULL,
    }
}

fn compute_layout(handle: &EditorHandle) -> ViewportLayout {
    let gw = neco_editor_viewport::gutter_width(
        handle.buffer.line_index().line_count(),
        &handle.viewport_metrics,
    );
    ViewportLayout {
        gutter_width: gw,
        content_left: 0.0,
    }
}

fn get_visible_lines_value(handle: &mut EditorHandle, scroll_top: f64, height: f64) -> JsValue {
    let (first_vl, last_vl) = neco_editor_viewport::visible_line_range(
        scroll_top,
        height,
        &handle.wrap_map,
        &handle.viewport_metrics,
    );

    // Determine the range of logical lines covered by visible visual lines.
    let (first_log, _) = handle.wrap_map.from_visual_line(first_vl);
    let (last_log, _) = handle.wrap_map.from_visual_line(last_vl);

    // Highlighter cache: if the visible range starts after our cached line,
    // the highlighter state is still valid and we can skip re-tokenizing
    // lines 0..highlight_valid_through. Otherwise, reset.
    let tokenize_from =
        if handle.highlight_valid_through > 0 && first_log >= handle.highlight_valid_through {
            handle.highlight_valid_through
        } else {
            if let Some(ref mut hl) = handle.highlighter {
                hl.reset();
            }
            0
        };

    let text = handle.buffer.text();
    let text_len = text.len();
    let line_count = handle.buffer.line_index().line_count();
    let arr = Array::new();

    for log_line in tokenize_from..line_count {
        // Defensive: skip out-of-range lines instead of panicking with expect.
        // This guards against any transient inconsistency between line_count and
        // line_range or text bounds, keeping the WASM module in a usable state.
        let line_range = match handle.buffer.line_index().line_range(log_line) {
            Ok(r) => r,
            Err(_) => break,
        };
        if line_range.start() > text_len || line_range.end() > text_len {
            break;
        }
        let line_text = &text[line_range.start()..line_range.end()];

        let tokens = if let Some(ref mut hl) = handle.highlighter {
            hl.tokenize_line(line_text)
        } else {
            Vec::new()
        };

        // Only emit render lines for visible logical lines.
        if log_line >= first_log && log_line <= last_log {
            let render_line = Object::new();
            set_prop(
                &render_line,
                "lineNumber",
                JsValue::from_f64(f64::from(log_line + 1)),
            );
            set_prop(&render_line, "text", JsValue::from_str(line_text));

            let token_arr = Array::new();
            for span in &tokens {
                token_arr.push(&token_span_to_js_value(span));
            }
            set_prop(&render_line, "tokens", token_arr.into());

            arr.push(&render_line.into());
        }

        // Stop tokenizing after the last visible line.
        if log_line > last_log {
            break;
        }
    }

    // Update cache: highlighter state is valid through the last tokenized line.
    handle.highlight_valid_through = last_log + 1;

    arr.into()
}

fn get_caret_rect_value(handle: &EditorHandle, offset: u32) -> Result<Rect, WasmErrorData> {
    let layout = compute_layout(handle);
    neco_editor_viewport::caret_rect_with_width_policy(
        handle.buffer.text(),
        offset as usize,
        handle.buffer.line_index(),
        &handle.wrap_map,
        &handle.viewport_metrics,
        &layout,
        &handle.width_policy,
    )
    .map_err(map_viewport_error)
}

fn get_selection_rects_value(
    handle: &EditorHandle,
    anchor: u32,
    head: u32,
) -> Result<Vec<Rect>, WasmErrorData> {
    let layout = compute_layout(handle);
    let selection = Selection::new(anchor as usize, head as usize);
    neco_editor_viewport::selection_rects_with_width_policy(
        handle.buffer.text(),
        &selection,
        handle.buffer.line_index(),
        &handle.wrap_map,
        &handle.viewport_metrics,
        &layout,
        &handle.width_policy,
    )
    .map_err(map_viewport_error)
}

fn visual_line_frame_value(
    handle: &EditorHandle,
    visual_line: u32,
) -> Result<neco_editor_viewport::VisualLineFrame, WasmErrorData> {
    let layout = compute_layout(handle);
    neco_editor_viewport::visual_line_frame(
        handle.buffer.text(),
        visual_line,
        handle.buffer.line_index(),
        &handle.wrap_map,
        &handle.viewport_metrics,
        &layout,
        &handle.width_policy,
        &handle.line_layout_policy,
    )
    .map_err(map_viewport_error)
}

fn get_visual_line_frame_value(
    handle: &EditorHandle,
    visual_line: u32,
) -> Result<JsValue, WasmErrorData> {
    visual_line_frame_value(handle, visual_line).map(|frame| visual_line_frame_to_js_value(&frame))
}

fn hit_test_value(handle: &EditorHandle, x: f64, y: f64, scroll_top: f64) -> usize {
    let layout = compute_layout(handle);
    neco_editor_viewport::hit_test_with_width_policy(
        x,
        y,
        scroll_top,
        handle.buffer.text(),
        handle.buffer.line_index(),
        &handle.wrap_map,
        &handle.viewport_metrics,
        &layout,
        &handle.width_policy,
    )
}

fn byte_offset_to_utf16_value(handle: &EditorHandle, offset: u32) -> Result<usize, WasmErrorData> {
    Utf16Mapping::new(handle.buffer.text())
        .byte_to_utf16(offset as usize)
        .map_err(map_viewport_error_from_text)
}

fn utf16_offset_to_byte_value(handle: &EditorHandle, offset: u32) -> Result<usize, WasmErrorData> {
    Utf16Mapping::new(handle.buffer.text())
        .utf16_to_byte(offset as usize)
        .map_err(map_viewport_error_from_text)
}

fn map_viewport_error_from_text(e: neco_editor::neco_textview::TextViewError) -> WasmErrorData {
    WasmErrorData {
        domain: "text",
        code: "utf16_mapping_error",
        message: e.to_string(),
    }
}

fn tokenize_line_value(handle: &mut EditorHandle, line: &str) -> Vec<TokenSpan> {
    match handle.highlighter {
        Some(ref mut hl) => hl.tokenize_line(line),
        None => Vec::new(),
    }
}

fn search_value(
    handle: &mut EditorHandle,
    pattern: &str,
    is_regex: bool,
    case_sensitive: bool,
    whole_word: bool,
) -> Result<(), WasmErrorData> {
    let query = SearchQuery {
        pattern: pattern.to_string(),
        is_regex,
        case_sensitive,
        whole_word,
    };
    let matches =
        neco_editor_search::find_all(handle.buffer.text(), handle.buffer.line_index(), &query)
            .map_err(map_search_error)?;
    handle.search_matches = matches;
    Ok(())
}

fn scroll_to_reveal_value(
    handle: &EditorHandle,
    offset: u32,
    scroll_top: f64,
    container_height: f64,
) -> Result<Option<f64>, WasmErrorData> {
    neco_editor_viewport::scroll_to_reveal(
        handle.buffer.text(),
        offset as usize,
        scroll_top,
        container_height,
        handle.buffer.line_index(),
        &handle.wrap_map,
        &handle.viewport_metrics,
    )
    .map_err(map_viewport_error)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handle(text: &str) -> EditorHandle {
        EditorHandle::new(text, "rust", 20.0, 8.0, 4)
    }

    fn auto_close_bracket_value(_handle: &EditorHandle, ch: u32) -> Option<u32> {
        let c = char::from_u32(ch)?;
        neco_editor::auto_close_bracket(c).map(|close| close as u32)
    }

    fn find_matching_bracket_value(handle: &EditorHandle, offset: u32) -> Option<(usize, usize)> {
        neco_editor::find_matching_bracket(handle.buffer.text(), offset as usize)
            .map(|pair| (pair.open(), pair.close()))
    }

    // -- Basic construction and getText -------------------------------------

    #[test]
    fn new_and_get_text() {
        let h = make_handle("hello world");
        assert_eq!(h.get_text(), "hello world");
    }

    #[test]
    fn is_dirty_initially_false() {
        let h = make_handle("hello");
        assert!(!h.is_dirty());
    }

    // -- applyEdit ----------------------------------------------------------

    #[test]
    fn apply_edit_changes_text() {
        let mut h = make_handle("hello world");
        let result = apply_edit_value(&mut h, 6, 11, "rust", "replace");
        assert!(result.is_ok());
        assert_eq!(h.get_text(), "hello rust");
    }

    #[test]
    fn apply_edit_insert() {
        let mut h = make_handle("hello");
        let result = apply_edit_value(&mut h, 5, 5, "!", "insert");
        assert!(result.is_ok());
        assert_eq!(h.get_text(), "hello!");
    }

    #[test]
    fn apply_edit_delete() {
        let mut h = make_handle("hello world");
        let result = apply_edit_value(&mut h, 5, 11, "", "delete");
        assert!(result.is_ok());
        assert_eq!(h.get_text(), "hello");
    }

    #[test]
    fn apply_edit_marks_dirty() {
        let mut h = make_handle("hello");
        let _ = apply_edit_value(&mut h, 5, 5, "!", "insert");
        assert!(h.is_dirty());
    }

    #[test]
    fn apply_edit_returns_range_changes() {
        let mut h = make_handle("hello world");
        let changes = apply_edit_value(&mut h, 6, 11, "rust", "replace").expect("should succeed");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].start(), 6);
        assert_eq!(changes[0].old_end(), 11);
        assert_eq!(changes[0].new_end(), 10);
    }

    #[test]
    fn apply_edit_out_of_bounds_returns_error() {
        let mut h = make_handle("abc");
        // Use a range (start != end) to trigger validation at TextPatch creation.
        let result = apply_edit_value(&mut h, 10, 12, "x", "oob");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.domain, "editor");
    }

    // -- Undo / Redo --------------------------------------------------------

    #[test]
    fn undo_redo_roundtrip() {
        let mut h = make_handle("hello world");
        apply_edit_value(&mut h, 6, 11, "rust", "replace").expect("edit should succeed");
        assert_eq!(h.get_text(), "hello rust");

        let undo_result = undo_value(&mut h).expect("undo should not error");
        assert!(undo_result.is_some());
        assert_eq!(h.get_text(), "hello world");

        let redo_result = redo_value(&mut h).expect("redo should not error");
        assert!(redo_result.is_some());
        assert_eq!(h.get_text(), "hello rust");
    }

    #[test]
    fn undo_at_root_returns_none() {
        let mut h = make_handle("hello");
        let result = undo_value(&mut h).expect("should not error");
        assert!(result.is_none());
        assert_eq!(h.get_text(), "hello");
    }

    // -- Search -------------------------------------------------------------

    #[test]
    fn search_finds_matches() {
        let mut h = make_handle("hello world hello");
        search_value(&mut h, "hello", false, true, false).expect("should succeed");
        assert_eq!(h.search_matches.len(), 2);
        assert_eq!(h.search_matches[0].range().start(), 0);
        assert_eq!(h.search_matches[1].range().start(), 12);
    }

    #[test]
    fn search_invalid_regex_returns_error() {
        let mut h = make_handle("hello");
        let result = search_value(&mut h, "[invalid", true, true, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.domain, "search");
    }

    #[test]
    fn search_stores_matches() {
        let mut h = make_handle("abc abc");
        search_value(&mut h, "abc", false, true, false).expect("should succeed");
        assert_eq!(h.search_matches.len(), 2);
    }

    // -- Error mapping ------------------------------------------------------

    #[test]
    fn error_mapping_textpatch() {
        let err = map_textpatch_error(TextPatchError::OffsetOutOfBounds { offset: 10, len: 5 });
        assert_eq!(err.domain, "editor");
        assert_eq!(err.code, "patch_failed");
        assert!(err.message.contains("10"));
    }

    #[test]
    fn error_mapping_search() {
        let err = map_search_error(SearchError::InvalidRegex("bad".to_string()));
        assert_eq!(err.domain, "search");
        assert!(err.message.contains("bad"));
    }

    // -- detect_indent ------------------------------------------------------

    #[test]
    fn detect_indent_tabs() {
        let h = make_handle("\tline1\n\tline2\n");
        let style = h.buffer.detect_indent(10);
        assert_eq!(style, IndentStyle::Tabs);
    }

    #[test]
    fn detect_indent_spaces() {
        let h = make_handle("def foo\n  bar\n  baz\n");
        let style = h.buffer.detect_indent(10);
        assert_eq!(style, IndentStyle::Spaces(2));
    }

    // -- Viewport -----------------------------------------------------------

    #[test]
    fn gutter_width_reflects_line_count() {
        let h = make_handle("a\nb\nc\nd\ne\nf\ng\nh\ni\nj");
        let gw = h.get_gutter_width();
        // 10 lines: 2 digits + 1 padding = 3 * 8 = 24
        assert!((gw - 24.0).abs() < f64::EPSILON);
    }

    #[test]
    fn caret_rect_basic() {
        let h = make_handle("hello\nworld");
        let rect = get_caret_rect_value(&h, 0).expect("should succeed");
        assert!(rect.y.abs() < f64::EPSILON);
    }

    #[test]
    fn hit_test_returns_offset() {
        let h = make_handle("hello");
        let layout = compute_layout(&h);
        let offset = hit_test_value(&h, layout.content_left, 0.0, 0.0);
        assert_eq!(offset, 0);
    }

    #[test]
    fn update_metrics_rebuilds_wrap_map_for_tab_width_changes() {
        let mut h = make_handle("a\t b");
        h.wrap_map = WrapMap::new(h.buffer.text().split('\n'), 2, &h.wrap_policy);

        let before = get_caret_rect_value(&h, "a\t".len() as u32).expect("rect before update");
        assert!((before.y - 20.0).abs() < f64::EPSILON);

        h.update_metrics(20.0, 8.0, 1);

        let after = get_caret_rect_value(&h, "a\t".len() as u32).expect("rect after update");
        assert!(after.y.abs() < f64::EPSILON);

        let hit = hit_test_value(&h, after.x, after.y, 0.0);
        assert_eq!(hit, "a\t".len());
    }

    #[test]
    fn visual_line_frame_value_uses_horizontal_layout_mode() {
        let mut h = make_handle("ab cd");
        h.wrap_map = WrapMap::new(h.buffer.text().split('\n'), 3, &h.wrap_policy);
        let frame = visual_line_frame_value(&h, 1).expect("frame");

        assert_eq!(frame.logical_line(), 0);
        assert_eq!(frame.visual_line(), 1);
        assert_eq!(frame.inline_advance(), 2);
        assert_eq!(frame.block_advance(), 1);
        assert_eq!(frame.layout_mode(), LayoutMode::HorizontalLtr);
    }

    #[test]
    fn utf16_mapping_roundtrips_through_wasm_helpers() {
        let h = make_handle("a😀b");

        assert_eq!(byte_offset_to_utf16_value(&h, 0).unwrap(), 0);
        assert_eq!(byte_offset_to_utf16_value(&h, 1).unwrap(), 1);
        assert_eq!(byte_offset_to_utf16_value(&h, 5).unwrap(), 3);
        assert_eq!(utf16_offset_to_byte_value(&h, 0).unwrap(), 0);
        assert_eq!(utf16_offset_to_byte_value(&h, 1).unwrap(), 1);
        assert_eq!(utf16_offset_to_byte_value(&h, 3).unwrap(), 5);
        assert_eq!(h.get_text_byte_length(), 6.0);
    }

    #[test]
    fn scroll_to_reveal_already_visible() {
        let h = make_handle("aaa\nbbb\nccc");
        let result = scroll_to_reveal_value(&h, 0, 0.0, 100.0).expect("should succeed");
        assert!(result.is_none());
    }

    // -- Read-only -------------------------------------------------------------

    #[test]
    fn read_only_blocks_apply_edit() {
        let mut h = make_handle("hello");
        h.set_read_only(true);
        let result = apply_edit_value(&mut h, 5, 5, "!", "insert");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "read_only");
        assert_eq!(h.get_text(), "hello");
    }

    #[test]
    fn read_only_flag_toggles() {
        let mut h = make_handle("hello");
        assert!(!h.is_read_only());
        h.set_read_only(true);
        assert!(h.is_read_only());
        h.set_read_only(false);
        assert!(!h.is_read_only());
    }

    #[test]
    fn read_only_allows_edit_after_disable() {
        let mut h = make_handle("hello");
        h.set_read_only(true);
        h.set_read_only(false);
        let result = apply_edit_value(&mut h, 5, 5, "!", "insert");
        assert!(result.is_ok());
        assert_eq!(h.get_text(), "hello!");
    }

    // -- Auto-indent -----------------------------------------------------------

    #[test]
    fn auto_indent_returns_leading_whitespace() {
        let h = make_handle("    hello\n    world");
        assert_eq!(h.auto_indent(0), "    ");
        assert_eq!(h.auto_indent(10), "    ");
    }

    #[test]
    fn auto_indent_tabs() {
        let h = make_handle("\thello\n\t\tworld");
        assert_eq!(h.auto_indent(0), "\t");
        assert_eq!(h.auto_indent(7), "\t\t");
    }

    #[test]
    fn auto_indent_no_indent() {
        let h = make_handle("hello");
        assert_eq!(h.auto_indent(0), "");
    }

    // -- Auto close bracket ----------------------------------------------------

    #[test]
    fn auto_close_bracket_returns_closing_pair() {
        let h = make_handle("");
        // '(' = 40
        let result = auto_close_bracket_value(&h, '(' as u32);
        assert_eq!(result, Some(')' as u32));
        let result = auto_close_bracket_value(&h, '[' as u32);
        assert_eq!(result, Some(']' as u32));
        let result = auto_close_bracket_value(&h, '{' as u32);
        assert_eq!(result, Some('}' as u32));
        let result = auto_close_bracket_value(&h, '"' as u32);
        assert_eq!(result, Some('"' as u32));
        let result = auto_close_bracket_value(&h, '\'' as u32);
        assert_eq!(result, Some('\'' as u32));
    }

    #[test]
    fn auto_close_bracket_returns_none_for_non_bracket() {
        let h = make_handle("");
        assert_eq!(auto_close_bracket_value(&h, 'a' as u32), None);
        assert_eq!(auto_close_bracket_value(&h, ')' as u32), None);
    }

    // -- Bracket matching (via handle) -----------------------------------------

    #[test]
    fn find_matching_bracket_via_handle() {
        let h = make_handle("(hello)");
        let pair = find_matching_bracket_value(&h, 0);
        assert_eq!(pair, Some((0, 6)));
    }

    #[test]
    fn find_matching_bracket_no_match() {
        let h = make_handle("hello");
        assert_eq!(find_matching_bracket_value(&h, 0), None);
    }

    // -- Adjust paste indent ---------------------------------------------------

    #[test]
    fn adjust_paste_indent_passthrough() {
        let h = make_handle("hello");
        assert_eq!(h.adjust_paste_indent("  pasted", 0), "  pasted");
    }
}
