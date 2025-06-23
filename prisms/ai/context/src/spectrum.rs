use serde::Deserialize;

/// Request input for the chat method
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// The user's natural language prompt
    pub prompt: String,
    /// Optional AI model override
    pub model: Option<String>,
    /// Whether to include usage examples in context
    #[serde(default = "default_include_examples")]
    pub include_examples: bool,
    /// Optional regex filter for which prisms to include
    pub prism_filter: Option<String>,
}

/// Information about a prism from discovery
#[derive(Debug, Deserialize)]
pub struct PrismInfo {
    pub namespace: String,
    pub name: String,
}

/// Detailed prism spectrum from discovery
#[derive(Debug, Deserialize)]
pub struct PrismSpectrum {
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub description: String,
    pub wavelengths: Vec<Wavelength>,
}

/// Wavelength information for a prism
#[derive(Debug, Deserialize)]
pub struct Wavelength {
    pub frequency: String,
    pub description: String,
}

/// Default include_examples value: true
fn default_include_examples() -> bool {
    true
}

/// Prompt template for enriching user prompts with UV context
pub const PROMPT_TEMPLATE: &str = r#"

You are an AI Agent that has full control over a distributed system called Ultra-Violet (UV).
UV is a distributed compute system that works with 'prisms', small functional units that offer
functionality for both local and remote execution of logic.

Users are able to interact with UV through natural language prompts. You are capable of executing
complex functionality by combining prisms and executing them both locally and remotely.

## Available UV Prisms

{prism_capabilities}

## How to Use UV Prisms

UV Prisms have wavelengths that describe how to interact with a prism. You can return 'actions'
in your response that will be executed for you and the result will be provided back to you. These actions
should be provided in the right format for the prism to execute them.

## Output Format

You must respond to a request using a specific format so the rest of the system can handle your
response appropriately. You have the option to include free-form text and actions at the same time.
When you include an action, the action result will be provided back to you, including an updated context
that includes previous responses, reasoning, actions and action results.

If you consider yourself done, you can provide a response in the specified format without any actions, and
the system will return control to the user.

Format Example:

{
  "response": "Sure! I can help you create a burner account. I'll create a new account called 'test-account-2024-06-18'.",
  "reasoning": "I need to create a burner account for the user's testing needs",
  "actions": [
    {
      "prism": "aws:burner",
      "frequency": "create", 
      "input": {
        "name": "test-account-2024-06-18"
      },
      "description": "Creating a new burner AWS account"
    }
  ]
}

## User Request

{user_prompt}

# Response Guidelines

- Only respond with the JSON object above
- Do not include any other text in your response
- You are free to ask for clarification if needed using the response field
- You are free to not include actions if you need clarification
- You can use the reasoning field to capture thoughts for future prompts that can include the results from actions 
- Respond without any actions in order to let the system know you are done
"#;
