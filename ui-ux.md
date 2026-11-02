# Shelf: UI/UX Specification

Working name. Applies to v0.1 through v0.3 on Linux. Every value here lives in `app/ui/src/tokens.css` and is consumed directly by the Tauri 2 + Svelte 5 shell.

## 1. Design position

Shelf is a quiet instrument. The user's files are the only loud thing on screen. Chrome recedes; content carries color.

Reference feel: Finder gallery view, Quick Look, Spotlight. Reference discipline: Apple HIG. Not a copy. Linux-native where the platform demands it (CSD, portals, accent color, Trash spec).

Three rules decide every disputed choice:

1. Content over chrome. If a pixel is not a file, a thumbnail, or a control the user is about to use, shrink it.
2. One accent, one radius family, one motion curve. Consistency reads as quality.
3. Every action shows what and why before it happens (matches Safety in the product plan).

## 2. Tokens

### 2.1 Color

Base palette is Apple's published set. Four values carry the whole UI.

| Name | Hex | Role |
|---|---|---|
| Action blue | `#0066CC` | Accent: selection, focus, links, primary action |
| White | `#FFFFFF` | Content surface (light) |
| Fog | `#F5F5F7` | Window and sidebar base (light) |
| Ink | `#1D1D1F` | Primary text (light), window base (dark) |

Derived tokens. Dark values and the semantic colors are our derivations, not Apple-published. Unverified: Apple's exact dark equivalents.

| Token | Light | Dark | Use |
|---|---|---|---|
| `surface.window` | `#F5F5F7` | `#1D1D1F` | Window base |
| `surface.sidebar` | `#F5F5F7` @ 80% | `#1D1D1F` @ 80% | Sidebar, translucent when compositor allows |
| `surface.content` | `#FFFFFF` | `#161617` | Grid and list area |
| `surface.raised` | `#FFFFFF` | `#2A2A2D` | Inspector, popovers, cards |
| `ink.primary` | `#1D1D1F` | `#F5F5F7` | Titles, filenames |
| `ink.secondary` | `#1D1D1F` @ 72% | `#F5F5F7` @ 72% | Metadata |
| `ink.tertiary` | `#1D1D1F` @ 48% | `#F5F5F7` @ 48% | Hints, disabled |
| `line.hairline` | `#1D1D1F` @ 10% | `#F5F5F7` @ 12% | Dividers |
| `accent` | system accent, fallback `#0066CC` | system accent, fallback `#2997FF` | Selection, focus, primary action |
| `caution` | `#B7791F` | `#E0A94A` | Medium-risk cleanup |
| `danger` | `#C2410C` | `#F08A5D` | High-risk, Trash confirmation |

Rules:

- White and Fog are the only light surfaces. Content sits on white, chrome on Fog. Separation comes from that tone step plus hairlines, not shadows.
- Read the accent from the `org.freedesktop.appearance` portal (`accent-color`). Use Action blue only when the portal gives none. Dark fallback is lighter because `#0066CC` on `#1D1D1F` is about 3:1. Verify in CI.
- Color scheme follows the `color-scheme` portal key.
- Apple HIG principles applied: color communicates state, not decoration; never color alone (always text or shape too); test both schemes and increased contrast. Unverified: the HIG page returned no body text when fetched, so these come from prior knowledge of the guideline.
- File-family colours appear only in content (the file silhouette), never in the sidebar. Sidebar icons are neutral: `ink.secondary`, accent when selected. Chips use accent only.
- Text contrast: 4.5:1 body, 3:1 large text and icons. Token contrast test runs in CI for both schemes.
- No gradients as decoration. Only exception: a 24 px scrim under overlaid thumbnail labels.

### 2.2 Type

One family: **Inter** (variable, OFL, bundled so output is identical on every distro). Fallback `system-ui`.

Use optical sizing and tabular figures for sizes, dates, and counts so columns align and numbers do not jitter while scanning.

| Role | Size / line | Weight | Tracking |
|---|---|---|---|
| Window title (Home greeting, shelf title) | 22 / 28 | 650 | -0.02em |
| Section header | 15 / 20 | 600 | -0.01em |
| Body, filename | 13 / 18 | 450 | 0 |
| Metadata | 12 / 16 | 450 | 0 |
| Caption, chip label | 11.5 / 14 | 500 | 0.005em |
| Search field | 15 / 20 | 450 | 0 |
| Quick Search field | 22 / 28 | 400 | -0.015em |

Rules:

