---
name: nsvim
description: The promo site for nsvim, set as the datasheet of a precision part.
colors:
  ink: "#0e1016"
  ink-deep: "#0a0c11"
  ink-raised: "#151925"
  rule: "#232838"
  rule-strong: "#2e3448"
  text: "#d6dae4"
  text-muted: "#a3abbd"
  text-faint: "#8b93a6"
  normal-blue: "#6c9eff"
  normal-blue-hover: "#8db4ff"
  insert-green: "#a7d46f"
  visual-violet: "#c39bff"
  command-orange: "#ffa266"
  error-red: "#f2777a"
  selection: "#2a3150"
typography:
  display:
    fontFamily: "Archivo Variable, Archivo, system-ui, sans-serif"
    fontSize: "clamp(4rem, 9vw, 6rem)"
    fontWeight: 800
    lineHeight: 0.92
    letterSpacing: "-0.02em"
    fontVariation: "\"wdth\" 62"
  headline:
    fontFamily: "Archivo Variable, Archivo, system-ui, sans-serif"
    fontSize: "clamp(1.5rem, 2.4vw, 2rem)"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.01em"
    fontVariation: "\"wdth\" 85"
  title:
    fontFamily: "Archivo Variable, Archivo, system-ui, sans-serif"
    fontSize: "1.0625rem"
    fontWeight: 700
    lineHeight: 1.5
    letterSpacing: "0.07em"
    fontVariation: "\"wdth\" 75"
  subtitle:
    fontFamily: "Archivo Variable, Archivo, system-ui, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 700
    lineHeight: 1.6
    fontVariation: "\"wdth\" 85"
  intro:
    fontFamily: "Archivo Variable, Archivo, system-ui, sans-serif"
    fontSize: "1.25rem"
    fontWeight: 400
    lineHeight: 1.5
  body:
    fontFamily: "Archivo Variable, Archivo, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.6
  label:
    fontFamily: "Archivo Variable, Archivo, system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 600
    letterSpacing: "0.08em"
    fontVariation: "\"wdth\" 75"
  figure:
    fontFamily: "JetBrains Mono Variable, JetBrains Mono, ui-monospace, monospace"
    fontSize: "clamp(1.25rem, 2vw, 1.625rem)"
    fontWeight: 600
    lineHeight: 1.2
    fontFeature: "\"tnum\" 1"
  mono:
    fontFamily: "JetBrains Mono Variable, JetBrains Mono, ui-monospace, monospace"
    fontSize: "0.75rem"
    fontWeight: 400
    lineHeight: 1.4
    fontFeature: "\"liga\" 0, \"calt\" 0"
rounded:
  hair: "3px"
  sm: "4px"
  md: "6px"
  lg: "8px"
spacing:
  space-1: "8px"
  space-2: "16px"
  space-3: "24px"
  space-4: "40px"
  space-5: "64px"
  space-6: "112px"
  gutter: "clamp(16px, 4vw, 48px)"
  col-gap: "clamp(16px, 2.2vw, 32px)"
components:
  button-primary:
    backgroundColor: "{colors.normal-blue}"
    textColor: "{colors.ink-deep}"
    rounded: "{rounded.md}"
    padding: "14px 22px 13px"
  button-primary-hover:
    backgroundColor: "{colors.normal-blue-hover}"
    textColor: "{colors.ink-deep}"
  mode-toggle:
    backgroundColor: "transparent"
    textColor: "{colors.text-faint}"
    typography: "{typography.mono}"
    height: "48px"
    padding: "8px 10px"
  mode-toggle-hover:
    backgroundColor: "{colors.ink-raised}"
    textColor: "{colors.text}"
  mode-toggle-active:
    backgroundColor: "{colors.normal-blue}"
    textColor: "{colors.ink-deep}"
  key-cap:
    backgroundColor: "{colors.ink-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "0.05em 0.45em"
  unit-chip:
    backgroundColor: "{colors.ink-deep}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "8px 12px"
  titled-panel:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.text-faint}"
    rounded: "{rounded.md}"
    padding: "26px 16px 16px"
  plate-frame:
    backgroundColor: "{colors.ink-deep}"
    rounded: "{rounded.lg}"
    padding: "14px 14px 0"
  data-table-head:
    backgroundColor: "{colors.ink-raised}"
    textColor: "{colors.text-muted}"
    typography: "{typography.label}"
    padding: "11px 16px"
  terminal:
    backgroundColor: "{colors.ink-deep}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "16px 18px"
