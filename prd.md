# Product Requirements Document: AI-Powered Productivity Assistant

## 1. Introduction

### 1.1. Purpose
This Product Requirements Document (PRD) outlines the features, functionalities, and technical specifications for the AI-Powered Productivity Assistant. The application combines comprehensive activity tracking, intelligent research capabilities, and local AI processing to help users understand their work habits, conduct efficient research, and achieve peak productivity while maintaining complete data privacy.

### 1.2. Scope
The current version delivers a cross-platform desktop application (built with Tauri) featuring:
- Advanced activity tracking and real-time monitoring
- AI-powered research assistant with web scraping capabilities
- Goal management and productivity scoring
- Privacy-first architecture with local data processing
- Modern, responsive UI with smooth animations

Future enhancements such as mobile apps, cloud synchronization, and advanced integrations are considered out of scope for this version.

### 1.3. Target Audience
The primary target audience includes:
- Knowledge workers and researchers who need efficient information gathering tools
- Developers and technical professionals seeking productivity insights
- Students conducting research and managing study time
- Privacy-conscious users who prefer on-device AI processing
- Anyone seeking to optimize their digital productivity and work patterns

## 2. Product Overview

### 2.1. Vision
To be the leading privacy-focused productivity application that combines intelligent activity tracking with powerful AI research capabilities, helping users achieve peak productivity through actionable insights and efficient information gathering, all while keeping data completely private and on-device.

### 2.2. Key Features (High-level)
*   **Browser AI Research Assistant**: Advanced web research tool with intelligent query splitting, parallel web scraping, and AI-powered result synthesis
*   **Comprehensive Activity Tracking**: Monitor application usage, window titles, input metrics, and system events with real-time updates
*   **Goal Management System**: Create and track productivity goals with allowed applications and time-based targets
*   **Intelligent Project Detection**: Automatically identify and categorize work based on active applications, file paths, and window contexts
*   **Privacy-First Architecture**: All data stored and processed locally with no cloud uploads or external data sharing
*   **AI-Powered Insights**: Local LLM integration for productivity scoring, pattern analysis, and personalized recommendations
*   **Interactive Dashboards**: Real-time visualizations of productivity metrics, application usage, and goal progress
*   **Research Task Management**: Save, organize, and revisit research tasks with tagging and note-taking capabilities

### 2.3. User Stories (High-level)
*   As a user, I want to conduct comprehensive web research with AI assistance so I can quickly gather relevant information on any topic
*   As a user, I want to automatically track my application usage and time spent so I can understand where my time goes
*   As a user, I want to create and monitor productivity goals with specific applications and time targets so I can stay focused
*   As a user, I want to see real-time activity monitoring and productivity scores so I can adjust my work patterns immediately
*   As a user, I want to save and organize my research tasks with tags and notes so I can reference them later
*   As a user, I want to view historical reports and trends of my productivity so I can identify patterns and improvements
*   As a user, I want to receive AI-powered recommendations based on my work habits so I can optimize my workflow
*   As a user, I want all my data to remain on my device with no cloud uploads so I can maintain complete privacy
*   As a user, I want the application to automatically detect my projects and contexts so I don't need manual categorization
*   As a user, I want to see my research progress with visual indicators so I know the status of ongoing tasks

## 3. Detailed Feature Requirements

### 3.0. Browser AI Research Assistant

#### 3.0.1. Intelligent Research System
The system shall provide comprehensive web research capabilities with AI-powered analysis.
*   **Query Processing**:
    *   **Requirement**: Accept natural language research queries and automatically split them into focused subtasks
    *   **Technical Detail**: `BrowserAIAgent` uses LLM to decompose complex queries into 3-5 specific search tasks
    *   **Requirement**: Support parallel execution of research subtasks for improved performance
    *   **Technical Detail**: Configurable `max_concurrent_scrapes` (default: 3) for parallel web scraping
*   **Web Search Integration**:
    *   **Requirement**: Perform intelligent web searches using multiple search engines
    *   **Technical Detail**: `SearchEngine` trait implementation supports DuckDuckGo with extensibility for other engines
    *   **Requirement**: Extract and rank search results by relevance
    *   **Technical Detail**: `SearchResult` includes relevance scoring based on title/snippet matching