- Sentence case everywhere. No all-caps labels. No eyebrows above headings.
- Filenames truncate in the middle (`Screenshot 2026-…-14.32.png`), never at the end, so extensions stay visible.
- Max line length in the inspector: 44 characters.
- Scale with system text-scaling factor. Layout must survive 150%.

### 2.3 Spacing, radii, elevation

- Base unit 4 px. Allowed steps: 4, 8, 12, 16, 24, 32, 48.
- Radii are concentric: inner radius = outer radius minus padding.

| Element | Radius |
|---|---|
| Window | 14 px (CSD, follows compositor) |
| Inspector, popovers, Quick Search panel | 12 px |
| Cleanup card | 12 px |
| Thumbnail | 6 px |
| Chip | full pill |
| Button | 8 px |
| Search field | 10 px |

- Elevation uses two levels only. Popovers and Quick Search: `0 8 24 -4` at 18% ink plus a 1 px `line.hairline` inner stroke. Everything else is flat and separated by hairlines or surface tone.
- Hairlines are 1 device pixel (0.5 logical at 2x), never 1 logical px at 2x.

### 2.4 Motion

One curve family. Springs for things the user moves, ease-out for things the system reveals.

| Token | Value | Use |
|---|---|---|
| `spring.standard` | stiffness 380, damping 34 | Inspector open, selection ring, drag |
| `ease.reveal` | cubic-bezier(0.2, 0, 0, 1), 180 ms | Popovers, chips, sheet |
| `ease.fade` | linear, 90 ms | Thumbnail decode-in |
| `ease.hover` | ease-out, 300 ms | Hover and press states on interactive controls (see section 20) |

Rules:

- No entrance animations on page load. The window appears fully formed (200 ms budget).
- Motion only answers an action: open, expand, confirm, move to Trash.
- One signature moment: **Move to Trash.** Selected items lift 2 px, scale to 96%, then fly to the Trash icon in the sidebar (240 ms). The Trash badge ticks up. The toast offers Undo. Nothing else on screen animates this much.
- Respect `prefers-reduced-motion`: replace all springs with 90 ms fades.
- Never animate during scroll. Scrolling holds 60 fps with zero effects.

### 2.5 Iconography

- 16–18 px grid at 1x. The filled `common-icons/` set is the source for navigation and file-kind glyphs; tint via CSS mask so one asset works in both schemes. Sidebar glyphs are 18 px and neutral.
- Do not mix Adwaita symbolic icons with the app set in the same surface. Extension badges (PDF, ZIP, PY, …) use type colour plus text, never colour alone.
- Icons never appear without a label in the sidebar. In toolbars they carry tooltips after 600 ms.

## 3. Window anatomy

Client-side decorations with a unified toolbar. Native window controls on the platform side (GNOME right, KDE configurable). Follow `gtk-decoration-layout` via the settings portal.

```
┌─────────────┬───────────────────────────────────────────────┬──────────────┐
│ ● ● ●       │  ‹ ›  Downloads        [ Search files…   ⌘K ] │ ⊞  ≡   ⇅   ⓘ │
│             ├───────────────────────────────────────────────┴──────────────┤
│ Recent      │                                                  │ Inspector  │
│ Downloads   │   thumbnail grid (virtualized)                   │            │
│ Screenshots │                                                  │ preview    │
│             │                                                  │ name       │
│ Images      │                                                  │ kind, size │
│ Videos      │                                                  │ from       │
│ Music       │                                                  │ dates      │
│ Documents   │                                                  │            │
│ Archives    │                                                  │ [Show in   │
│ Apps        │                                                  │  Files]    │
│             │                                                  │            │
│ Saved       │                                                  │            │
│  Invoices   │                                                  │            │
│             │                                                  │            │
│ Clean Up    │                                                  │            │
│ Trash       │                                                  │            │
└─────────────┴──────────────────────────────────────────────────┴────────────┘
```

Dimensions:

- Default window 1080 x 700. Minimum 720 x 480.
- Sidebar 216 px, collapsible to 0 with `Ctrl+B`. Below 840 px width it becomes an overlay.
- Inspector 300 px, toggled with `Space` (Quick Look style) or `Ctrl+I`. Below 960 px it overlays the grid.
- Toolbar height 48 px. Content padding 24 px.
- Window state (size, sidebar, inspector, view mode, sort) persists per shelf.

Sidebar grouping is by meaning, separated by 16 px of space, not by dividers or labels:

1. Time and place: Recent, Downloads, Screenshots.
2. Kinds: Images, Videos, Music, Documents, Archives, Apps.
3. Saved searches (v0.2).
4. Clean Up, Trash.

Selected sidebar row: accent at 14% fill, accent glyph, ink.primary label. No bar on the left edge.

## 4. Core surfaces

### 4.1 Home

Opens on Downloads-first content, not a dashboard of widgets.

- Top: the 12 newest files as a single horizontal strip of large thumbnails (160 px). Scrolls horizontally. Title: "Just added".
- Below: one shelf per category, each a one-row preview of 6 to 8 items with the category name and total count. Click the title to open the full grid.
- No stat tiles. No charts. No welcome copy after first run.
- Shelves that are empty are hidden, not shown as blank cards.
- The Clean Up entry appears in the sidebar with a quiet count badge only when reclaimable space exceeds 1 GB. It never nags with a banner.

### 4.2 Grid and list

Grid:

- Thumbnails keep their aspect ratio inside a square cell (fit, not crop). A photo is never cut by the UI.
- Cell sizes: S 96, M 176 (default), L 248, XL 320. A toolbar slider (96–320), pinch, or `Ctrl+scroll` resizes continuously; snap to the nearest 8 px. Cells stretch to fill the row, like the prototype.
- Gap 12 px. Filename below, one line, middle-truncated. Second line (date or size) appears only at L and XL.
- Selection: 2 px accent ring with a 2 px inset gap, filename gets a pill fill. Multi-select by click-drag marquee (from a file or empty space), `Shift`, `Ctrl`, and `Ctrl+A`; clicking empty space clears.
- Placeholder while decoding: a neutral file-type silhouette (see 4.2.1), not a text label floating in a box; transparent images sit on a 16 px checkerboard so alpha is visible. No shimmer, no skeleton animation.
- Decode-in: 90 ms fade from the placeholder. Prefer the freedesktop thumbnail cache; generate only visible and near-visible cells.
- Videos show a 12 px play glyph and duration at the lower trailing corner on a 24 px scrim when duration is known.
- `.txt` and Markdown cards show a page-shaped snippet from the first 512 bytes. Markdown is parsed into a small safe HTML subset (headings, emphasis, code, lists, quotes); raw HTML is dropped. The snippet fades at the bottom and never becomes a reader.
- PDF cards use the first page rendered by `pdftoppm` at the requested thumbnail bucket; the inspector/Quick Look request the 1024 px bucket.
- Home rails scroll with trackpad deltaX, `Shift+wheel`, drag/swipe, and Arrow keys when focused; the cursor is a grab hand and clicks are suppressed after a drag.
- Group headers in time sorts: "Today", "Yesterday", "This week", "Earlier in October". Sticky, 28 px tall.

List:

- Row height 32 px. Columns: Name, Date modified, Size, Kind, From. Users can show or hide columns from the toolbar; order is fixed in v0.1.
- Size column is right-aligned with tabular figures. Date uses relative form under 7 days ("2 hours ago"), absolute after.
- Zebra striping is off. Hover fill `ink.primary` at 4%.

Sorting menu: Name, Date added, Date modified, Size, Kind. Ascending and descending shown with an arrow in the menu, not a second control.

### 4.2.1 File type icon system

Every file without a real thumbnail uses one neutral file-type silhouette. The family decides the shape and muted colour; the extension is a small secondary label. Colour never carries meaning alone; the glyph and label do.

| Variant | Size | Composition |
|---|---|---|
| List row | 20 px | Type glyph only. No extension text. |
| Grid tile | 96–320 px | Folded-corner sheet, central type glyph, small extension label beneath. |
| Inspector / Quick Look | 140–200 px | Folded-corner sheet, type glyph, extension, short family description. |