---

# Design System: nsvim

## Overview

**Creative North Star: "The Datasheet"**

The site sells nsvim the way a maker sells a precision part. Each page is a datasheet. It shows real screen plates, measured characteristics in tables, a functional block diagram and ordering information. The world uses the app's own dark palette and the app's own panel form. The page looks like the product because the materials come from the product.

Density is high but calm. Hairline rules divide the content. Titled-border panels hold it. Numbers sit in a monospace face with tabular figures. Colour is functional coding. The four mode colours mean the four editor modes, and nothing else carries chroma.

Motion follows the app. Content changes in the next frame. Only colour fades. The page answers the editor's own keys: `i`, `v`, `:` and `Esc` switch Plate 1, and the mode colour moves through the page.

**Key Characteristics:**
- A dark ink ground with three steps of ink and two steps of rule.
- Four mode colours used as functional coding, never as decoration.
- A condensed grotesk for headings and a monospace face for figures, keys, part numbers and code.
- Titled-border panels and hairline rules instead of cards and shadows.
- Real screens rendered by the app, set as numbered plates with captions.

## Colors

The palette is the app's default dark theme: near-black blue ink, cool grey text, and four saturated mode colours.

### Primary
- **Normal Blue** (#6c9eff): The Normal mode colour. Normal is the resting state of the editor, so Normal Blue is also the colour of links, the focus ring, the caret and the Download button. **Normal Blue Hover** (#8db4ff) is the hover state for links and the Download button.

### Secondary
- **Insert Green** (#a7d46f): The Insert mode colour. It shows on the mode strip, the top mode rule and the Plate 1 title when Insert mode is active.
- **Visual Violet** (#c39bff): The Visual mode colour. It has the same role for Visual mode.
- **Command Orange** (#ffa266): The Command mode colour. It has the same role for Command mode.

### Tertiary
- **Error Red** (#f2777a): The app's error colour. The site defines it as a token for diagnostics and does not use it on the current page.

### Neutral
- **Ink** (#0e1016): The page ground. It is also the knockout behind every title that sits on a border.
- **Ink Deep** (#0a0c11): The ground inside plates, frames, chips and terminal blocks. It is also the text colour on a mode-coloured fill.
- **Ink Raised** (#151925): Table heads, key caps and the hover fill of mode buttons.
- **Rule** (#232838): Internal hairlines. Use it between table rows, between list items and around chips and terminal blocks.
- **Rule Strong** (#2e3448): Outer hairlines. Use it for section rules, plate frames, panel borders and screen-shot borders.
- **Text** (#d6dae4): Headings, figures, table row heads and body copy that leads.
- **Text Muted** (#a3abbd): Supporting copy, captions, table cells and nav links at rest.
- **Text Faint** (#8b93a6): Plate numbers, panel titles, units, notes and terminal comments.
- **Selection** (#2a3150): The text selection fill.

### Named Rules
**The Mode Coding Rule.** A mode colour means its mode. Insert Green, Visual Violet and Command Orange appear only where the page shows that mode. Normal Blue carries the interactive roles because Normal is the default mode.

**The Live Mode Rule.** The `--mode` property holds the current mode colour. The top mode rule and the Plate 1 title read it. A component that shows the current mode reads `--mode` and does not hard-code a mode colour.

## Typography

**Display Font:** Archivo Variable, with Archivo and system-ui as fallbacks
**Body Font:** Archivo Variable, with Archivo and system-ui as fallbacks
**Label/Mono Font:** JetBrains Mono Variable, with JetBrains Mono and ui-monospace as fallbacks

**Character:** One grotesk family carries all prose. The width axis makes the voice. Narrow widths make the datasheet headings, and normal width carries the reading text. The monospace face carries every measured number, key, file name and command.

### Hierarchy
- **Display** (800, clamp(4rem, 9vw, 6rem), 0.92, width 62%): The product name in the first viewport. Use it once per page.
- **Headline** (600, clamp(1.5rem, 2.4vw, 2rem), 1.15, width 85%): The tagline under the product name.
- **Title** (700, 1.0625rem, 0.07em tracking, uppercase, width 75%): Section titles. Each sits on the section's top hairline.
- **Subtitle** (700, 1.125rem or 1rem, width 85%): Headings inside a section, such as a keymap or an install step.
- **Intro** (400, 1.25rem, 1.5): The first paragraph of a section. Keep it to 62ch.
- **Body** (400, 1rem, 1.6): Running copy. Side copy stops at 40ch on wide screens and at 58ch to 62ch elsewhere.
- **Label** (600, 0.75rem, 0.08em tracking, uppercase, width 75%): Table column heads and the key characteristics caption.
- **Figure** (600 mono, clamp(1.25rem, 2vw, 1.625rem), 1.2, tabular): Key characteristic values. The unit follows at 0.6em in Text Muted.
- **Mono** (400 mono, 0.75rem to 0.8125rem): Plate numbers, panel titles, part numbers, file names, header metadata and terminal text.

### Named Rules
**The Measured Figure Rule.** Every measured number uses the monospace face with tabular figures. Numeric table cells align right. The unit sits in its own column or in a smaller muted span.

**The Width Axis Rule.** Make a heading louder with a narrower width and a heavier weight, not with a larger size. Headings use width 62%, 75% or 85%. Prose stays at the normal width.

**The No Ligature Rule.** Code, keys and figures turn ligatures off, so each character shows as typed.

## Layout

The page uses a 12-column grid inside a centred wrap. The wrap is 1480px wide at most, with a fluid side gutter. Columns are separated by a fluid column gap. The first viewport puts the copy in columns 1 to 5 and Plate 1 in columns 6 to 12. The key characteristics row spans all 12 columns under both.

Sections follow one another with a large step of space above each section. A plate takes 8 columns and its side copy takes 4. Pairs of plates take 6 columns each. A full plate takes all 12. Theme plates use a 3-column grid.

Spacing uses a six-step scale: 8, 16, 24, 40, 64 and 112 pixels. Use the 40px step between blocks inside a section. Use the 112px step between sections.

At 1100px and below, the header metadata hides. The first-viewport copy and Plate 1 stack, and side copy goes under its plate. The key characteristics row changes to 3 columns. At 760px and below, the nav keeps only Download and GitHub. Pairs of plates stack, and the key characteristics become one row per figure. Data tables marked to stack show each row as a 2-column block with its labels inline.

### Named Rules
**The Plate And Caption Rule.** Every screen is a numbered plate. The caption sets the plate number in Mono, then a bold short title, then one or two sentences of explanation.

## Elevation & Depth

The system is flat. Depth comes from three steps of ink and from hairline borders. One soft ambient shadow lifts the Plate 1 frame in the first viewport. The sticky header strip uses a translucent ink fill with a background blur, so content scrolls under it.

### Shadow Vocabulary
- **Plate lift** (`box-shadow: 0 30px 80px -40px rgb(0 0 0 / 0.9), 0 2px 6px rgb(0 0 0 / 0.4)`): Only on the Plate 1 frame, the hero plate.
- **Swatch ring** (`box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.12)`): An inner ring on colour swatches, so a dark swatch stays visible on the ink ground.

### Named Rules
**The Hairline Rule.** Separate content with 1px rules in Rule or Rule Strong. Do not use a shadow to separate a panel from the page.

## Shapes

Corners are small and functional. The hair radius of 3px is for swatches and the focus ring. The small radius of 4px is for key caps, chips and the screen inside a frame. The medium radius of 6px is for plates, tables, panels, terminal blocks and the Download button. The large radius of 8px is only for the Plate 1 frame.

The signature form is the titled border, taken from the app's own panels. A 1px border holds the content. The title sits on the top border, on an Ink knockout, offset from the left edge. Section titles use the same form at page scale: a full-width hairline with the title on its left end. Key caps have a 2px bottom border, so they read as physical keys.

## Components

### Buttons
- **Shape:** Medium corners (6px).
- **Primary:** The Download button. Normal Blue fill, Ink Deep text, padding 14px 22px 13px. It has two lines: a 700 label at 1.0625rem and the archive name in Mono at 0.72rem.
- **Hover / Focus:** The fill fades to Normal Blue Hover over the standard fade. Focus shows a 2px Normal Blue outline at a 3px offset.
- **Secondary:** A plain text link next to the primary button. There is no outlined or ghost button.

### Mode Strip
A row of four equal buttons under Plate 1, one for each mode. Each button holds a key cap and the mode name in Mono at 600, 0.04em tracking.
- **Rest:** Transparent fill, Text Faint, a Rule hairline between buttons, 48px tall at least.
- **Hover:** Ink Raised fill, Text colour.
- **Active:** The button's own mode colour as the fill, Ink Deep text. The key cap inside goes translucent dark.
- **Phone:** The key cap stacks over the mode name.

### Key Caps
Mono at 0.82em, Ink Raised fill, a Rule Strong border with a 2px bottom edge, small corners. Use a key cap for every key the reader can press, in prose, in tables and in captions. Separate the keys of a sequence with a 4px gap.

### Cards / Containers
- **Corner Style:** Medium corners (6px). The Plate 1 frame uses large corners (8px).
- **Background:** Ink Deep for plates, chips and terminal blocks. Ink for titled panels.
- **Shadow Strategy:** Flat. Only the Plate 1 frame has the plate lift.
- **Border:** Rule Strong for outer frames and panels. Rule for terminal blocks, chips and table wraps.
- **Internal Padding:** 16px for panels and terminal blocks, 14px for the plate frame.

### Data Tables
The datasheet table. A Rule hairline wraps the table with medium corners. The head row uses an Ink Raised fill and the Label style in Text Muted. Row heads are 600 in Text. Cells are Text Muted. Numeric cells are Mono, right aligned, in Text. Units are Mono in Text Faint. A row hover tints the row with translucent Ink Raised.

### Functional Block Diagram
Two titled panels joined by a bus. Each panel holds unit chips: Mono at 0.8125rem on Ink Deep with a Rule border and small corners. A chain of chips shows a flow from left to right. On a phone, the panels stack and the bus turns horizontal.

### Terminal Block
Mono at 0.8125rem with a line height of 1.75, on Ink Deep, with a Rule border and medium corners. Comments and the prompt are Text Faint. The prompt cannot be selected. On a phone, long lines wrap.

### Navigation
A sticky header strip, 56px tall at least, on translucent Ink with a background blur and a Rule bottom border. It holds the mark, the header metadata as a definition list in Mono at 0.75rem, and the section links. Links rest in Text Muted and go to Text on hover. The GitHub link rests in Text. A 2px mode rule in the current mode colour sits fixed above the strip.

### Signature: Plate 1 With Live Modes
The hero plate is a frame that holds four real screens, one per mode, with the mode strip under them. The frame title shows the plate number, the file name and the current mode in the mode colour. The mode keys and the mode strip swap the screen at once. Only the colours fade. A polite live region announces each change.

## The App

This file is the site's design system. The app has its own rules, and the site's plates show
the app as it is:

- Corners are square. The app's radius is 2px (`rounded-(--radius)`, `nsh/theme.json`).
- Choices are checkboxes, not switches.
- Interface copy is in sentence case, per the NSH Design Guides. Vim's own text (`NORMAL`,
  `E37: ...`) stays as Vim writes it.
- Colours come only from theme tokens.

## Do's and Don'ts

### Do:
- **Do** use the four mode colours only to mean their modes, and read the current mode from `--mode`.
- **Do** show the product with real screens rendered by the app, set as numbered plates with captions.
- **Do** set every measured number in Mono with tabular figures, with the unit separate and muted.
- **Do** hold grouped content in a titled-border panel, with the title on an Ink knockout on the top border.
- **Do** divide content with 1px hairlines in Rule (#232838) or Rule Strong (#2e3448).
- **Do** put every key the reader can press in a key cap.
- **Do** fade only colour, over 160ms with `cubic-bezier(0.16, 1, 0.3, 1)`, and change content in the next frame.
- **Do** make headings with the narrow width axis of Archivo, at 62%, 75% or 85%.

### Don't:
- **Don't** use a mode colour as decoration, as a gradient or as a background wash.
- **Don't** show the product in a laptop mockup or in a drawn screen. Use a real render.
- **Don't** set the product name in any case other than lowercase `nsvim`.
- **Don't** separate panels from the page with shadows. The plate lift is for Plate 1 only.
- **Don't** animate position, size or opacity of content. Content changes in the next frame.
- **Don't** add light-mode colours to the site. The site uses the app's dark theme only.