*   **Content Extraction**:
    *   **Requirement**: Scrape and extract meaningful content from web pages
    *   **Technical Detail**: `ScraperEngine` uses headless browser automation for JavaScript-heavy sites
    *   **Requirement**: Handle various content types including articles, documentation, and dynamic pages
    *   **Technical Detail**: Intelligent extraction using CSS selectors and content heuristics
*   **Result Synthesis**:
    *   **Requirement**: Generate comprehensive conclusions from gathered research data
    *   **Technical Detail**: LLM-powered synthesis creates summaries and insights from scraped content
    *   **Requirement**: Group results by subtask with relevance scoring
    *   **Technical Detail**: `ResearchResult` includes subtask association and relevance scores

#### 3.0.2. Research Task Management
*   **Task Tracking**:
    *   **Requirement**: Create unique identifiers for each research task
    *   **Technical Detail**: UUID-based task IDs with timestamp tracking
    *   **Requirement**: Track task status through multiple stages
    *   **Technical Detail**: `TaskStatus` enum: Pending, SplittingTasks, Searching, Scraping, Analyzing, Completed, Failed
*   **Progress Monitoring**:
    *   **Requirement**: Provide real-time progress updates during research
    *   **Technical Detail**: `BrowserAIProgress` events emitted via Tauri event system
    *   **Requirement**: Display subtask completion status
    *   **Technical Detail**: Progress includes current subtask, completed count, and percentage
*   **Data Persistence**:
    *   **Requirement**: Auto-save research progress periodically
    *   **Technical Detail**: Event-driven saves to SurrealDB via `save-research-progress` events
    *   **Requirement**: Support saving completed research with tags and notes
    *   **Technical Detail**: `SavedResearchTask` model with metadata in SurrealDB

#### 3.0.3. Browser Window Management
*   **Window Control**:
    *   **Requirement**: Create and manage browser windows for research tasks
    *   **Technical Detail**: `BrowserWindowManager` creates Tauri windows with web content
    *   **Requirement**: Support navigation and script execution in browser windows
    *   **Technical Detail**: JavaScript injection for DOM reading and interaction
*   **Content Reading**:
    *   **Requirement**: Extract DOM content from browser windows
    *   **Technical Detail**: `DOMReader` with specialized extraction scripts
    *   **Requirement**: Support platform-specific content extraction (e.g., Twitter)
    *   **Technical Detail**: Custom extraction scripts for social media platforms

### 3.1. Goal Management System

#### 3.1.1. Goal Creation and Configuration
*   **Goal Definition**:
    *   **Requirement**: Create productivity goals with name, duration, and allowed applications
    *   **Technical Detail**: `Goal` model with duration in minutes and app whitelist
    *   **Requirement**: Support multiple simultaneous goals
    *   **Technical Detail**: Goals stored in local storage with unique IDs
*   **Application Filtering**:
    *   **Requirement**: Define allowed applications for each goal
    *   **Technical Detail**: Case-insensitive app name matching in `is_app_allowed`
    *   **Requirement**: Track time only when using allowed applications
    *   **Technical Detail**: Real-time filtering during activity tracking

#### 3.1.2. Goal Tracking and Progress
*   **Progress Monitoring**:
    *   **Requirement**: Calculate and display goal progress as percentage
    *   **Technical Detail**: `update_progress` calculates percentage based on time spent vs duration
    *   **Requirement**: Track active goal sessions
    *   **Technical Detail**: `GoalSession` model tracks start/end times and duration
*   **Goal Activation**:
    *   **Requirement**: Support activating/deactivating goals
    *   **Technical Detail**: `is_active` flag with timestamp tracking
    *   **Requirement**: Only one goal active at a time
    *   **Technical Detail**: Goal service ensures single active goal constraint

### 3.2. Activity Tracking System