- The sheet is a silhouette with a folded corner, not a rounded rectangle container.
- Archives use the provided folder silhouette with the extension written beneath it. Video cards use the MP4 file silhouette; code uses the supplied code-simple `<>` glyph inside the sheet. Application cards try the native desktop-entry/icon-theme artwork first, then the provided computer glyph for packages or the rocket for launchable apps.
- Real thumbnails replace the icon completely on success. The icon appears only when there is no thumbnail, the format is unsupported, or generation failed.
- Native application icons are read from `.desktop` entries, icon themes (`~/.local/share/icons`, `~/.icons`, `$XDG_DATA_DIRS/icons`, Flatpak exports) and adjacent AppImage artwork. The lookup is read-only; embedded AppImage internals are only used when an extracted `squashfs-root` sibling exists.
- Families: Document, Spreadsheet, Presentation, Code, Archive, Image, Video, Audio, Font, Database, Application, Unknown. Images use sky blue, audio pink, video red, archives blue and spreadsheets green, tuned for each theme. Other file-type artwork is near-neutral ink (near-white in dark mode). Native application artwork keeps its own color.
- Extension labels: `.tar.gz` is `TAR.GZ`; a leading-dot file such as `.env` is `ENV`; long extensions are shortened to four characters; the full extension stays in the tooltip and accessible description. Unknown extensions keep a generic file glyph.
- The icon is `aria-hidden`; its parent must expose the complete description, for example “Lease-renewal-2026.pdf, PDF document, 2.4 MB, modified 12 Aug 2026”.
- Tooltips use the same language as the accessible description: family plus full extension, never colour names.

File-state matrix:

| State | Presentation |
|---|---|
| Loading | Silhouette at 55% opacity; no spinner. |
| Thumbnail unavailable / decode failure | Full silhouette with a diagonal slash; inspector says “Preview unavailable” and offers Open. |
| Unsupported format | Full silhouette, no slash; inspector says “No preview for this file type” and offers Open. |
| Permission denied | File silhouette plus “Shelf can't read this file. Allow access in system settings.” |
| File removed while displayed | Silhouette plus “File no longer on disk. Reindex to update.” |
| Network/FUSE mount skipped | Not indexed, so it never appears as a missing file; the first-run/settings copy explains the exclusion. |
| Broken symlink | Excluded from the index; never shown. |
| Folder | Not shown in the grid; Shelf is a file index, not a file manager. |
| Hidden file / dotfile | Indexed (`.env`, `.gitignore`) with a type family; dot-directories and OS metadata (`.DS_Store`, `Thumbs.db`) stay excluded. |
| Executable / AppImage | Application glyph; no preview. |
| Empty or zero-byte file | File silhouette, size 0 B, “Empty file” reason in the inspector. |
| Multi-file selection | Stacked preview, count, combined size; type glyph shown only when the selection has one type. |
| Very long / compound extension | Short label (max four characters, compound archives keep one dot), full extension in tooltip/description. |

Screenshot cases for this system: PDF cover, Markdown card, plain-text card, ZIP, Python file, `.env`, `tar.gz`, unknown extension, failed thumbnail, video with duration, audio with cover art, zero-byte file, and multi-select.

### 4.2.2 Filters

A compact filter bar sits directly below the toolbar on every file surface except Home and Clean Up. It uses pills, not a modal.

- **Kind**: All kinds, Images, Videos, Music, Documents, Archives, Apps, Other files. Hidden when the sidebar view already fixes the kind (Images, Documents, Screenshots, etc.).
- **Date**: Any time, Today, Last 7 days, Last 30 days, This year. Applies to file modification time, using local-day boundaries.
- **Type**: appears only when the effective kind is Documents. Groups: All types, PDF, Word documents (`.doc/.docx/.docm/.odt/.rtf/.pages`), Spreadsheets (`.xls/.xlsx/.xlsm/.ods/.csv/.tsv`), Slides (`.ppt/.pptx/.pptm/.odp`), Markdown (`.md/.markdown/.mdx`), Plain text (`.txt/.tex`), E-books (`.epub/.djvu`).
- Active pills use accent-soft with an accent border. A `Clear filters` action appears when anything is active.
- Filters intersect with free-text search: a search for “pdf lease” with Type PDF stays PDF-only; a conflicting type produces no results rather than silently widening the query.
- Sidebar navigation resets filters. Filters persist while typing a search or changing dates within the same view.
- Each pill is a menu button with `aria-haspopup="menu"`; options are radio items; Escape/click-outside closes. The bar wraps instead of overflowing at narrow widths.
- Empty results caused only by filters keep the filter bar visible so users can clear them; the empty state names the active filters rather than saying “no files”.

### 4.3 Search

The search field is the product's front door. `Ctrl+F` or `/` focuses it from anywhere.

- Plain-English input parses live (debounced 80 ms). The field always keeps the raw text editable; interpreted parts appear as chips in an “Interpreted as” bar under the toolbar, so typing never rewrites or moves the caret.
- Clicking a chip removes that filter from the query; drag to reorder is not supported (order never matters).
- Chips: pill, 22 px tall, `accent` at 12% fill, 11.5 px label, leading 12 px glyph naming the filter kind (type, date, size, source, place).
- Parse that fails to understand a word leaves it as plain text and searches filenames and content for it. It never silently drops input.
- Explainability (principle from the plan): hovering a chip shows "Matches files modified between Aug 1 and Aug 31". One line, plain.
- Results update in place. Target under 50 ms. No spinner for local filename search. A 2 px accent progress hairline under the toolbar appears only if a query exceeds 300 ms (OCR and content).
- Result ranking defaults to frecency once a text query starts. A tiny "Opened often" hint is not shown; ranking just works.

