---
name: Sickle
description: A railway-interlocking instrument system for clear CCL routes.
colors:
  enamel: "#12372a"
  enamel-deep: "#0b241c"
  porcelain: "#f4f0de"
  porcelain-deep: "#ded8bc"
  ink: "#171b19"
  ink-muted: "#4e5b55"
  route-amber: "#f2b544"
  route-amber-deep: "#bd7624"
  signal-red: "#e4573d"
  clear-mint: "#89c8b2"
  steel: "#769087"
typography:
  display:
    fontFamily: '"Barlow Condensed", "Arial Narrow", sans-serif'
    fontSize: "clamp(3.2rem, 6.5vw, 6rem)"
    fontWeight: 700
    lineHeight: 0.84
    letterSpacing: "-0.035em"
  headline:
    fontFamily: '"Barlow Condensed", "Arial Narrow", sans-serif'
    fontSize: "clamp(2rem, 3.5vw, 3.5rem)"
    fontWeight: 600
    lineHeight: 0.95
    letterSpacing: "-0.025em"
  body:
    fontFamily: '"Barlow Condensed", "Arial Narrow", sans-serif'
    fontSize: "clamp(1.1rem, 1.5vw, 1.35rem)"
    fontWeight: 400
    lineHeight: 1.38
  code:
    fontFamily: '"JetBrains Mono Variable", ui-monospace, monospace'
    fontSize: "clamp(0.72rem, 1vw, 0.9rem)"
    fontWeight: 400
    lineHeight: 1.72
    fontFeatureSettings: '"liga" 1, "calt" 1'
  label:
    fontFamily: '"JetBrains Mono Variable", ui-monospace, monospace'
    fontSize: "0.65rem"
    fontWeight: 700
    lineHeight: 1.45
    letterSpacing: "0.05em"
rounded:
  square: "0"
  lamp: "50%"
spacing:
  page-pad: "clamp(1rem, 3.5vw, 4rem)"
  section-block: "clamp(5.5rem, 10vw, 10rem)"
  panel: "clamp(2rem, 4vw, 4rem)"
components:
  copy-button:
    backgroundColor: "{colors.route-amber}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.square}"
    padding: "0.75rem 1.2rem"
    height: "3.25rem"
  route-selected:
    backgroundColor: "{colors.route-amber}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.square}"
    padding: "0.9rem"
    height: "4.25rem"
  porcelain-station:
    backgroundColor: "{colors.porcelain}"
    textColor: "{colors.ink}"
    rounded: "{rounded.square}"
    padding: "1.3rem"
---

# Design System: Sickle

## Overview

**Creative North Star: "Railway Interlocking"**

Sickle presents CCL as traffic moving through a physical route-control instrument. Deep green enamel carries the operating surface; porcelain plates hold inspectable source; amber, red, and mint lamps communicate route, attention, and clear states. Track geometry, engraved labels, and exact command rails make technical relationships visible rather than decorating a generic developer landing page.

The system feels engineered, direct, and trustworthy. Condensed communication lettering carries the public voice while monospaced measurement text operates the controls. Density is organized through rails, boards, and ledgers; selective physical depth makes instrument plates tangible without turning the interface into a stack of cards.

**Key Characteristics:**
- Deep green enamel boards paired with warm porcelain working surfaces.
- Amber route paths, red focus or warning signals, and mint clear lamps.
- Condensed uppercase communication type with monospaced controls and code.
- Square and cut-corner plates connected by track geometry and command rails.
- Responsive re-routing that preserves the command action before detailed code.

## Colors

The palette comes from an electromechanical signal panel: dark enamel and warm porcelain form the field, while three sparse signal colors carry operational meaning.

### Primary
- **Interlocking Enamel:** The main instrument-board field, header, and dark structural sections.
- **Deep Enamel:** Recessed switches, code wells, and command rails.

### Secondary
- **Route Amber:** Active routes, selected controls, install readiness, and primary actions.
- **Deep Route Amber:** Amber adapted for legible emphasis on porcelain.