#### 3.2.1. Cross-Platform Activity Monitoring
The system shall continuously monitor and record user activity on macOS.
*   **Application Usage**:
    *   **Requirement**: Track the currently active application's name, bundle ID, and window title.
    *   **Technical Detail**: Implemented via `AppWatcher` using `NSWorkspace` and `CGWindowListCopyWindowInfo` APIs.
    *   **Requirement**: Categorize applications (e.g., Development, Social Media, Entertainment, Productivity) and determine if they are productive based on predefined rules.
    *   **Technical Detail**: `AppWatcher` contains logic for categorization and productivity assessment.
    *   **Requirement**: For web browsers, attempt to extract the current website URL.
    *   **Technical Detail**: `AppWatcher` includes placeholder logic for `detect_browser_url` which currently extracts page titles from window titles. Full URL extraction requires more advanced macOS API integration or browser extensions (out of scope for v1).
    *   **Requirement**: For code editors, attempt to extract the current open file path.
    *   **Technical Detail**: `AppWatcher` includes placeholder logic for `detect_editor_file` which currently extracts filenames from window titles.
    *   **Requirement**: For terminal applications, attempt to extract the current working directory and last executed command.
    *   **Technical Detail**: `AppWatcher` includes placeholder logic for `detect_terminal_details` which currently extracts directory from window titles and notes that command history requires AppleScript.
*   **Input Monitoring**:
    *   **Requirement**: Monitor keyboard keystrokes and mouse clicks/movement.
    *   **Technical Detail**: `InputMonitor` is a placeholder; actual implementation requires macOS CoreGraphics/CoreFoundation event taps.
    *   **Requirement**: Provide statistics on keystrokes, mouse clicks, and mouse distance.
    *   **Technical Detail**: `InputMonitor` provides `InputStatistics` struct. Note: `Activity` model expects `InputMetrics` which is more detailed; a conversion or refinement is needed.
*   **System Events**:
    *   **Requirement**: Monitor system idle time, screen lock status, battery percentage, and power adapter status.
    *   **Technical Detail**: `SystemEventMonitor` is a placeholder; actual implementation requires macOS IOKit and `CGEventSourceSecondsSinceLastEventType` APIs.
    *   **Requirement**: Monitor CPU and memory usage.
    *   **Technical Detail**: `SystemEventMonitor` is a placeholder; actual implementation requires host_statistics or similar APIs.

#### 3.2.2. Project Detection
The system shall intelligently detect the project a user is working on.
*   **Requirement**: Automatically identify project context based on active applications (e.g., terminal, code editor) and their associated directories/window titles.
*   **Technical Detail**: `ProjectDetector` uses `osascript` for terminal directory detection (Terminal.app, iTerm2) and heuristics based on window titles for VSCode.
*   **Requirement**: Traverse up the directory tree to find a project root indicated by common project files (e.g., `Cargo.toml`, `package.json`, `.git`).
*   **Technical Detail**: `ProjectDetector` implements `is_project_directory` and `get_project_from_directory` for this purpose.
*   **Requirement**: Determine the project type (e.g., Rust, JavaScript/Node, Python, Go) based on detected project files.
*   **Technical Detail**: `ProjectDetector` implements `detect_project_type`.

#### 3.2.3. Data Collection & Privacy
The system shall adhere to strict privacy principles.
*   **Requirement**: All activity data shall be stored exclusively on the user's local machine.
*   **Technical Detail**: `ActivityStore` currently uses a local JSON file (`activity_data.db`).
*   **Requirement**: No activity data shall be uploaded to any cloud service or shared with third parties.
*   **Technical Detail**: Enforced by design; no network calls for data export are implemented.
*   **Requirement**: Users shall have control over data retention, with an option to prune old data.
*   **Technical Detail**: `ActivityStore` implements `prune_old_activities`. Default retention is 90 days (`DEFAULT_DATA_RETENTION_DAYS`).
*   **Requirement**: Data shall be collected periodically (e.g., every 5 seconds).
*   **Technical Detail**: Configurable via `DEFAULT_TRACKING_INTERVAL` (5 seconds). `ActivityTracker` manages the collection loop.

### 3.3. Best Practices Feedback Engine
*This section is largely conceptual in the current codebase, relying on LLM insights for feedback rather than explicit rule-based patterns.*

#### 3.3.1. Pattern Recognition
*   **Requirement**: Identify unhealthy work patterns (e.g., long sessions without breaks, excessive context switching).
*   **Technical Detail**: Primarily handled by the LLM integration (`llm_integration/prompt_templates.rs` for insights and recommendations) which analyzes raw activity data. No explicit rule-based pattern recognition engine is currently implemented.

