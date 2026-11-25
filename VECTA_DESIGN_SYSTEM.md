# VECTA — DESIGN SYSTEM & VISUAL SPECIFICATION

**Product:** Vecta  
**Platform:** Desktop Application  
**Design Direction:** Premium Desktop Productivity  
**Brand Personality:** Bold, Trusted, Clever, Precise  
**Theme:** Dark-first  
**Primary Accent:** Cobalt Blue  
**Design Philosophy:** Precision, with character.

---

# 01. PRODUCT DESIGN PHILOSOPHY

## 1.1 Product Context

Vecta is a career intelligence application designed to help job seekers discover relevant employment opportunities.

Its core workflow consists of:

1. Users describe their skills, experience, and capabilities using natural language.
2. AI processes and structures the information into a career profile.
3. Vecta searches and collects relevant job listings.
4. A matching algorithm evaluates the relevance of each opportunity.
5. Users review matching results and understand why an opportunity may be relevant to their profile.

The interface must support these workflows through clear information hierarchy, thoughtful interactions, and consistent visual patterns.

Vecta should not look like a generic job board, an ordinary SaaS dashboard, or an AI-generated interface template.

It should feel like a carefully designed desktop productivity application.

## 1.2 Brand Personality

The interface must communicate five characteristics.

**Bold**

Confident typography, strong hierarchy, and clear primary actions. Avoid excessive decoration.

**Trusted**

Predictable interactions, readable information, transparent feedback, and restrained use of color.

**Clever**

Intelligent information organization, meaningful context, and useful explanations. Avoid superficial futuristic decoration.

**Precise**

Consistent alignment, spacing, dimensions, typography, and component behavior.

**Premium**

Careful attention to proportions, visual balance, interaction details, and surface treatment.

## 1.3 Core Design Principle

Every visual element must serve a purpose.

Before adding an element, consider:

- Does it improve usability?
- Does it establish hierarchy?
- Does it communicate useful information?
- Does it improve navigation?
- Does it provide meaningful interaction feedback?

If it does none of these, it probably does not belong in the interface.

## 1.4 Design References

Use the following products as references for design quality and interaction principles:

- macOS native applications
- Linear
- Raycast

Learn from their:

- typography discipline;
- spatial consistency;
- restrained visual effects;
- clear component hierarchy;
- predictable interaction patterns;
- careful treatment of desktop layouts.

Do not directly copy their layouts or visual identities.

Vecta must maintain its own design language.

---

# 02. ART DIRECTION

## 2.1 Visual Identity

**Obsidian Navy × Cobalt Blue**

The interface uses dark navy surfaces as its foundation, with Cobalt Blue as the primary accent.

The overall appearance should be:

- dark and sophisticated;
- clean and structured;
- information-oriented;
- restrained but distinctive;
- professional without feeling corporate or sterile.

## 2.2 Surface Philosophy

Use layered surfaces rather than excessive cards.

The application should have a clear distinction between:

1. Main application background.
2. Sidebar and secondary navigation.
3. Elevated content surfaces.
4. Interactive and selected surfaces.
5. Floating overlays and dialogs.

These layers should be distinguishable through subtle tonal differences and borders.

Do not create a different background color for every component.

## 2.3 Avoid

Do not use:

- excessive gradients;
- unnecessary neon colors;
- decorative glow effects;
- excessive glassmorphism;
- excessive backdrop blur;
- generic AI illustrations;
- oversized decorative icons;
- unnecessary dashboard statistics;
- excessive rounded pill components;
- random card arrangements;
- arbitrary spacing;
- inconsistent typography;
- decorative animations without functional purpose.

Do not confuse visual complexity with design quality.

---

# 03. COLOR SYSTEM

All colors must be implemented as reusable design tokens.

Do not repeatedly hardcode color values inside individual components.

## 3.1 Brand Colors