Example (what the user sees):

```
[ pdf ] [ in August ] [ over 5 MB ] [ from Gmail ]  invoice
```

Empty results copy: "No files match. Remove a filter to widen the search." with the active chips listed below as removable items. Offer the nearest relaxation (for example, "Drop 'over 5 MB' to see 14 files").

### 4.4 Inspector

Quick Look-grade preview with metadata beneath. Updates instantly on selection change, no transition other than the preview crossfading (90 ms).

Top to bottom:

1. Preview: images and video at full width with the real aspect ratio; PDFs show page 1 with page stepper; text and code show the first 40 lines in the system monospace; audio shows cover art and a scrubber.
2. Name (editable only through the file manager; read-only here) and kind.
3. Facts as a two-column definition list: Size, Dimensions, Created, Modified, Added, Location. Text and code files show the first 40 lines.
4. From (when known): site or app, with the full URL on hover. Hidden when unknown. Never shows "Unknown".
5. Found text (v0.2): matched OCR excerpt with the hit highlighted at accent 20%.
6. Actions: Show in Files (primary), Copy path, Move to Trash. Max three visible; the rest in an overflow menu.

Multi-select shows a stacked preview, count, combined size, and the same action set.

### 4.5 Quick Look

`Space` on a selection opens a floating large preview anchored to the window, up to 80% of the window. Arrow keys step through the current sort. `Space` or `Esc` closes. This is the same inspector preview renderer at larger scale (image, video frame, PDF page 1, or audio cover), so it costs no extra decode paths. If no frame exists, it shows the file-type icon and an Open action.

## 5. Clean Up (v0.2)

Purpose: free space safely. Tone: calm, specific, never alarming.

Layout: card column 640 px max width, left-aligned in the content area. On windows wider than 1100 px, a summary panel sits to the right: total reclaimable, risk split, last scan time. Cards sorted by reclaimable size, largest first.

Card anatomy:

```
┌──────────────────────────────────────────────────────────┐
│  Installers in Downloads                          2.4 GB │
│  14 packages in Downloads. Review before moving anything. │
│                                                           │
│  [thumb][thumb][thumb][thumb]  +10                        │
│                                                           │
│  Low risk · Moves to Trash              [ Review ]        │
└──────────────────────────────────────────────────────────┘
```

- Title states what was found. Size at the trailing edge in tabular figures.
- One-sentence reason ("why"). Never marketing language.
- Evidence strip: up to 4 real thumbnails or file icons plus a count.
- Risk shown as text plus a 8 px dot: Low (ink.tertiary), Medium (caution), High (danger). Never color alone.
- The single button is "Review". Cleanup never happens from the card.

Review page:

- List with checkboxes, all selected by default for Low risk, none selected for Medium and High.
- Sticky footer: "Move 14 files to Trash · 2.4 GB". The button label updates with the live selection.
- A dry-run preview toggle ("Show what will happen") lists paths and the resulting free space.
- After the action: toast "Moved 14 files to Trash. Undo." Undo restores through the Trash spec for 30 seconds in-app, longer through the Trash view.

System-level findings (apt, dnf, pacman caches, Docker, Podman):

- Report only. The card replaces "Review" with "Copy command".
- The command is shown in a monospace block with a one-line explanation and the expected space gain.
- Copy confirmation: the button label changes to "Copied" for 1.5 s. No toast.

Hardlinks and reflinks never count toward the size shown. Cards where counted size differs from apparent size say so: "Counts only space you get back."

## 6. Duplicates (v0.2)

- Groups shown as horizontal sets of the same file at larger size.
- Shelf picks the suggested keeper by rule (oldest in a user-organized folder, not Downloads) and states the rule in one line. The user can change the keeper with one click.
- Exact and look-alike groups sit under separate tabs. Look-alikes show a split-compare view with a draggable divider and a difference hint (resolution, size, crop).
- Nothing pre-selected for look-alikes.

## 7. Quick Search panel (v0.3)