### Tertiary
- **Signal Red:** Focus outlines, copy-button hover, and removed-value signals.
- **Clear Mint:** Ready lamps, complete states, and the secondary API route.

### Neutral
- **Porcelain:** Page ground and physical source plates.
- **Deep Porcelain:** Subdued inverse copy and plate borders.
- **Instrument Ink:** Primary text and hard borders.
- **Muted Instrument Ink:** Supporting prose and annotations.
- **Panel Steel:** Quiet dividers, inactive tracks, and control borders.

### Named Rules

**The Signal Has Meaning Rule.** Amber selects or routes, red demands attention, and mint confirms clear or complete; never use them as interchangeable decoration.

**The Enamel-and-Porcelain Rule.** Enamel holds systems and controls; porcelain holds reading and evidence. Preserve that material distinction.

## Typography

**Display Font:** Barlow Condensed (with Arial Narrow and sans-serif fallbacks)

**Body Font:** Barlow Condensed (with Arial Narrow and sans-serif fallbacks)

**Label/Mono Font:** JetBrains Mono Variable (with ui-monospace and monospace fallbacks)

**Character:** Barlow Condensed reads like forceful railway communication lettering without becoming nostalgic. JetBrains Mono gives code, commands, measurements, labels, and controls one exact instrument voice.

### Hierarchy
- **Display** (700, fluid 3.2–6rem, 0.84): Uppercase section declarations, tightly tracked and usually held to 9–14 characters per line.
- **Hero Display** (700, fluid 3.6–5rem, 0.84): The first-view statement; one route-defining phrase may shift to deep amber.
- **Headline** (600, fluid 2–3.5rem, 0.95): Local route explanations and API statements.
- **Body** (400, fluid 1.1–1.35rem, 1.38): Direct explanatory copy, muted and normally constrained to 34–38rem.
- **Code** (400, fluid 0.72–0.9rem, 1.68–1.8): Source, output, and document specimens with tabular numerals.
- **Label** (650–800, about 0.62–0.76rem, 0.05–0.08em): Uppercase panel identifiers, statuses, navigation, and controls.

### Named Rules

**The Two-Gauge Rule.** Barlow communicates; JetBrains Mono identifies, measures, operates, and demonstrates.

**The Connected-Code Rule.** Code specimens render through Expressive Code, with JetBrains Mono's standard and contextual ligatures enabled for operators and call chains.

**The Station-Lettering Rule.** Display copy is condensed, uppercase, tightly tracked, and short-lined; do not soften it with ornamental kickers.

## Layout

The system uses a centered 96rem operating canvas with one fluid page gutter. Major sections use a large fluid block interval, while related technical content stays connected inside boards, routes, ledgers, and command rails rather than separating into cards.

Desktop layouts use purposeful unequal columns and visible track relationships. The signature interlocking board routes source, selector, and destination across three columns; route diagrams sit behind the physical plates. At 1000px the board becomes a two-column plate arrangement with the selector above it, larger content pairs collapse, and proof items become a two-by-two board. At 680px all working surfaces stack, command rails become two-row controls, ledgers become vertical records, and navigation remains visible as a compact two-column index.

On mobile, the route selector comes first, the exact install command second, then source and destination evidence. Code and commands scroll horizontally rather than wrapping into misleading syntax. Motion is brief and state-driven, and collapses to near-zero under reduced-motion preferences.

**The Connected-Route Rule.** A technical sequence remains one bordered route or board; do not fragment it into floating feature cards.

**The Command-Before-Evidence Rule.** When space collapses, keep the chosen route and exact command ahead of long code specimens.

## Elevation & Depth

The system is physically layered but flat by default. Enamel recesses, porcelain plates, hard borders, and track beds establish most depth. Broad green-tinted shadows appear only under raised instrument assemblies; small colored glows belong only to illuminated lamps and active route nodes.