| Token | HEX | Usage |
|---|---|---|
| Brand Primary | #4169E1 | Primary actions and important emphasis |
| Brand Hover | #6F8FFF | Hover states |
| Brand Active | #3154C5 | Pressed states |
| Brand Soft | #243B75 | Subtle selected backgrounds |

Cobalt Blue is the primary brand identity.

Use it selectively. Not every icon, border, heading, or interactive element needs to be blue.

## 3.2 Background Colors

| Token | HEX | Usage |
|---|---|---|
| Background Primary | #111827 | Main application background |
| Background Secondary | #172235 | Sidebar and secondary surfaces |
| Background Elevated | #1A2333 | Cards, dialogs, and elevated content |
| Background Hover | #243249 | Hovered interactive surfaces |

Maintain a clear visual distinction between surface levels.

## 3.3 Border Colors

| Token | HEX | Usage |
|---|---|---|
| Border Default | #2C3A50 | Component boundaries |
| Border Divider | #263449 | Subtle separators |

Borders should be subtle.

Do not outline every nested element.

Use borders when they improve grouping, separation, or affordance.

## 3.4 Typography Colors

| Token | HEX | Usage |
|---|---|---|
| Text Primary | #E8EDF5 | Main content |
| Text Secondary | #98A6BA | Supporting information |
| Text Muted | #68788F | Low-priority metadata |

Do not use muted text for important information or essential instructions.

Verify actual contrast in the rendered application.

## 3.5 Semantic Colors

| Token | HEX | Usage |
|---|---|---|
| Success | #34C38F | Successful operations |
| Warning | #F0B85A | Warnings and incomplete states |
| Error | #F07178 | Errors and destructive actions |
| Info | #55A6E8 | Informational messages |

Semantic colors must communicate meaning.

Do not use them as random decoration.

Important statuses must not rely on color alone. Include text or another visual indicator.

## 3.6 AI Accent

| Token | HEX | Usage |
|---|---|---|
| AI Accent | #9B8CFA | AI-specific interactions |

Violet is reserved for AI-related functionality.

It must not become a second general-purpose brand color.

## 3.7 Color Rules

- Cobalt Blue represents Vecta's primary brand identity.
- Violet represents AI-related interactions.
- Green represents successful states.
- Yellow represents caution.
- Red represents errors or destructive actions.
- Text hierarchy must remain readable.
- Avoid unnecessary accent colors.
- Do not use gradients as a default design treatment.

---

# 04. TYPOGRAPHY SYSTEM

## 4.1 Primary Typeface

**Satoshi**

Satoshi is the primary typeface for Vecta.

Its geometric character provides a balance between confidence, modernity, and approachability.

The typography should feel:

- confident;
- clean;
- intelligent;
- contemporary;
- highly readable.

Use Satoshi consistently throughout the application.

Bundle the font with the application according to its license and distribution requirements.

If Satoshi cannot be reliably bundled, use Geist as the alternative.

Do not randomly mix multiple typefaces.

Recommended CSS:

```css
font-family: "Satoshi", "Geist", sans-serif;
```

## 4.2 Typography Scale

| Role | Size | Weight |
|---|---:|---:|
| Display | 36px | 700 |
| Page Heading | 26px | 700 |
| Section Heading | 18px | 600 |
| Card Heading | 16px | 600 |
| Body | 14px | 400 |
| Secondary | 13px | 400–500 |
| Metadata | 12px | 400–500 |

Display typography should be used sparingly.

Most application pages should begin with a page heading, not an oversized hero section.

## 4.3 Font Weight Rules

- 400: normal body content.
- 500: navigation, labels, and buttons.
- 600: section headings and card titles.
- 700: page headings and important numerical emphasis.

Avoid making every text element bold.

Visual hierarchy depends on the relationship between typography sizes, weights, spacing, and contrast.

## 4.4 Numerical Typography

Use tabular numerals for:

- match scores;
- percentages;
- statistics;
- counts;
- repeated numerical information.

Numerical alignment should remain consistent across lists and tables.