- Centered panel, 680 px wide, 12 px radius, appears 22% from the top of the active display. No window chrome.
- Single field at 22 px. Results list below at 44 px rows with a thumbnail, middle-truncated name, folder path in secondary ink, and a trailing key hint (`Enter` to open, `Ctrl+Enter` to show in Files).
- Up to 8 results, then "See all N in Shelf".
- Opens in under 100 ms from the daemon. Dismiss on focus loss or `Esc`.
- Wayland: portal GlobalShortcuts where available, otherwise `shelf --quick`. The Settings page shows which path is active and the exact command to bind, with a Copy button.

## 8. Tray (v0.3)

- Popover of the last 10 downloads: 40 px thumbnail, name, source and age ("github.com, 2 min ago").
- Rows are draggable into other apps (file drag with correct URI list and text/plain path).
- Footer: "Open Shelf". Nothing else.
- Tray icon is monochrome symbolic. No unread dot; new files are not an alert.

## 9. Copy and voice

- Plain verbs, sentence case, active voice, no filler.
- A command keeps one name through the whole flow. "Move to Trash" is the button, the toast says "Moved to Trash", the menu says "Move to Trash". Never "Delete" or "Remove" for the same action.
- Say what, not how. "Finding text in screenshots" rather than "Running OCR indexer".
- Errors name the problem and the fix, no apology, no mood.
- Empty states are directions.

| Situation | Copy |
|---|---|
| First scan | "Indexing 128,400 files. You can use Shelf now." |
| First scan, large home | "Indexing in the background. Search results will fill in as it goes." |
| Empty Downloads | "Nothing here yet. New downloads appear as they finish." |
| Daemon not running | "Shelf's background service stopped. Restart it" (button: Restart) |
| Permission denied folder | "Shelf can't read ~/Work. Allow access in your system settings." |
| Trash failed | "Couldn't move report.pdf to Trash. The disk is read-only." |
| Network mount skipped | "Skipped /mnt/nas. Network drives are off by default." |
| OCR model missing | "Search inside images needs a 38 MB download." (button: Download) |
| Clean Up, nothing found | "Nothing to clean up. Shelf checks again after each scan." |

## 10. Keyboard and input

Everything is reachable without a pointer.

| Action | Shortcut |
|---|---|
| Focus search | `Ctrl+F`, `/` |
| Quick Search (in app) | `Ctrl+K` |
| Quick Look | `Space` |
| Toggle inspector | `Ctrl+I` |
| Toggle sidebar | `Ctrl+B` |
| Grid / list | `Ctrl+1`, `Ctrl+2` |
| Resize thumbnails | `Ctrl+Plus`, `Ctrl+Minus` |
| Show in file manager | `Ctrl+Enter` |
| Copy path | `Ctrl+Shift+C` |
| Move to Trash | `Delete` |
| Undo | `Ctrl+Z` |
| Back / forward | `Alt+Left`, `Alt+Right` |
| Command list | `Ctrl+Shift+P` |

- Arrow keys move selection in 2D across the grid. `Home` and `End` jump to first and last. Type-to-select works in list view.
- Touchpad: two-finger scroll with momentum, pinch to resize. Respect natural-scroll setting.
- Touch targets 32 px minimum, 44 px on touch displays.

## 11. Accessibility

- Full screen-reader tree through AT-SPI. Grid cells expose name, kind, size, modified date, and selection state.
- Focus ring: 2 px accent, 2 px offset, always visible on keyboard focus, hidden on pointer focus.
- No information by color alone. Risk, selection, and errors always carry text or a shape.
- High-contrast mode: swap hairlines to 1 px solid at 40% ink, remove translucency.
- Reduced motion, reduced transparency, and text scale from the settings portal are honored.
- Language: strings are externalized from day one. Layout tested with 40% longer strings and right-to-left mirroring.

## 12. Platform behavior (Linux)

- GNOME: CSD, header layout from `gtk-decoration-layout`, accent from portal.
- KDE Plasma: CSD with Breeze-compatible controls, blur-behind via KWin protocol where available.
- Sway and XFCE: server-side decoration fallback, no translucency, otherwise identical.
- Fractional scaling: every raster asset is rendered at the actual scale, not upscaled. Thumbnails request the size they display times scale factor.
- Drag and drop: file URIs out, file URIs in (adds a folder to the index only through an explicit confirm).
- Open files with `xdg-open` or the OpenURI portal. Show in Files uses the `org.freedesktop.FileManager1` D-Bus call with fallback to opening the parent folder.

## 13. Performance rules that shape the UI

