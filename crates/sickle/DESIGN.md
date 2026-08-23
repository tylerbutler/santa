---
name: Sickle
description: An annotated compiler-diagnostic system for precise CCL tooling.
colors:
  registration-violet: "#6c3cff"
  registration-violet-dark: "#4a1ee2"
  correction-red: "#d1432f"
  acid-proof: "#d7ff3f"
  paper: "#f3f0e8"
  paper-deep: "#ddd8ca"
  sheet: "#e9e5da"
  ink: "#20231f"
  ink-muted: "#55594f"
  steel: "#2d322e"
  rule: "#aaa99f"
typography:
  display:
    fontFamily: '"Archivo Variable", "Arial Narrow", sans-serif'
    fontSize: "clamp(3rem, 6.3vw, 6rem)"
    fontWeight: 810
    lineHeight: 0.92
    letterSpacing: "-0.04em"
  headline:
    fontFamily: '"Archivo Variable", "Arial Narrow", sans-serif'
    fontSize: "clamp(1.8rem, 3vw, 3.1rem)"
    fontWeight: 700
    lineHeight: 1
    letterSpacing: "-0.035em"
  body:
    fontFamily: '"Archivo Variable", "Arial Narrow", sans-serif'
    fontSize: "1.08rem"
    fontWeight: 400
    lineHeight: 1.5
  code:
    fontFamily: '"Source Code Pro Variable", ui-monospace, monospace'
    fontSize: "clamp(0.76rem, 1vw, 0.98rem)"
    fontWeight: 400
    lineHeight: 1.72
  label:
    fontFamily: '"Source Code Pro Variable", ui-monospace, monospace'
    fontSize: "0.68rem"
    fontWeight: 700
    lineHeight: 1.5
    letterSpacing: "0.08em"
rounded:
  square: "0"
spacing:
  page-gutter: "clamp(1rem, 3.3vw, 3.75rem)"
  section-block: "clamp(5rem, 10vw, 10rem)"
  panel: "clamp(1rem, 2vw, 2rem)"
components:
  copy-button:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.square}"
    padding: "0.65rem 1rem"
    height: "3rem"
  copy-button-inverse:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    typography: "{typography.label}"
    rounded: "{rounded.square}"
    padding: "0.65rem 1rem"
    height: "3rem"
  mode-selected:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    typography: "{typography.label}"
    rounded: "{rounded.square}"
    padding: "0.4rem 0.85rem"
    height: "2.7rem"
  diagnostic-sheet:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.square}"
    padding: "{spacing.panel}"
---

# Design System: Sickle

## Overview

**Creative North Star: "The Compiler Diagnostic Sheet"**

Sickle presents technical material as one continuous, annotated source artifact: warm listing paper, carbon ink, ruled divisions, registration color, and square tool controls. The system is precise rather than nostalgic. Its paper and print references organize real code, feature relationships, and commands instead of decorating generic marketing layouts.

Large compressed headlines establish confidence, while monospaced labels, syntax, and controls carry operational detail. Density is deliberate but legible: strong rules create compartments, generous section spacing creates pauses, and every accent has a specific editorial job.

**Key Characteristics:**
- Warm paper surfaces with carbon rules instead of white cards.
- Oversized, tightly set display type paired with compact monospaced tooling text.
- Violet for registration and active structure, red for annotation, and acid green for readiness or successful change.
- Square, bordered controls and continuous ruled containers.
- Responsive reordering that keeps the install action ahead of the code specimen on small screens.

## Colors

The palette behaves like a marked-up technical proof: warm neutrals carry the page, dark ink structures it, and three sparse signals communicate registration, correction, and readiness.

### Primary
- **Registration Violet:** Marks the central idea, active syntax, feature relationships, focus outlines, and selected structural emphasis.
- **Deep Registration Violet:** Keeps violet legible in smaller syntax and inline emphasis on pale surfaces.

### Secondary
- **Correction Red:** Draws editorial annotations, change marks, and exceptional geometric registration details.
- **Acid Proof:** Signals ready-to-act states, successful additions, selection, and hover feedback where instant recognition matters.

### Neutral
- **Listing Paper:** The page ground and inverse text color.
- **Deep Listing Paper:** Full-width ruled sections and scrollbar tracks.
- **Specimen Sheet:** Repeated code sheets, diffs, and the closing command field.
- **Carbon Ink:** Primary text, rules, and command rails.
- **Muted Carbon:** Supporting copy and secondary labels.
- **Code Steel:** Dark code blocks nested inside the paper system.
- **Registration Rule:** Secondary dividers and dashed change boundaries.

### Named Rules

**The Three-Mark Rule.** Violet registers structure, red corrects or annotates, and acid green confirms readiness or addition; do not interchange their meanings.

**The Paper-First Rule.** Warm paper remains the dominant field. Dark ink creates rails and code wells; accent colors remain annotations rather than large decorative fills.

## Typography

**Display Font:** Archivo Variable (with Arial Narrow and sans-serif fallbacks)  
**Body Font:** Archivo Variable (with Arial Narrow and sans-serif fallbacks)  
**Label/Mono Font:** Source Code Pro Variable (with ui-monospace and monospace fallbacks)

**Character:** Archivo supplies compact, forceful editorial mass without leaving the technical world. Source Code Pro turns commands, labels, navigation, registration data, and code into a consistent instrument layer.

