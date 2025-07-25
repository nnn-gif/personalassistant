# Browser AI Module

This module provides intelligent browser automation capabilities using Chrome DevTools Protocol (CDP) and Large Language Models (LLMs).

## Features

### 1. **Chrome DevTools Protocol Integration**
- Full programmatic control over Chrome browser
- Cross-platform support (macOS, Windows, Linux)
- Can launch new Chrome instances or connect to existing ones
- Headless and headful modes

### 2. **LLM-Driven Automation**
- Natural language task execution
- Automatic action generation based on page context
- Support for multiple LLM providers through `genai` crate
- Optional vision mode for screenshot-based understanding

### 3. **Smart Form Filling**
- Fill forms using natural language descriptions
- Automatic field detection and mapping
- Handles various input types (text, select, checkbox, etc.)

### 4. **Intelligent Web Scraping**
- Extract structured data using natural language prompts
- No need to write CSS selectors or XPath
- Returns data as JSON

## Architecture

```
browser_ai/
├── mod.rs              # Module exports
├── cdp_client.rs       # Chrome DevTools Protocol client
├── chrome.rs           # High-level Chrome controller with LLM integration
├── agent.rs            # Browser AI agent (existing)
└── scraper.rs          # Web scraping utilities (existing)
```

## Usage Examples

### Basic Browser Control
```rust
let mut chrome = ChromeController::new();
chrome.launch_browser(false).await?;
chrome.open_url("https://example.com").await?;
chrome.close().await?;
```

### LLM-Driven Automation
```rust
let mut chrome = ChromeController::with_llm("claude-3-5-sonnet-20241022").await?;
chrome.launch_browser(false).await?;

// Execute natural language tasks
chrome.execute_task("Search for Rust tutorials on Google").await?;
```

### Smart Form Filling
```rust
chrome.fill_form_smart(r#"
    Name: John Doe
    Email: john@example.com
    Country: United States
    Accept terms: Yes
"#).await?;
```

### Web Scraping with LLM
```rust
let data = chrome.extract_structured_data(r#"
    Extract all product names, prices, and ratings
"#).await?;
```

## How It Works

1. **DOM Serialization**: Interactive elements are extracted and serialized into a structured text format that LLMs can understand
2. **Action Generation**: The LLM analyzes the page state and generates appropriate actions
3. **Action Execution**: Actions are executed through CDP commands
4. **Feedback Loop**: The process continues until the task is complete

## Configuration

Set your LLM API key in the environment:
```bash
export ANTHROPIC_API_KEY="your-key-here"
# or
export OPENAI_API_KEY="your-key-here"
```

## Key Components

### PageState
Represents the current state of a web page:
- URL and title
- Interactive elements with their properties
- Optional screenshot for vision-enabled LLMs
- Console logs for debugging

### BrowserAction
Supported browser actions:
- `Click`: Click an element by index
- `Type`: Type text into an input field
- `Select`: Select an option from a dropdown
- `Navigate`: Go to a URL
- `Wait`: Wait for an element to appear
- `Screenshot`: Capture a screenshot
- `Complete`: Mark task as complete

### DomElement
Represents an interactive element on the page:
- Index for identification
- HTML tag name
- Text content
- Attributes (id, class, name, value, etc.)
- Visibility and interactivity flags

## Advanced Features

### Vision Mode
Enable vision mode to send screenshots to vision-capable LLMs:
```rust
chrome.set_vision_mode(true);
```

### Custom Chrome Launch
Connect to an existing Chrome instance with debugging enabled:
```rust
// Start Chrome with: --remote-debugging-port=9222
chrome.connect_to_browser("ws://localhost:9222/devtools/browser/...").await?;
```

## Limitations

- Maximum 50 iterations per task to prevent infinite loops
- Requires Chrome/Chromium to be installed
- LLM costs apply for each interaction
- Some complex JavaScript interactions may require custom handling

## Future Enhancements

- Session recording and replay
- Parallel browser sessions
- Enhanced error recovery
- Browser extension integration
- Mobile browser support