#### 3.3.2. Real-time Notifications
*   **Requirement**: Send gentle reminders for breaks or suggestions based on detected behavior.
*   **Technical Detail**: Not explicitly implemented in the current Rust backend. This would likely be a UI-driven feature triggered by LLM insights or simple time-based rules.

### 3.4. Current Activity Dashboard
*This section describes the real-time view of user activity.*

#### 3.4.1. Real-time Monitoring View
*   **Requirement**: Display the currently active application, its window title, and the duration of its use.
*   **Technical Detail**: `get_current_activity` Tauri command retrieves this data from `ActivityTracker`.
*   **Requirement**: Show the current project being worked on.
*   **Technical Detail**: `get_current_project` Tauri command retrieves this data from `ActivityTracker`.
*   **Requirement**: Display a real-time productivity score or focus indicator.
*   **Technical Detail**: `get_productivity_score` Tauri command can be used to fetch a score, though it's LLM-generated and not strictly "real-time" in the sense of continuous updates.

### 3.5. Productivity Analysis Dashboard
*This section describes the historical analysis and visualization features.*

#### 3.5.1. Habit Analysis
*   **Requirement**: Analyze historical activity data to identify productive and unproductive habits.
*   **Technical Detail**: Primarily handled by the LLM integration (`llm_integration/prompt_templates.rs` for insights).

#### 3.5.2. Visual Reports
The system shall provide various visual reports to help users understand their activity patterns.
*   **Requirement**: **Productivity Timeline**: A chart showing productive vs. unproductive time over a selected period (e.g., daily, weekly).
*   **Technical Detail**: `AnalysisEngine::generate_productivity_timeline` generates `ChartData` (line chart) based on `is_productive` flag.
*   **Requirement**: **Application Usage**: A chart showing time spent on different applications.
*   **Technical Detail**: `AnalysisEngine::generate_app_usage_chart` generates `ChartData` (bar chart) for top 10 applications.
*   **Requirement**: **Project Distribution**: A chart showing time distribution across different projects.
*   **Technical Detail**: `AnalysisEngine::generate_project_distribution` generates `ChartData` (pie chart) based on detected projects.
*   **Requirement**: **Productivity Score History**: A chart showing the trend of the overall productivity score over time.
*   **Technical Detail**: `AnalysisEngine::generate_score_history` generates `ChartData` (line chart) from historical `ProductivityScore` data.

### 3.6. Productivity Enhancement Tools

#### 3.6.1. Personalized Recommendations
*   **Requirement**: Provide actionable recommendations for improving productivity based on analyzed activity data.
*   **Technical Detail**: `llm_integration::LlmClient::generate_recommendations` uses LLM to generate a list of recommendations.

#### 3.6.2. Habit Formation Assistance
*   **Requirement**: (Conceptual) Track habit streaks and provide encouraging feedback.
*   **Technical Detail**: Not explicitly implemented in the current Rust backend. This would likely build upon the analysis engine and UI.

#### 3.6.3. Manual Activity Entry
*   **Requirement**: Allow users to manually record activity entries for periods not automatically tracked or for specific tasks.
*   **Technical Detail**: `record_manual_activity` Tauri command is a placeholder; implementation needed to create and store manual `Activity` records.

### 3.7. Local LLM Integration

#### 3.7.1. On-device Processing
*   **Requirement**: All LLM inference shall occur locally on the user's device.
*   **Technical Detail**: `llm_integration::ollama` communicates with a local Ollama server (`http://localhost:11434`).
*   **Requirement**: Support configurable LLM models and providers.
*   **Technical Detail**: `LlmConfig` allows setting `model_name` and `provider` (though only Ollama is currently implemented).

#### 3.7.2. Intelligent Analysis
*   **Requirement**: Generate natural language insights about productivity patterns.
*   **Technical Detail**: `llm_integration::LlmClient::generate_productivity_insights` uses LLM to provide a summary, key insights, and suggested improvements.
*   **Requirement**: Calculate a multi-component productivity score (overall, focus, efficiency, break).
*   **Technical Detail**: `llm_integration::LlmClient::generate_productivity_score` uses LLM to return a JSON object mapping to `ProductivityScore` model.
*   **Requirement**: Provide specific, actionable recommendations.
*   **Technical Detail**: `llm_integration::LlmClient::generate_recommendations` uses LLM to return a JSON array of recommendation strings.
*   **Requirement**: Activity data provided to the LLM shall be in a structured JSON format.
*   **Technical Detail**: `prompt_templates.rs` serializes `Activity` data into JSON within the prompt.