---

# 05. SPACING SYSTEM

A consistent spacing system is mandatory.

Use a 4px-based spacing scale.

## 5.1 Spacing Tokens

| Token | Value |
|---|---:|
| Space 1 | 4px |
| Space 2 | 8px |
| Space 3 | 12px |
| Space 4 | 16px |
| Space 5 | 20px |
| Space 6 | 24px |
| Space 8 | 32px |
| Space 10 | 40px |
| Space 12 | 48px |
| Space 16 | 64px |

Avoid arbitrary spacing values unless there is a specific layout requirement.

## 5.2 Spacing Relationships

| Relationship | Spacing |
|---|---:|
| Icon to label | 8px |
| Label to input | 8px |
| Related controls | 8–12px |
| Elements within a group | 12px |
| Card internal padding | 20–24px |
| Section separation | 32–48px |
| Page horizontal padding | 32px |

## 5.3 Spatial Hierarchy

Spacing must communicate relationships.

Related information should be grouped closely.

Separate content groups should have larger spacing.

Section boundaries should be recognizable without requiring borders everywhere.

Repeated components must maintain consistent vertical rhythm.

Never use random margin values to compensate for a poorly structured layout.

---

# 06. CORNER RADIUS SYSTEM

Vecta uses a restrained geometric shape language.

Radius values are measured in pixels, not degrees.

## 6.1 Radius Tokens

| Token | Value | Usage |
|---|---:|---|
| Radius Small | 6px | Small controls and compact elements |
| Radius Control | 10px | Buttons and inputs |
| Radius Card | 15px | Cards and panels |
| Radius Dialog | 20px | Dialogs and large overlays |
| Radius Full | 9999px | Circular or intentionally pill-shaped elements |

## 6.2 Shape Rules

Buttons and inputs should use 10px radius.

Cards and panels should use 15px radius.

Dialogs may use 20px radius.

Circular controls should use fully rounded geometry.

Do not make every element use 15px radius.

Consistency means components follow the same geometric system, not that every component has identical dimensions.

Avoid combining highly rounded elements with unnecessarily sharp components.

---

# 07. LAYOUT SYSTEM

## 7.1 Desktop Workspace

Vecta should use a structured desktop workspace.

Recommended initial dimensions:

| Element | Dimension |
|---|---:|
| Sidebar | 240px |
| Header | 64px |
| Main Content Max Width | 1200px |
| Page Padding | 32px |
| Sidebar Internal Padding | 16px |

The sidebar should remain stable while the main content adapts to the available window width.

These dimensions are starting points and must be tested against the actual application window.

## 7.2 Alignment Rules

- Page titles must align with the primary content region.
- Card edges should follow consistent content boundaries.
- Repeated controls must share dimensions.
- Related information should follow a consistent grid.
- Avoid arbitrary center alignment.
- Maintain consistent left and right content boundaries.

## 7.3 Information Density

Vecta is an information-oriented application.

Do not sacrifice useful information density for excessive whitespace.

Job listings should allow users to scan important details efficiently.

Prioritize:

- Job title.
- Company.
- Location.
- Work arrangement.
- Match score.
- Relevant skills.
- Missing requirements, when available.
- Listing source.
- Listing freshness, when available.

Do not invent missing information.

## 7.4 Window Adaptation

At smaller desktop window sizes:

- Reflow multi-column layouts.
- Prevent horizontal overflow.
- Preserve readable text sizes.
- Keep important actions accessible.
- Adapt navigation where the existing architecture supports it.
- Avoid shrinking every component indiscriminately.

---

# 08. COMPONENT DESIGN SYSTEM

Every component should use shared tokens and reusable variants.

Do not implement slightly different versions of the same component across different pages.

## 8.1 Button System

Buttons must have a clear hierarchy.

### Primary Button

Used for the most important action.

Specifications:

