use serde::{Deserialize, Serialize};

/// Request input for the execute method
#[derive(Debug, Deserialize, Clone)]
pub struct ExecuteRequest {
    /// User's natural language request
    pub prompt: String,
    /// Optional AI model override
    pub model: Option<String>,
    /// Whether to include usage examples in context
    #[serde(default = "default_include_examples")]
    pub include_examples: bool,
}

/// Agent plan returned from context prism
#[derive(Debug, Deserialize)]
pub struct AgentPlan {
    /// User-facing response text
    pub response: String,
    /// AI's reasoning process
    pub reasoning: String,
    /// List of actions to execute
    #[serde(default)]
    pub actions: Vec<AgentAction>,
}

/// Individual action to execute
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AgentAction {
    /// Target prism (e.g., "aws:burner")
    pub prism: String,
    /// Frequency/method to call (e.g., "create")
    pub frequency: String,
    /// Input data for the action
    pub input: serde_json::Value,
    /// Human-readable description
    pub description: String,
}

/// Result of executing an action
#[derive(Debug, Serialize, Clone)]
pub struct ActionResult {
    /// The action that was executed
    pub action: AgentAction,
    /// Whether the action succeeded
    pub success: bool,
    /// Result data if successful
    pub result: Option<serde_json::Value>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Types of streaming events emitted by the agent
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    /// AI conversational response
    #[serde(rename = "ai_response")]
    AIResponse { content: String },
    
    /// AI reasoning/thinking process
    #[serde(rename = "ai_reasoning")] 
    AIReasoning { content: String },
    
    /// AI progress update (action started)
    #[serde(rename = "ai_progress")]
    AIProgress { content: String },
    
    /// Structured output from action execution
    #[serde(rename = "action_output")]
    ActionOutput { 
        action: AgentAction,
        success: bool,
        data: Option<serde_json::Value>,
        error: Option<String>,
    },
    
    /// Workflow completion
    #[serde(rename = "complete")]
    Complete,
}

/// Conversation context for iterative workflows
#[derive(Debug, Clone)]
pub struct ConversationContext {
    /// Original user prompt
    pub original_prompt: String,
    /// Conversation turns
    pub turns: Vec<ConversationTurn>,
}

/// A single turn in the conversation
#[derive(Debug, Clone)]
pub struct ConversationTurn {
    /// AI's response to the user
    pub response: String,
    /// AI's reasoning/scratch pad
    pub reasoning: String,
    /// Actions taken in this turn
    pub actions: Vec<AgentAction>,
    /// Results from the actions
    pub results: Vec<ActionResult>,
}

impl ConversationContext {
    /// Create a new conversation context
    pub fn new(original_prompt: String) -> Self {
        Self {
            original_prompt,
            turns: Vec::new(),
        }
    }
    
    /// Add a new turn to the conversation
    pub fn add_turn(&mut self, response: String, reasoning: String, actions: Vec<AgentAction>, results: Vec<ActionResult>) {
        self.turns.push(ConversationTurn {
            response,
            reasoning,
            actions,
            results,
        });
    }
    
    /// Check if this is the first turn
    pub fn is_first_turn(&self) -> bool {
        self.turns.is_empty()
    }
}

/// Default include_examples value: true
fn default_include_examples() -> bool {
    true
}