These are design constraints, not engineering notes.

- No blur, shadow, or alpha-composited layer behind scrolling content. Translucency belongs to the sidebar and Quick Search only, and turns off on software renderers.
- Virtualize everything. Rendered cells = visible plus one screen above and below.
- Thumbnail requests carry a priority by distance from the viewport and cancel when a cell scrolls away.
- Never block the first frame on the daemon. Draw the shell, then fill content. The last known layout is cached and painted at launch.
- Scroll position, selection, and inspector state restore after the daemon reconnects.

## 14. Settings (v0.3)

Single page, no tabs, grouped sections with headers in sentence case:

- Appearance: Light, Dark, Minimal, Match system (default).
- Indexing: folders to include, exclusions, pause on battery saver, current status.
- Search: text recognition on or off, model download status and size.
- Clean Up: automatic rules (off), rule list with risk levels.
- Shortcuts: editable table.
- Privacy: one sentence ("Everything stays on this computer.") plus the list of the only network calls: update check and license check.

Controls apply immediately. No Save button. No dialogs for reversible changes.

## 15. First run

Two screens at most, both skippable.

1. A folder picker preselected with the Home folder, Downloads, Pictures, Documents, and Desktop. Excluded by default: network mounts, `.cache`, `node_modules`. One sentence on privacy. Button: "Start indexing".
2. The app, already populated. Results appear as indexing proceeds.

No tour, no carousel, no account prompt.

## 16. Anti-slop checklist

Review every screen against this before it ships.

- [ ] Any identical-card grid where hierarchy should differ?
- [ ] More than one radius family or more than two shadow levels?
- [ ] A gradient, glow, or glass effect that carries no information?
- [ ] An eyebrow label, all-caps label, or numbered marker on content that is not a sequence?
- [ ] A stat tile or chart with no decision attached?
- [ ] A hover transition on a non-interactive element?
- [ ] An entrance animation on load?
- [ ] An emoji, mascot, or sparkle icon standing in for AI?
- [ ] Copy that sells, jokes, or apologizes?
- [ ] A color used for decoration rather than state or category?
- [ ] Any word "smart", "magic", "AI-powered", or "supercharge" in the UI?
- [ ] A layout that fails at 720 px, 150% text scale, or dark mode?

Any "yes" blocks release of that screen.

## 17. Acceptance criteria

Visual and interaction quality gates, measured with the same harness as the performance budgets.

- Frame time under 16.6 ms at the 99th percentile while scrolling 50,000 items.
- Window visible in under 200 ms; first content from cache in under 250 ms.
- Selection feedback within one frame of input.
- Contrast tests pass in both schemes for all tokens.
- Screenshot regression tests for Home, grid (S, M, L), list, inspector, search with chips, Clean Up card, Review page, Quick Search, and tray, in Light, Dark, 100%, 150%, and 200% scale.
- File-state cases: PDF, ZIP, Python file, `.env`, `tar.gz`, unknown extension, failed thumbnail, video with duration, audio with cover art, zero-byte file, multi-select. Run at list, tile, and inspector sizes.
- Keyboard-only walkthrough completes: search a file, preview it, show it in Files, move another to Trash, undo.
- Screen-reader walkthrough of the same task passes on GNOME with Orca.

## 18. Open design decisions

- Final typeface: Inter is the default. Evaluate one alternative against it on real Linux screenshots at 100% and 150% scale before v0.1 ships.
- Toolkit: Tauri 2 + Svelte 5 is decided. Slint remains the fallback if UI PSS exceeds the reversal trigger.
- Minimal theme definition: proposed as no sidebar tint, no thumbnail radius, list view default.
- Product name, wordmark, and app icon. Icon direction: a single flat shape readable at 16 px, no gradient.

## 19. Composition rules

Apply to the app, the website, and every marketing or onboarding surface.

- Asymmetric by default. Heroes put the primary content on one side (about 60/40 or 7/5 columns) and let whitespace hold the other. Center alignment is reserved for single-purpose moments: Quick Search, empty states, first-run dialog.
- Do not center everything. Text left-aligns. Numbers right-align in tables.
- Clear hierarchy: one dominant element per screen, one supporting tier, everything else quiet. If two things compete, shrink one.
- Generous whitespace. Section padding 64 px minimum on desktop, 40 px on mobile. Whitespace groups; borders are a last resort.
- Vary section structure. Never repeat the same card grid in consecutive sections. Rotate between: a full-bleed image with a text column, a horizontal strip, a two-column list, a single large number with a sentence, a side-by-side comparison.
- In the app this means Home mixes forms on purpose: the "Just added" strip is a large horizontal rail, category shelves are single rows with counts, Clean Up is a vertical card column. No uniform tile grid across Home.

