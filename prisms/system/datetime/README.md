# DateTime Prism (system:datetime)

The DateTime prism provides current date and time information in multiple formats, giving AI agents temporal awareness for better decision making and naming.

## Available Frequencies

### `now`
Returns current date and time in multiple formats.

**Input:**
```json
{}
```

**Output:**
```json
{
  "timestamp": "2025-01-20T15:30:45.123Z",
  "unix_timestamp": 1737385845,
  "unix_timestamp_ms": 1737385845123,
  "date": "2025-01-20",
  "time": "15:30:45",
  "datetime": "2025-01-20 15:30:45 UTC"
}
```

**Example Usage:**
```bash
uv system:datetime now '{}'
```

## Use Cases

### For AI Agents
- **Temporal awareness**: Understanding current date/time for context
- **File naming**: Creating timestamped files and directories
- **Logging**: Adding timestamps to operations and reports
- **Scheduling**: Understanding current time for scheduling decisions
- **Timer testing**: Getting start time before using timer prism

### Example Agent Actions
```json
{
  "prism": "system:datetime",
  "frequency": "now",
  "input": {},
  "description": "Getting current timestamp for file naming"
}
```

## Output Formats

- **`timestamp`**: ISO 8601 format with milliseconds (UTC)
- **`unix_timestamp`**: Unix timestamp in seconds
- **`unix_timestamp_ms`**: Unix timestamp in milliseconds
- **`date`**: Date only in YYYY-MM-DD format
- **`time`**: Time only in HH:MM:SS format
- **`datetime`**: Human-readable date and time

## Integration with Timer Prism

Perfect companion to the timer prism for testing:

1. Get current time with `system:datetime now`
2. Start timer with `system:timer wait`
3. Get time again to verify duration

This gives AI agents full temporal awareness and control!