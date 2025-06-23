# Composition-Based Architecture

## Overview

This document describes the new composition-based architecture where prisms are pure functional units with no knowledge of other prisms, and their interactions are defined through external composition definitions.

## Core Concepts

### Pure Prisms
- Define only input/output schemas in their spectrum
- No refractions or dependencies on other prisms
- Focus purely on their core functionality

### Compositions
- External definitions that describe how prisms connect
- Define data flow and transformations between prisms
- Can be versioned and shared independently

### Composition Engine
- Dynamically loads required prisms
- Establishes connections based on composition definitions
- Handles data transformation and routing

## Composition Definition Format

```json
{
  "name": "ai-chat-bedrock",
  "version": "1.0.0",
  "description": "AI chat workflow using Bedrock",
  "entry": {
    "prism": "ai:context",
    "frequency": "chat"
  },
  "flow": [
    {
      "id": "context",
      "prism": "ai:context",
      "frequency": "chat_prepare",
      "input_mapping": {
        "prompt": "$.prompt",
        "include_examples": "$.include_examples",
        "prism_filter": "$.prism_filter"
      }
    },
    {
      "id": "llm",
      "prism": "core:bedrock", 
      "frequency": "invoke_stream",
      "input_mapping": {
        "prompt": "$.context.enriched_prompt",
        "model": "$.model",
        "max_tokens": "4096"
      }
    }
  ],
  "connections": [
    {
      "from": "context",
      "to": "llm",
      "mapping": {
        "enriched_prompt": "prompt"
      }
    }
  ],
  "output_mapping": {
    "token": "$.llm.token"
  }
}
```

## Alternative Compositions

```json
{
  "name": "ai-chat-q",
  "version": "1.0.0", 
  "description": "AI chat workflow using Amazon Q",
  "entry": {
    "prism": "ai:context",
    "frequency": "chat"
  },
  "flow": [
    {
      "id": "context",
      "prism": "ai:context", 
      "frequency": "chat_prepare"
    },
    {
      "id": "llm",
      "prism": "core:q",
      "frequency": "invoke_stream"
    }
  ],
  "connections": [
    {
      "from": "context",
      "to": "llm",
      "mapping": {
        "enriched_prompt": "prompt"
      }
    }
  ]
}
```

## Implementation Strategy

1. **Remove Refractions from Prisms**: Strip all refraction definitions from prism spectra
2. **Create Composition Engine**: New component that interprets composition definitions
3. **Update Prism Interface**: Simplify prisms to be pure input/output processors
4. **Composition Registry**: Store and manage composition definitions
5. **CLI Integration**: Allow calling compositions instead of prisms directly

## Benefits

- **Prism Independence**: Prisms have no knowledge of other prisms
- **Flexible Composition**: Same prisms can be combined in different ways
- **Runtime Flexibility**: Switch backends without changing prism code
- **Testability**: Each prism can be tested in isolation
- **Reusability**: Compositions can be shared and versioned