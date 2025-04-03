# UI Rendering and Inference

## Overview

Ultraviolet provides a unified interface system where UI is automatically inferred from data structures by client applications. This approach ensures a clean separation between the core [distributed platform](distributed-architecture.md) and its various client interfaces. Prisms focus solely on their core functionality and data structures, while client applications handle the presentation using intelligent inference to create rich, context-appropriate interfaces.

## Core Philosophy

1. **Data-First Design**: Prisms should focus on producing well-structured data, not presentation details
2. **Implicit Over Explicit**: UI rendering should be inferred from data structure where possible
3. **Progressive Enhancement**: Simple interfaces work everywhere; rich features added where supported
4. **Consistent Experience**: Similar data structures should have consistent presentation across prisms

## Automatic UI Inference

The system automatically infers appropriate UI components based on data structure:

| Data Structure | CLI Rendering | Web Rendering |
|----------------|---------------|--------------|
| Array of Objects | Table with columns from keys | Interactive data table with sorting |
| Single Object | Key-value list or card | Styled card component |
| Simple Value | Direct output | Appropriately styled text element |
| Array of Values | Line-by-line list | Bulleted or numbered list |
| Stream of Tokens | Progressive output | Live-updating text area |

### Example: Automatic Table Inference

When a prism outputs an array of objects with consistent keys:

```json
[
  { "name": "example1", "id": "123", "status": "active" },
  { "name": "example2", "id": "456", "status": "inactive" }
]
```

This is automatically rendered as:

- **CLI**: ASCII table with columns for name, id, and status
- **Web**: HTML table with sortable columns, possibly with status badges

### Example: Card Inference

When a prism outputs a single object:

```json
{
  "name": "example1",
  "id": "123",
  "status": "active",
  "created": "2025-01-01T12:00:00Z"
}
```

This is automatically rendered as:

- **CLI**: Formatted key-value list with color highlighting
- **Web**: Card component with labeled fields and appropriate formatting

## Special Cases

### 1. Streaming Data

For prisms that emit a stream of tokens or partial results:

- **CLI**: Progressive output with appropriate cursor positioning
- **Web**: Live-updating element with scroll behavior

### 2. Interactive Elements

Some outputs may imply interactive elements:

- URLs are rendered as clickable links
- File paths are highlighted and made clickable when they refer to valid paths
- Email addresses are formatted as mailto links in capable interfaces
- Timestamps are formatted according to locale preferences

### 3. Binary Data

For binary responses (images, files, etc.):

- **CLI**: Indicates binary data availability and size
- **Web**: Renders preview where possible, with download options

### 4. Nested Data

For complex nested structures:

- **CLI**: Uses indentation and collapsible sections
- **Web**: Implements expandable sections or nested views

### 5. Status Values

Fields named "status" or containing status-like values (active/inactive, running/stopped):

- **CLI**: Color-coded based on value
- **Web**: Styled badges or indicators

## Type-Based Enhancements

The system detects common data types and provides appropriate formatting:

| Data Type | Detection Method | Enhancement |
|-----------|-----------------|-------------|
| URLs | String pattern matching | Clickable links |
| Dates/Times | ISO8601 pattern or type hint | Formatted to locale |
| File Sizes | Number with B/KB/MB suffix | Human-readable formatting |
| Durations | Time spans | Formatted as human-readable duration |
| IDs | Common ID patterns | Shortened with copy option |
| Boolean | true/false values | Checkmarks/toggles |
| Percentages | Number with % | Progress bars/indicators |

## Implementation Notes

The rendering system follows these steps:

1. Analyze response structure to determine base component type
2. Apply type-specific formatting (dates, numbers, etc.)
3. Render using interface-appropriate components 

Each interface layer (CLI, web, etc.) implements the rendering logic appropriate for its medium while maintaining consistent user experience.

## Extensibility

The inference system is designed to be extensible:

1. New data patterns can be recognized as they become common
2. Additional rendering targets (mobile, desktop GUI) can be added
3. The system learns from user interactions to improve inference

## Benefits

This approach offers several advantages:

1. **Reduced Developer Burden**: Prism authors focus solely on data
2. **Consistent UX**: Users get familiar UI patterns across all prisms
3. **Progressive Disclosure**: Simple data is simply presented; rich data gets rich presentation
4. **Interface Flexibility**: New interface types can be added without changing prisms
5. **Future-Proof**: As UI standards evolve, renderers can be updated centrally