- Height: 40–44px.
- Horizontal padding: 16px.
- Radius: 10px.
- Font weight: 500–600.
- Background: Brand Primary.
- Hover: Brand Hover.
- Active: Brand Active.
- Icon gap: 8px.

Use one dominant primary action per relevant visual region whenever possible.

### Secondary Button

Used for supporting actions.

Specifications:

- Same dimensions as primary buttons.
- Subtle surface or border.
- Less visual emphasis.
- Consistent interaction states.

### Tertiary Button

Used for low-emphasis actions.

Examples:

- View details.
- Cancel.
- Secondary navigation.
- Inline actions.

### Destructive Button

Used for actions that remove or permanently change information.

Use semantic error styling and explicit action labels.

### Button States

Every button must support:

- Default.
- Hover.
- Pressed.
- Focus-visible.
- Disabled.
- Loading, when applicable.

Disabled controls must remain understandable.

Loading controls must prevent accidental repeated submission where appropriate.

## 8.2 Input System

Standard specifications:

- Height: approximately 44px.
- Radius: 10px.
- Consistent horizontal padding.
- Visible labels.
- Clear focus state.
- Clear validation state.
- Consistent helper text spacing.

Do not depend on placeholder text as the only input label.

For Vecta's natural-language profile input, provide sufficient writing space.

The component should communicate that users can describe their skills and experience in detail.

Do not make it look like a generic search box.

## 8.3 Text Area

Text areas should:

- have a comfortable minimum height;
- support longer natural-language input;
- use clear labels and instructions;
- have visible focus and error states;
- preserve readability;
- provide a clear submission action.

Avoid unnecessary visual decoration.

## 8.4 Select and Dropdown

- Match the general dimensions of other form controls.
- Use consistent radius.
- Provide clear hover and selected states.
- Maintain predictable menu alignment.
- Avoid excessive menu decoration.
- Ensure keyboard accessibility.

## 8.5 Card System

Standard specifications:

- Radius: 15px.
- Padding: 20–24px.
- Background: Elevated Surface.
- Border: subtle when needed.
- Shadow: restrained or absent for ordinary cards.

Cards must group meaningful information.

Do not place every small piece of information inside a separate card.

## 8.6 Badge and Status System

Badges should be compact.

Use semantic colors only when they represent meaningful states.

Ordinary categories and skill labels should use neutral styling.

Avoid turning every keyword into a colorful pill.

## 8.7 Icon Buttons

- Use consistent icon sizes.
- Maintain consistent hit areas.
- Provide tooltips for unclear actions.
- Support keyboard focus.
- Avoid unexplained icon-only destructive actions.

## 8.8 Dialog and Popover

Dialogs must have:

- clear title;
- organized content;
- consistent padding;
- predictable actions;
- appropriate focus behavior;
- clear dismissal behavior.

Use dialog radius of 20px.

Do not create oversized dialogs for simple confirmation tasks.

## 8.9 Navigation

Navigation should prioritize orientation and clarity.

- Keep labels concise.
- Maintain consistent icon dimensions.
- Use a clear selected state.
- Avoid highlighting every navigation item.
- Keep the sidebar visually subordinate to the main workspace.

---

# 09. VECTA-SPECIFIC INFORMATION DESIGN

## 9.1 Job Listing Presentation

A job listing should prioritize decision-making information.

Recommended hierarchy:

1. Job title.
2. Company name.
3. Location and work arrangement.
4. Match score.
5. Relevant skills.
6. Skill gaps, when available.
7. Source and freshness, when available.

The job title should remain the most prominent textual element.

The match score should be noticeable but must not dominate the job title.

## 9.2 Match Score

A percentage alone is not a sufficient explanation of relevance.

Where supported by the matching system, provide:

- matched skills;
- missing requirements;
- relevant experience;
- concise explanation of the match.

Use tabular numerals.

Do not imply that a high match score guarantees recruitment success.

Do not invent reasons for a score when the matching engine does not provide the underlying evidence.

## 9.3 AI-Generated Information

