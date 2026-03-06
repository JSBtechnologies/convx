# Research: Replace LibreOffice & Calibre with Native Packages

## Objective

Replace the two heaviest ConvX dependencies — **LibreOffice** (794MB) and **Calibre** (1.1GB) — with lightweight, native alternatives using Rust crates, Python packages, or small CLI tools. The goal is zero full-application dependencies while maintaining conversion quality.

---

## Current Usage & What Needs Replacing

### LibreOffice (soffice --headless)

Used for 6 conversion paths that nothing else currently handles:

| Conversion | Why It's Hard | Priority |
|---|---|---|
| **DOC → PDF** | .doc is Microsoft's proprietary binary format (OLE2/CFBFS). Very few tools can parse it. | Medium — legacy format, declining usage |
| **DOC → DOCX** | Same .doc parsing problem, plus must output valid OOXML | Medium |
| **PPTX → PDF** | Must parse OOXML presentation format AND render slides (layout, fonts, images, animations) to PDF | High — common user need |
| **PDF → PPTX** | Must extract PDF content and reconstruct as PowerPoint slides | Low — rarely needed |
| **XLSX → PDF** | Must parse spreadsheet AND render cells/formatting/charts to PDF | Medium |
| **PDF → XLSX** | Must extract tabular data from PDF and reconstruct as Excel | Low — rarely needed |

### Calibre (ebook-convert)

Used for 2 conversion paths:

| Conversion | Why It's Hard | Priority |
|---|---|---|
| **EPUB → MOBI** | MOBI is Amazon's proprietary format (now largely replaced by KF8/AZW3) | Low — MOBI is deprecated, Amazon uses EPUB now |
| **MOBI → EPUB** | Must parse MOBI/PRC format and reconstruct as valid EPUB | Low |

Note: **EPUB → PDF** was already migrated to Pandoc + weasyprint.

---

## Research Areas

### 1. DOC (Word 97-2003 Binary) Parsing

The .doc format is the hardest problem. Research these approaches:

**Rust crates:**
- `cfb` crate — Can parse the OLE2 Compound File Binary container that .doc files use
- `doc-rs` or similar — Check if anyone has built a .doc parser in Rust
- Search crates.io for "doc", "ole2", "compound file", "word"

**Python packages:**
- `python-docx` — ONLY handles .docx (OOXML), NOT .doc
- `antiword` — C program (tiny, ~100KB) that extracts text from .doc. Available as a package (`brew install antiword`, `apt install antiword`). Loses formatting but preserves text.
- `textract` — Python package that can extract text from .doc (uses antiword or catdoc internally)
- `olefile` — Python package for parsing OLE2 files (the container format .doc uses)
- `wvWare` / `wv` — C library for parsing .doc files, used by AbiWord. Small package.

**Hybrid approach:**
- Use `antiword` or `wv` to extract text → then Pandoc to convert to PDF/DOCX
- Loses complex formatting but handles 90% of use cases
- antiword is a tiny package (~100KB), acceptable dependency

**Questions to answer:**
- What percentage of .doc files have complex formatting that antiword would lose?
- Is there a Rust OLE2 parser that could extract formatted text with styles?
- Could we use `cfb` crate + custom parsing of the Word binary stream?

### 2. PPTX Rendering (Slides → PDF)

PPTX is OOXML (ZIP of XML files + media). Parsing is straightforward; **rendering** is the hard part.

**Rust crates:**
- `pptx-rs` — Check crates.io for any PPTX parsing crate
- `zip` crate + `quick-xml` — We already have these. Could parse PPTX XML manually.
- `printpdf` — We already use this for image→PDF. Could render parsed slide content to PDF pages.
- `resvg` / `tiny-skia` — SVG/2D rendering in Rust. Could render slide elements to a canvas then export as PDF.

**Python packages:**
- `python-pptx` — Excellent PPTX parser. Can read all slide content (shapes, text, images, tables). CANNOT render to PDF.
- `python-pptx` + rendering pipeline: Parse with python-pptx → render each slide to HTML → convert HTML to PDF via weasyprint
- `comtypes` (Windows only) — Can automate PowerPoint COM object for rendering. Not cross-platform.
- `unoserver` — Lightweight LibreOffice-as-a-service. Still requires LibreOffice but runs it as a persistent daemon rather than spawning a new process each time. Significant performance improvement.