## 20. Micro-interactions

Subtle. They confirm that the system saw the input.

- Interactive controls only. Buttons, rows, chips, thumbnails, links. Static text and containers never react to hover.
- Duration `ease.hover`, 300 ms ease-out. Properties: background, color, shadow, transform.
- Hover: scale up to 1.02 on thumbnails and cards, or one shadow step (never both on the same element). Buttons shift fill by one tone step.
- Press: scale to 0.98, 90 ms in, 300 ms out.
- Focus: ring appears with no delay. Keyboard users never wait on an animation.
- Never animate layout properties (width, height, top, left). Use transform and opacity.
- Never during scroll. Hover states are suppressed while scrolling to protect 60 fps.
- Reduced motion: drop scale and shadow shifts; keep color changes at 90 ms.
- Ceiling: no more than one animated element reacts to a single input, apart from the Move to Trash flight.

## 21. Responsiveness

Mobile first. The same rules govern the website and the app's narrow-window behavior.

- Design the narrowest layout first at 375 px, then add columns. Every section must hold at 375 px with no horizontal scroll, no clipped text, no overlap.
- Breakpoints: 375 (base), 640, 960, 1280, 1680. Layouts change at breakpoints; type scales fluidly between them with clamp.
- Touch targets 44 px minimum on touch breakpoints. Spacing between targets 8 px minimum.
- Asymmetric heroes collapse to a single column: text first, image second, image keeps its aspect ratio.
- Tables and code blocks scroll inside their own container. The page never scrolls sideways.
- Navigation: sidebar becomes a bottom sheet or overlay below 840 px. Inspector becomes a full-height sheet.
- Safe areas respected on phones (notch, home indicator).
- Test matrix: 375, 768, 1080, 1440 px, at 100% and 150% text scale, light and dark.

## 22. Copy

Real, specific, short. Never lorem ipsum, never "Lorem", "Item 1", "Your text here".

- Placeholder content in mockups and screenshots uses believable files and names: `Lease-renewal-2026.pdf`, `IMG_4821.HEIC`, `Screenshot 2026-10-14 at 14.32.07.png`, `invoice-0387-acme.pdf`, `node_modules` at 1.8 GB.
- Sizes, dates, and counts look real: 2.4 GB, 14 files, "2 hours ago".
- Headlines state the benefit in plain terms: "Find the file you downloaded last Tuesday." Not "Supercharge your workflow."
- Body copy: 1 to 2 sentences per block. One idea each.
- Marketing claims carry a number or a mechanism: "Search 500,000 files in under 50 ms", not "Blazing fast".
- Buttons name the result: "Download for Linux", "Review 14 files", "Copy command".

## 23. Imagery

- Real images only. No stock-looking handshake or laptop-on-desk photos, no AI-generated people, no 3D blobs.
- Primary imagery is the product itself: true-to-scale screenshots on a real desktop, light and dark variants, rendered at actual size with correct scaling.
- Every image has a fixed aspect ratio set in layout, and fills it with `object-cover`. Subject-critical crops specify a focal point.
- Standard ratios: hero 4:3 or 3:2, shelf previews 16:10, avatars 1:1, thumbnails in the app are never cropped (fit, see 4.2).
- Alt text describes what matters, not "screenshot": "Shelf showing 12 new downloads, with a PDF invoice selected and its source listed as github.com."
- Decorative images use empty alt text.
- Serve at 1x and 2x, AVIF or WebP with a JPEG fallback. Reserve space to prevent layout shift.
- Screenshots in marketing show real populated states, never empty windows.

## 24. Brand assets

Needed before v0.1 ships. Status: waiting on input.

| Asset | Spec | Status |
|---|---|---|
| App icon | Single flat shape, readable at 16 px, no gradient. SVG master plus 16, 32, 48, 64, 128, 256, 512 PNG | Needed |
| Symbolic icon | Monochrome, 16 px, for tray and sidebar | Needed |
| Wordmark | One-color SVG, light and dark variants | Needed |
| Favicon | 32 px and SVG | Needed |
| Name | Final product name replaces "Shelf" | Open |

Icon rules: no gradients, no glow, no sparkles, no letter-in-a-circle. Shape should read as a file or a shelf without text. One accent color from section 2.1 at most.