## 4. Technical Specifications

### 4.1. Architecture Overview
*   **Backend**: Rust
    *   Core logic for activity tracking, browser AI, data storage, and LLM integration
    *   Asynchronous task management with Tokio runtime
    *   Modular service architecture with clear separation of concerns
*   **Frontend**: React + Vite + Tailwind CSS
    *   Modern component-based UI with hooks for state management
    *   Framer Motion for smooth animations and transitions
    *   Recharts for data visualization
    *   Responsive design with dark theme
*   **Desktop Framework**: Tauri
    *   Cross-platform desktop application
    *   Secure IPC between frontend and backend
    *   Native system integration and window management
*   **Database**: SurrealDB
    *   Embedded database for research task persistence
    *   Schema-less design for flexible data storage

### 4.2. Data Storage
*   **Activity Data**: Local JSON file (`activity_data.db`) for activity records
    *   Aggregation by hour and application for efficient retrieval
    *   90-day default retention with configurable pruning
*   **Research Data**: SurrealDB for research tasks and results
    *   Structured storage with relationships between tasks, subtasks, and results
    *   Support for tags and notes on saved research
*   **Goal Data**: Local storage for goal definitions and sessions
    *   Real-time progress tracking and session management
*   **Future Enhancement**: Unified SQLite database for all data types

### 4.3. LLM Integration Details
*   **Providers**: 
    *   Ollama (default): Local LLM via `http://localhost:11434/api/generate`
    *   Google Generative AI: Optional cloud-based LLM for enhanced capabilities
*   **Use Cases**:
    *   Research query decomposition into subtasks
    *   Content extraction and summarization from web pages
    *   Productivity insights and scoring
    *   Personalized recommendations
    *   Research conclusion synthesis
*   **Communication**: 
    *   HTTP POST requests via `reqwest` for Ollama
    *   Google AI SDK for Generative AI integration
*   **Prompt Engineering**: 
    *   Dynamic prompt generation with context injection
    *   Structured JSON output formats for reliable parsing
    *   System prompts for consistent AI behavior

### 4.4. Key Data Models

#### Core Activity Models
*   **`Activity`**: User activity record with timestamp, duration, application details, and metrics
*   **`AppUsage`**: Application details including name, bundle ID, window title, and productivity classification
*   **`InputMetrics`**: Keyboard and mouse interaction statistics
*   **`SystemState`**: System metrics like CPU, memory, battery, and idle time
*   **`ProjectContext`**: Detected project information with type classification

#### Research Models
*   **`ResearchTask`**: Main research task with query, status, subtasks, and results
*   **`ResearchResult`**: Individual search result with content, relevance score, and metadata
*   **`BrowserAIProgress`**: Real-time progress updates for research tasks
*   **`SavedResearchTask`**: Persisted research with tags and notes

#### Goal Models
*   **`Goal`**: Productivity goal with duration, allowed apps, and progress tracking
*   **`GoalSession`**: Individual goal session with timing information

#### Analytics Models
*   **`ProductivityScore`**: Multi-dimensional score (overall, focus, efficiency, break)
*   **`ProductivityInsights`**: AI-generated insights and recommendations
*   **`ChartData`**: Visualization data for various chart types

### 4.5. Error Handling Strategy
*   **Error Types**: Custom `AppError` enum with categories:
    *   Platform-specific errors (macOS API, Windows API)
    *   Database errors (Storage, SurrealDB)
    *   LLM errors (connection, parsing)
    *   Browser AI errors (scraping, timeout)
    *   Configuration and permission errors
*   **Error Propagation**: Consistent `Result<T, AppError>` pattern
*   **User Experience**: 
    *   Graceful degradation for non-critical failures
    *   User-friendly error messages via notification system
    *   Detailed logging for debugging
*   **Recovery Strategies**:
    *   Automatic retry for transient network errors
    *   Fallback mechanisms for LLM and scraping failures
    *   Queue persistence for failed research tasks

## 5. Future Considerations / Out of Scope