**Hybrid approach (most promising):**
```
python-pptx (parse PPTX)
    → Generate HTML for each slide (with CSS for layout/fonts/colors)
    → weasyprint (HTML → PDF)
    → Merge pages with a PDF library
```

**Questions to answer:**
- How well does python-pptx preserve slide layout/formatting?
- Can weasyprint render slides at exact dimensions (e.g., 10" × 7.5")?
- What slide elements are hardest to render? (SmartArt, charts, animations, embedded video)
- Is there a Rust OOXML parser that handles presentations?

### 3. XLSX Rendering (Spreadsheet → PDF)

Similar to PPTX — parsing is easy, rendering is hard.

**Rust crates:**
- `calamine` — Excellent XLSX/XLS reader in Rust. Can read all cell data, formulas, formatting.
- `rust_xlsxwriter` — Can write XLSX files. We already have the read side covered.
- `printpdf` + `calamine` — Read spreadsheet → render to PDF using printpdf for table layout.

**Python packages:**
- `openpyxl` — We already use this. Full XLSX read/write with formatting.
- `openpyxl` + `weasyprint`: Read spreadsheet → generate HTML table with CSS styling → weasyprint to PDF
- `xlrd` — Reads older .xls format (pre-2007)
- `reportlab` — Python PDF generation library. Could render spreadsheet data as PDF tables.

**Hybrid approach (most promising):**
```
openpyxl (parse XLSX, including formatting/styles)
    → Generate HTML table (with CSS for column widths, cell colors, borders, fonts)
    → weasyprint (HTML → PDF)
```

**Questions to answer:**
- How well does openpyxl preserve cell formatting (merged cells, conditional formatting, charts)?
- Can we handle multi-sheet workbooks (one PDF page per sheet)?
- What about embedded charts? (These are the hardest — may need matplotlib or similar)
- Is calamine + printpdf viable for a pure-Rust path?

### 4. PDF → PPTX / PDF → XLSX (Reverse Conversions)

These are the least common conversions and the hardest to do well.

**PDF → XLSX:**
- `tabula-py` — Python wrapper for Tabula. Extracts tables from PDFs. Excellent for structured data.
- `camelot` — Python package for PDF table extraction. Better than Tabula for complex tables.
- `pdfplumber` — Python package that can extract text and tables from PDFs.
- Approach: Extract tables with tabula/camelot → write to XLSX with openpyxl

**PDF → PPTX:**
- `pdf2image` (poppler) — We already convert PDF→images. Could create PPTX with each page as a full-slide image.
- `python-pptx` — Can create PPTX files. Insert page images as slide backgrounds.
- Approach: PDF → PNG per page (poppler) → python-pptx creates slides with images
- This loses editability but preserves visual fidelity.

### 5. EPUB ↔ MOBI (Ebook Conversions)

**MOBI is effectively dead.** Amazon switched to KF8/EPUB in 2022. Consider:

**Option A: Drop MOBI support entirely**
- Remove EPUB↔MOBI conversion paths
- Most users don't need MOBI anymore
- Eliminates Calibre dependency completely

**Option B: Minimal MOBI support**
- `ebooklib` (Python) — Can read/write EPUB. Does NOT handle MOBI.
- `KindleUnpack` (Python) — Can extract content from MOBI/AZW files
- `kindlegen` — Amazon's official MOBI creator. Discontinued but binaries still available. Tiny (~5MB).
- Approach for EPUB→MOBI: Use kindlegen binary (ship it or download on first use)
- Approach for MOBI→EPUB: Use KindleUnpack to extract, then ebooklib to create EPUB

---

## Architecture Context

### Where converters live
```
convx-core/src/converters/
├── audio.rs      — FFmpeg
├── data.rs       — Python (pandas/openpyxl) + pure Rust (serde)
├── document.rs   — Pandoc, weasyprint, pdf2docx, LibreOffice (fallback), pure Rust (printpdf)
├── ebook.rs      — Calibre (ebook-convert)
├── image.rs      — libvips, FFmpeg (ICO), pure Rust (printpdf for image→PDF)
├── video.rs      — FFmpeg
└── mod.rs        — Converter trait definition
```

### Converter trait
```rust
pub trait Converter: Send + Sync {
    fn can_convert(&self, from: Format, to: Format) -> bool;
    fn convert(&self, input: &Path, output: &Path, options: &ConversionOptions) -> Result<ConversionResult, ConvxError>;
}
```