Clearly distinguish between:

- user-provided information;
- AI-structured information;
- system-generated matching results.

Use the AI accent selectively.

AI-generated information should not automatically be treated as verified fact.

Where the product supports editing, users should be able to review and correct structured profile information.

Provide clear processing, success, and failure feedback.

## 9.4 BYOK Settings

API-key configuration is a trust-sensitive workflow.

The interface must:

- clearly identify the provider;
- mask saved credentials;
- never display a complete stored API key;
- communicate connection and validation status;
- provide a clear way to replace or remove credentials;
- avoid exposing secrets in logs or UI messages.

Use secure, platform-appropriate credential storage in the actual application implementation.

Never claim credentials are securely stored unless the implementation genuinely supports that claim.

---

# 10. MOTION AND INTERACTION

Motion should communicate interaction feedback and state transitions.

It should not exist merely to make the application look animated.

## 10.1 Motion Timing

| Interaction | Duration |
|---|---:|
| Button Hover | 150ms |
| Button Press | 100ms |
| Tooltip | 150ms |
| Dropdown | 180ms |
| Dialog | 220ms |
| Page Transition | 200–250ms |

## 10.2 Motion Principles

- Prefer subtle opacity and position transitions.
- Use ease-out for entering elements.
- Use ease-in-out for state transitions.
- Avoid exaggerated bounce.
- Avoid elastic overshoot.
- Avoid unnecessary continuous animations.
- Never delay essential interactions for decoration.
- Respect reduced-motion preferences.

## 10.3 Interaction Feedback

Every meaningful action should communicate its state.

Examples:

- Loading.
- Success.
- Validation error.
- Failure.
- Selected state.
- Disabled state.

Do not rely on animation alone to communicate important information.

---

# 11. ACCESSIBILITY AND USABILITY

Accessibility is part of the design system.

Requirements:

- Maintain readable text contrast.
- Do not communicate status through color alone.
- Provide visible keyboard focus.
- Support keyboard navigation.
- Use meaningful accessible labels.
- Associate form labels with inputs.
- Ensure icon-only controls have accessible names.
- Keep disabled controls understandable.
- Respect reduced-motion settings.
- Maintain usable hit areas.
- Avoid horizontal overflow.

Use WCAG contrast guidance as a reference and test actual rendered color combinations.

A visually attractive palette is not automatically an accessible palette.

---

# 12. DESIGN TOKENS IMPLEMENTATION

Use the following CSS as the initial token source.

Adapt the syntax to the application's actual styling architecture.

```css
:root {
  /* BRAND */
  --brand-primary: #4169E1;
  --brand-hover: #6F8FFF;
  --brand-active: #3154C5;
  --brand-soft: #243B75;

  /* BACKGROUNDS */
  --bg-primary: #111827;
  --bg-secondary: #172235;
  --bg-elevated: #1A2333;
  --bg-hover: #243249;

  /* BORDERS */
  --border-default: #2C3A50;
  --border-divider: #263449;

  /* TEXT */
  --text-primary: #E8EDF5;
  --text-secondary: #98A6BA;
  --text-muted: #68788F;

  /* SEMANTIC */
  --status-success: #34C38F;
  --status-warning: #F0B85A;
  --status-error: #F07178;
  --status-info: #55A6E8;

  /* AI */
  --ai-accent: #9B8CFA;

  /* TYPOGRAPHY */
  --font-primary: "Satoshi", "Geist", sans-serif;

  /* SPACING */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-10: 40px;
  --space-12: 48px;
  --space-16: 64px;

  /* RADIUS */
  --radius-sm: 6px;
  --radius-control: 10px;
  --radius-card: 15px;
  --radius-dialog: 20px;
  --radius-full: 9999px;
}
```

---

# 13. IMPLEMENTATION WORKFLOW

The coding agent must implement the design system systematically.

Do not begin by randomly modifying individual screens.

## Phase 1 — Inspect Existing Application