### Hierarchy
- **Display** (810, fluid 3–6rem, 0.92): Section statements; tightly tracked and balanced, usually constrained to 11–13 characters per line.
- **Hero Display** (820, fluid 3.4–6rem, 0.83): The single most compressed headline treatment, with the key phrase registered in violet.
- **Headline** (bold, fluid 1.8–3.1rem, 1): Comparison titles and local editorial statements.
- **Body** (regular to medium, 1.08rem, 1.5): Explanations use muted ink and typically stop near 34–38rem.
- **Code** (regular, fluid 0.76–0.98rem, 1.72): Source specimens use tabular numerals and generous line spacing.
- **Label** (700, 0.68rem, 0.08em): Uppercase pane names, registrations, navigation, and tool controls.

### Named Rules

**The Two-Face Rule.** Archivo speaks and explains; Source Code Pro identifies, operates, and demonstrates.

**The Compressed-Headline Rule.** Display text is heavy, tightly tracked, short-lined, and never diluted with lightweight ornamental copy.

## Layout

The system uses a centered maximum canvas of 96rem with one fluid page gutter. Major sections breathe vertically on a large fluid interval, while source sheets and comparison ledgers are subdivided by one-pixel rules rather than detached cards.

Desktop compositions use purposeful asymmetry: introductory copy sits in unequal columns, the diagnostic sheet divides input and output, and the document story pairs a narrower explanation with a wider specimen. At 900px these structures collapse to one column and proof items become a two-by-two ledger. At 640px navigation becomes a compact two-column index, ledgers and comparisons stack, and feature rows shed their table header.

On small screens the diagnostic remains one artifact but its order changes: registration first, install rail second, then source and output. Annotations disappear when their leader lines cannot retain useful spatial meaning. Code remains horizontally scrollable, command text does not wrap, and tap labels increase in size.

**The Continuous-Sheet Rule.** Related content shares borders and dividers inside one ruled object; do not fragment a sequence into floating feature cards.

**The Action-Before-Detail Rule.** When space collapses, preserve access to the install command before the longer code panes.

## Elevation & Depth

The system is flat by default. Hierarchy comes from tonal paper changes, carbon rails, repeating 32px ruling, and strict borders. Only literal specimen sheets receive a soft, downward ambient shadow (`0 18px 38px rgb(32 35 31 / 13–15%)`) to read as physical paper above the page.

### Shadow Vocabulary
- **Raised Specimen:** A broad carbon-tinted shadow used only beneath diagnostic and diff sheets.

### Named Rules

**The Evidence-Casts-a-Shadow Rule.** Elevation belongs to inspectable source artifacts, never to ordinary sections, rows, navigation, or controls.

## Shapes

Corners are square throughout. Containers, controls, rails, and code wells rely on one-pixel carbon borders, while secondary separations may use the quieter registration rule. The recurring silhouettes are rectangular sheets, narrow command rails, ruled ledgers, and occasional thin red registration geometry. Rounded cards and pill controls do not belong to this system.

## Components

### Buttons
- **Shape:** Square and compact, with a one-pixel current-color border and a minimum 3rem target.
- **Primary:** Copy controls inherit their surrounding command rail, use monospaced bold labels, and keep padding compact.
- **Hover / Focus:** Hover inverts to listing paper and carbon ink. Keyboard focus uses an external violet 3px outline with 4px offset.
- **Mode Control:** Adjacent choices share one border; the active choice fills with carbon ink, while inactive hover uses acid proof.

### Cards / Containers
- **Corner Style:** Strictly square.
- **Background:** Specimen sheets use the sheet neutral with faint horizontal ruling; ordinary content stays directly on listing paper.
- **Shadow Strategy:** Only raised specimens use the documented ambient shadow.
- **Border:** One-pixel carbon outer borders and internal dividers.
- **Internal Padding:** Fluid panel padding keeps code dense on desktop and viable on mobile.

### Navigation

Navigation is a borderless monospaced uppercase index beside the compact wordmark. Links underline only on hover and retain the global violet focus outline. On narrow screens the links become a right-aligned two-column grid instead of a concealed menu.

### Diagnostic Sheet

The signature component combines a registration strip, paired source/output panes, contextual red annotations, and a full-width install rail. Its mode switch changes both the output specimen and exact Cargo command. On mobile the command rail moves directly below registration, before either code pane.

### Feature Ruler

Feature options are rows in a ruled ledger, not cards. Desktop rows align feature, capability, and inclusion information in three columns; mobile rows become compact vertical records while retaining their divider rhythm.

### Command Rail

Commands sit on carbon ink with listing-paper text. A short status cell uses either acid proof or registration violet, and a bordered copy control completes the rail. Command strings remain single-line and horizontally scroll when necessary.

## Do's and Don'ts

### Do:
- **Do** build long technical stories from continuous ruled sheets, rails, and ledgers.
- **Do** reserve violet, red, and acid green for their registered semantic roles.
- **Do** keep code, commands, metadata, and controls in Source Code Pro with tabular numerals.
- **Do** preserve visible keyboard focus, horizontal code scrolling, and the mobile action-first order.
- **Do** use physical depth only when a surface represents inspectable source evidence.

### Don't:
- **Don't** introduce rounded cards, pills, soft dashboard tiles, or a conventional split hero.
- **Don't** use accent colors as interchangeable decoration or broad page backgrounds.
- **Don't** hide mobile navigation behind an invented menu when the compact index fits.
- **Don't** place prose inside dark code wells or code inside display typography.
- **Don't** add shadows to ordinary controls, rows, or sections.