### How dispatch works
`ConvxEngine` holds a `Vec<Box<dyn Converter>>` and iterates them in order. First converter that returns `can_convert() == true` handles the conversion. Order: Image → Video → Audio → Document → Data → Ebook.

### Adding a new converter or replacing an existing one
1. Add/modify the conversion method in the relevant converter file
2. Update the `convert()` match arms to route to the new method
3. Update `can_convert()` if adding/removing conversion paths
4. Update `convertible_targets()` in `format.rs` to match
5. Run `cargo test` — 47 tests cover format targets, conversion routing, and integration

### Python execution pattern
All Python code runs via the ConvX venv at `~/.convx/venv`:
```rust
let python = DependencyChecker::convx_python()?;  // ~/.convx/venv/bin/python3
let script = format!("from module import ...; ...");
Command::new(python).args(["-c", &script]).output()?;
```

New pip packages are auto-installed by the setup wizard or `DependencyChecker::install_pip_module()`.

### Dependency registration
- `convx-core/src/utils/deps.rs` — Binary/module detection
- `convx-app/src-tauri/src/commands.rs` — `get_missing_dependencies()` + `install_single_dependency()`
- `installers/bootstrap-*.sh` — OS-level install scripts
- `installers/dependency-manifest.json` — Pinned versions

---

## Recommended Implementation Order

### Phase 1: Quick wins (eliminate Calibre)
1. **Drop MOBI or use kindlegen** — MOBI is dead. Either remove the paths or ship the tiny kindlegen binary.
2. EPUB→PDF is already on Pandoc. This eliminates Calibre entirely.

### Phase 2: XLSX → PDF (medium difficulty)
1. Use `openpyxl` → HTML table → `weasyprint` → PDF pipeline
2. Already have openpyxl and weasyprint as dependencies
3. Write a Python script that reads XLSX formatting and generates styled HTML

### Phase 3: PPTX → PDF (hard)
1. Use `python-pptx` → HTML slides → `weasyprint` → PDF pipeline
2. Add `python-pptx` as a pip dependency
3. Start with text/image slides, iterate on complex elements

### Phase 4: DOC → PDF/DOCX (medium)
1. Add `antiword` as a package dependency (tiny, ~100KB)
2. Pipeline: antiword extracts text → Pandoc converts to PDF/DOCX
3. Accept that complex formatting will be simplified

### Phase 5: PDF → PPTX / PDF → XLSX (low priority)
1. PDF→XLSX: tabula-py or camelot for table extraction → openpyxl
2. PDF→PPTX: PDF→images (poppler) → python-pptx creates image slides
3. These are niche — implement last

### Phase 6: Pure Rust (long-term)
1. Replace Python XLSX rendering with calamine + printpdf
2. Build Rust PPTX parser using zip + quick-xml
3. Build Rust .doc parser using cfb crate
4. Eliminate Python dependency entirely (stretch goal)

---

## Success Criteria

- [ ] All 47 existing tests pass
- [ ] No quality regression in conversion output
- [ ] LibreOffice removed from dependency-manifest.json and all bootstrap scripts
- [ ] Calibre removed from dependency-manifest.json and all bootstrap scripts
- [ ] New dependencies are all packages (pip, brew, cargo) — no full applications
- [ ] Setup wizard auto-installs any new dependencies
- [ ] Total new dependency size < 50MB (vs 1.9GB for LibreOffice + Calibre)

---

## Key Files to Modify

| File | What to change |
|---|---|
| `convx-core/src/converters/document.rs` | Replace LibreOffice calls with new pipelines |
| `convx-core/src/converters/ebook.rs` | Replace Calibre calls or remove MOBI paths |
| `convx-core/src/types/format.rs` | Update `convertible_targets()` if removing paths |
| `convx-core/src/utils/deps.rs` | Add new deps, remove LibreOffice/Calibre checks |
| `convx-app/src-tauri/src/commands.rs` | Update `get_missing_dependencies()` + installers |
| `installers/bootstrap-macos.sh` | Update install commands |
| `installers/bootstrap-linux.sh` | Update install commands |
| `installers/bootstrap-windows.ps1` | Update install commands |
| `installers/dependency-manifest.json` | Update manifest |
| `convx-core/Cargo.toml` | Add any new Rust crates |