Before making changes:

1. Inspect the existing application architecture.
2. Identify the current framework and styling approach.
3. Inspect all existing screens and routes.
4. Identify shared and duplicated components.
5. Understand existing user workflows.
6. Identify functionality that must remain unchanged.
7. Check existing desktop window constraints.

Do not replace the existing stack without a concrete technical reason.

## Phase 2 — Establish Design Foundations

1. Implement the color tokens.
2. Implement typography tokens.
3. Implement spacing tokens.
4. Implement radius tokens.
5. Establish global layout rules.
6. Load and verify the primary typeface.
7. Review surface hierarchy.
8. Verify contrast.

## Phase 3 — Build Reusable Components

Establish or consolidate reusable components:

- Button.
- Icon Button.
- Input.
- Text Area.
- Select.
- Checkbox.
- Radio.
- Badge.
- Status Indicator.
- Card.
- Dialog.
- Tooltip.
- Sidebar.
- Page Header.
- Page Container.
- Job Listing Component.
- Match Score Component.

Do not create unnecessary abstractions for trivial wrappers.

Component reuse should improve consistency and maintainability.

## Phase 4 — Apply the Design System

Apply the system consistently across all existing screens.

For every screen:

1. Establish its primary task.
2. Define the visual hierarchy.
3. Organize related information.
4. Apply consistent spacing.
5. Use shared components.
6. Verify interaction states.
7. Verify loading and error states.
8. Check smaller desktop window sizes.
9. Preserve existing functionality.

Do not redesign only the dashboard while leaving other pages inconsistent.

## Phase 5 — Visual Audit

Review the complete application.

Check:

- Typography consistency.
- Spacing consistency.
- Alignment.
- Component dimensions.
- Radius consistency.
- Color usage.
- Surface hierarchy.
- Hover states.
- Focus states.
- Loading feedback.
- Error feedback.
- Accessibility.
- Information density.
- Window adaptation.
- Functional regressions.

Fix systemic inconsistencies through shared tokens and components rather than isolated CSS patches.

---

# 14. QUALITY CONTROL AND ACCEPTANCE CRITERIA

The implementation is ready for review when:

- All major screens use the same design tokens.
- Typography follows the defined hierarchy.
- Spacing follows the defined scale.
- Buttons have consistent dimensions.
- Inputs have consistent dimensions.
- Radius usage follows the shape system.
- Cobalt Blue remains the primary brand accent.
- Violet is reserved for AI-specific interactions.
- Cards group meaningful information.
- Job listings are easy to scan.
- Match scores have appropriate context.
- API credentials are masked.
- Interaction states are implemented.
- Keyboard focus is visible.
- Reduced-motion preferences are respected.
- Existing workflows remain functional.
- No screen feels visually disconnected from the rest of the application.

## Final Design Quality Test

The application should feel like one coherent product.

Users should be able to move between screens without encountering arbitrary changes in spacing, typography, component shape, or interaction behavior.

The interface should be:

- confident without being loud;
- sophisticated without being unnecessarily complex;
- information-dense without becoming chaotic;
- modern without relying on visual trends;
- polished without excessive decoration.

**The objective is not to make every screen look impressive in isolation. The objective is to make the entire application feel intentionally designed.**

---

# 15. FINAL INSTRUCTIONS FOR THE CODING AGENT

Treat this document as the design system specification for Vecta.

Prioritize consistency, usability, maintainability, and implementation quality.

Do not interpret "premium" as permission to add gradients, glassmorphism, oversized typography, excessive animations, or decorative cards.

Do not invent new features.

Do not remove existing functionality.

Do not arbitrarily change the information architecture.

Inspect the existing application before implementation.

Use reusable design tokens and components as the foundation.

When a design decision is not explicitly specified, choose the simplest solution that supports the product's workflow and preserves the established visual language.

The final result must feel deliberate, cohesive, and production-ready.

**Vecta — Precision, with character.**