### 5.1. Out of Scope for Current Release
*   **Mobile Applications**: iOS and Android apps
*   **Cloud Synchronization**: Multi-device sync (privacy-first principle)
*   **Browser Extensions**: Deep browser integration for enhanced tracking
*   **Team Features**: Collaboration and team productivity metrics
*   **Advanced AI Models**: Custom-trained models for specific domains
*   **API Access**: Third-party integrations and webhooks
*   **Advanced Automation**: Workflow automation based on patterns

### 5.2. Potential Future Enhancements
*   **Enhanced Research Capabilities**:
    *   Support for academic paper analysis
    *   Integration with knowledge bases
    *   Citation management
    *   Multi-language research support
*   **Advanced Analytics**:
    *   Machine learning for pattern prediction
    *   Anomaly detection in work patterns
    *   Personalized productivity coaching
*   **Platform Expansion**:
    *   Linux support
    *   Web-based dashboard
    *   Mobile companion apps
*   **Integration Ecosystem**:
    *   Calendar integration
    *   Task management tools
    *   Time tracking APIs
    *   IDE plugins

## 6. Success Metrics

### 6.1. User Engagement Metrics
*   **Daily Active Users**: >60% of installed base
*   **Research Tasks Created**: Average 5+ per week per active user
*   **Goals Completion Rate**: >40% of created goals reaching target
*   **Feature Adoption**: >70% users utilizing both tracking and research features

### 6.2. Technical Performance Metrics
*   **Resource Usage**: <2% CPU and <100MB RAM during idle
*   **Research Speed**: Average task completion <30 seconds
*   **Data Accuracy**: >95% precision in activity tracking
*   **Crash Rate**: <0.1% of sessions
*   **Response Time**: UI interactions <100ms

### 6.3. Privacy & Security Metrics
*   **Data Leakage**: Zero instances of unauthorized data transmission
*   **Local Processing**: 100% of AI inference on-device
*   **Audit Compliance**: Pass quarterly security audits

### 6.4. User Satisfaction Metrics
*   **App Store Rating**: 4.5+ stars
*   **NPS Score**: >50
*   **Support Tickets**: <5% of active users
*   **Feature Requests**: High engagement in feedback channels

## 7. UI/UX Design Principles

### 7.1. Visual Design
*   **Dark Theme First**: Optimized for extended use with reduced eye strain
*   **Color System**: 
    *   Primary: Blue (#3b82f6) for actions and focus
    *   Secondary: Purple (#8b5cf6) for accents
    *   Success: Green (#10b981) for positive feedback
    *   Warning: Orange (#f59e0b) for alerts
    *   Danger: Red (#ef4444) for errors
*   **Typography**: Clean, readable fonts with clear hierarchy
*   **Animations**: Smooth transitions using Framer Motion for polish

### 7.2. User Experience
*   **Intuitive Navigation**: Sidebar-based navigation with clear sections
*   **Progressive Disclosure**: Complex features revealed as needed
*   **Real-time Feedback**: Immediate visual responses to user actions
*   **Keyboard Shortcuts**: Power user features for efficiency
*   **Responsive Design**: Adapts to different window sizes

### 7.3. Component Library
*   **Cards**: Consistent container styling with hover effects
*   **Buttons**: Multiple variants (primary, secondary, ghost) with loading states
*   **Modals**: Smooth animations with backdrop blur
*   **Progress Indicators**: Visual feedback for long operations
*   **Notifications**: Non-intrusive toast-style alerts

## 8. Security & Privacy Considerations

### 8.1. Data Protection
*   **Local Storage Only**: No cloud uploads or external servers
*   **Encryption**: Sensitive data encrypted at rest
*   **Access Control**: Application-level sandboxing
*   **Data Retention**: Configurable automatic pruning

### 8.2. Permission Management
*   **Explicit Consent**: Clear permission requests for system access
*   **Granular Controls**: Users can disable specific tracking features
*   **Transparency**: Clear indication of what data is collected

### 8.3. Third-party Services
*   **LLM Security**: Local models only, no data sent to cloud
*   **Web Scraping**: Respects robots.txt and rate limits
*   **No Analytics**: No third-party analytics or tracking



1) Use tauri 2
2) For Research AI agent use playwright which has more control
3) use genai rust library for communication ollama