### Shadow Vocabulary
- **Raised Interlocking** (`0 22px 44px rgb(11 36 28 / 24%)`): The primary route-control assembly.
- **Raised Routing Plate** (`0 18px 34px rgb(11 36 28 / 20%)`): A substantial porcelain-on-enamel evidence assembly.
- **Porcelain Station** (`0 10px 22px rgb(5 21 16 / 28%)`): Source and destination plates mounted on the board.
- **Recessed Switch** (`0 8px 18px rgb(4 17 13 / 35%)`): The dark route selector housing.

### Named Rules

**The Instrument Depth Rule.** Shadows describe mounted plates, housings, and illuminated hardware; ordinary sections, rows, links, and controls remain unlifted.

## Shapes

The dominant form is square, bordered, and mechanical. Porcelain stations and code wells use one clipped upper-right corner instead of a radius. Two-pixel ink borders define primary plates; one-pixel steel rules divide instrument interiors. Circles are reserved for lamps, route nodes, and line markers because those objects are physically circular in the chosen world.

**The Hardware Exception Rule.** Square is the default; circles are allowed only for signal hardware and route geometry, never as generic pills or rounded containers.

## Components

### Buttons
- **Shape:** Square, compact, and rail-bound, with a minimum height of 3.25rem.
- **Primary:** Route amber with instrument ink, monospaced bold labeling, and compact horizontal padding.
- **Hover / Focus:** Hover changes to signal red with porcelain text. Keyboard focus uses a 3px signal-red outline with 4px offset.
- **Selected Route:** The selected switch fills amber and inverts to ink; inactive switches remain deep enamel and may brighten slightly on hover.

### Cards / Containers
- **Corner Style:** Square by default; inspectable plates may use a single clipped upper-right corner.
- **Background:** Enamel for operating boards, porcelain for evidence, and deep enamel for recessed code or controls.
- **Shadow Strategy:** Only mounted instrument assemblies use the documented physical shadows.
- **Border:** Two-pixel ink or porcelain plate borders with one-pixel steel internal divisions.
- **Internal Padding:** Compact plate padding around 1.3–1.5rem; large route boards use the fluid panel spacing.

### Navigation

Navigation is an always-visible uppercase monospaced index beside the condensed wordmark. Links are borderless, turn amber on hover, and use the global red focus outline. On narrow screens, links become a right-aligned two-column grid instead of hiding behind a menu.

### Signal Lamp

Signal lamps are small circular indicators with a two-pixel rim, inset glass shadow, and a restrained colored glow when lit. Every lamp state is accompanied by a text label or sits within a text-labeled control; color is never the sole carrier of status.

### Interlocking Board

The signature assembly connects a porcelain CCL source plate to one of two output routes. A labeled route switch updates the illuminated path, destination code, and exact Cargo command as one synchronized state. The path animation is short, uses an ease-out curve, and yields to reduced-motion preferences.

### Command Rail

Commands live on deep enamel with porcelain monospaced text and remain on one line. A status cell names readiness, while an adjacent amber copy control completes the rail. On narrow screens the status spans the full width above the horizontally scrollable command.

### Feature Board

Feature choices are ledger rows inside one recessed enamel board. Each row aligns signal, capability, and included route; on small screens it becomes a compact vertical record while retaining the shared border rhythm.

## Do's and Don'ts

### Do:
- **Do** build technical stories from connected routes, mounted plates, ledgers, and exact command rails.
- **Do** keep amber, red, and mint tied to their established signal meanings.
- **Do** use JetBrains Mono for code, commands, measurements, status labels, navigation, and controls.
- **Do** preserve visible focus, text-labeled signal states, horizontal code scrolling, and reduced-motion behavior.
- **Do** use depth selectively to distinguish mounted evidence from recessed operating surfaces.

### Don't:
- **Don't** introduce generic cards, pills, rounded dashboard furniture, or a conventional split hero.
- **Don't** turn the palette into generic neon-on-dark developer styling or use signal colors as ambient decoration.
- **Don't** hide compact mobile navigation behind an invented menu.
- **Don't** place prose in code wells or set commands in display typography.
- **Don't** add glyph-only controls, ornamental kickers, or shadows without a physical instrument role.
