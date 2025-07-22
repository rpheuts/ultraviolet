# Timer Prism (system:timer)

The Timer prism provides delay functionality for UV workflows, allowing agents and users to introduce controlled pauses in their operations.

## Available Frequencies

### `wait`
Waits for a specified duration before completing the operation.

**Input:**
```json
{
  "duration_ms": 5000,
  "message": "Waiting 5 seconds..."
}
```

**Parameters:**
- `duration_ms` (required): Duration to wait in milliseconds
- `message` (optional): Message to emit while waiting

**Example Usage:**
```bash
uv system:timer wait '{"duration_ms": 3000, "message": "Pausing for 3 seconds..."}'
```

## Use Cases

### For AI Agents
- **Rate limiting**: Wait between API calls to avoid hitting rate limits
- **Delays**: Pause before retrying failed operations
- **Pacing**: Control the speed of automated workflows
- **Timeouts**: Wait for external processes to complete

### Example Agent Actions
```json
{
  "prism": "system:timer",
  "frequency": "wait",
  "input": {
    "duration_ms": 2000,
    "message": "Waiting 2 seconds before retrying..."
  },
  "description": "Adding delay before retry attempt"
}
```

## Behavior

1. Receives a wavefront with duration and optional message
2. Emits optional message photon if provided
3. Waits asynchronously for the specified duration
4. Emits completion trap when timer expires

The prism blocks execution until the timer completes, making it perfect for introducing controlled delays in agent workflows.