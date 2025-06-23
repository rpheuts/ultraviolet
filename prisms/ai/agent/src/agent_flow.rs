use uuid::Uuid;

use crate::spectrum::{ExecuteRequest, AgentPlan, AgentEvent, ActionResult, ConversationContext};
use crate::context_collector::ContextCollector;
use crate::action_executor::ActionExecutor;
use uv_core::{Result, UVLink, UVSpectrum};

/// Orchestrates the complete agentic workflow
pub struct AgentFlow {
    context_collector: ContextCollector,
    action_executor: ActionExecutor,
}

impl AgentFlow {
    /// Create a new AgentFlow
    pub fn new(spectrum: UVSpectrum) -> Self {
        Self {
            context_collector: ContextCollector::new(spectrum),
            action_executor: ActionExecutor::new(),
        }
    }

    /// Execute iterative agentic workflow with conversation context
    pub async fn execute_iterative_workflow(&self, request: ExecuteRequest, id: Uuid, link: &UVLink) -> Result<()> {
        let mut context = ConversationContext::new(request.prompt.clone());
        
        loop {
            // Build contextual request for this iteration
            let contextual_request = self.build_contextual_request(&request, &context);
            
            // 1. Get plan from context prism
            let plan = self.context_collector.collect_and_parse(contextual_request).await?;

            // 2. Stream the response and reasoning
            self.stream_plan_response(&plan, id, link).await?;

            // 3. Execute actions if any
            let action_results = if !plan.actions.is_empty() {
                self.execute_actions_with_streaming(&plan.actions, id, link).await?
            } else {
                Vec::new()
            };

            // 4. Add this turn to conversation context
            context.add_turn(
                plan.response.clone(),
                plan.reasoning.clone(),
                plan.actions.clone(),
                action_results,
            );

            // 5. Check if AI is done (no actions = complete)
            if plan.actions.is_empty() {
                break;
            }
        }

        // Final completion
        self.stream_event(&AgentEvent::Complete, id, link).await?;
        link.emit_trap(id, None)?;

        Ok(())
    }

    /// Build a contextual request incorporating conversation history
    fn build_contextual_request(&self, original_request: &ExecuteRequest, context: &ConversationContext) -> ExecuteRequest {
        if context.is_first_turn() {
            // First turn, use original request
            original_request.clone()
        } else {
            // Build prompt with conversation history
            let mut contextual_prompt = format!("# Original Request\n{}\n\n", context.original_prompt);
            
            contextual_prompt.push_str("# Conversation History\n");
            for (i, turn) in context.turns.iter().enumerate() {
                contextual_prompt.push_str(&format!("## Turn {}\n", i + 1));
                contextual_prompt.push_str(&format!("**Your Response:** {}\n", turn.response));
                contextual_prompt.push_str(&format!("**Your Reasoning:** {}\n", turn.reasoning));
                
                if !turn.actions.is_empty() {
                    contextual_prompt.push_str("**Actions Taken:**\n");
                    for action in &turn.actions {
                        contextual_prompt.push_str(&format!("- {}: {}\n", action.prism, action.description));
                    }
                    
                    contextual_prompt.push_str("**Results:**\n");
                    for result in &turn.results {
                        if result.success {
                            if let Some(data) = &result.result {
                                contextual_prompt.push_str(&format!("- ✓ {}: {}\n", 
                                    result.action.description,
                                    serde_json::to_string_pretty(data).unwrap_or_else(|_| "Unable to format result".to_string())
                                ));
                            } else {
                                contextual_prompt.push_str(&format!("- ✓ {}: Success\n", result.action.description));
                            }
                        } else {
                            contextual_prompt.push_str(&format!("- ✗ {}: {}\n", 
                                result.action.description,
                                result.error.as_deref().unwrap_or("Unknown error")
                            ));
                        }
                    }
                }
                contextual_prompt.push('\n');
            }
            
            contextual_prompt.push_str("# Next Steps\nBased on the conversation above, what should you do next? If the original request has been fully satisfied, respond without any actions.\n");
            
            ExecuteRequest {
                prompt: contextual_prompt,
                model: original_request.model.clone(),
                include_examples: original_request.include_examples,
            }
        }
    }

    /// Stream the response and reasoning from the plan
    async fn stream_plan_response(&self, plan: &AgentPlan, id: Uuid, link: &UVLink) -> Result<()> {
        // Stream AI response
        let response_event = AgentEvent::AIResponse {
            content: plan.response.clone(),
        };
        self.stream_event(&response_event, id, link).await?;

        // Stream AI reasoning
        let reasoning_event = AgentEvent::AIReasoning {
            content: plan.reasoning.clone(),
        };
        self.stream_event(&reasoning_event, id, link).await?;

        Ok(())
    }

    /// Execute actions with streaming progress updates
    async fn execute_actions_with_streaming(&self, actions: &[crate::spectrum::AgentAction], id: Uuid, link: &UVLink) -> Result<Vec<ActionResult>> {
        let mut results = Vec::new();

        for action in actions {
            // Stream AI progress update
            let progress_event = AgentEvent::AIProgress {
                content: format!("Executing: {}", action.description),
            };
            self.stream_event(&progress_event, id, link).await?;

            // Execute the action
            let result = self.action_executor.execute_action(action).await?;

            // Stream AI progress result
            let progress_content = if result.success {
                format!("✓ {}", action.description)
            } else {
                format!("✗ {} - Error: {}", action.description, result.error.as_deref().unwrap_or("Unknown"))
            };

            let progress_result_event = AgentEvent::AIProgress {
                content: progress_content,
            };
            self.stream_event(&progress_result_event, id, link).await?;

            // Stream structured action output
            let action_output_event = AgentEvent::ActionOutput {
                action: action.clone(),
                success: result.success,
                data: result.result.clone(),
                error: result.error.clone(),
            };
            self.stream_event(&action_output_event, id, link).await?;

            // If action failed, stop execution (fail-fast)
            if !result.success {
                results.push(result);
                break;
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Stream a single event
    async fn stream_event(&self, event: &AgentEvent, id: Uuid, link: &UVLink) -> Result<()> {
        let event_json = serde_json::to_value(event)?;
        link.emit_photon(id, event_json)?;
        Ok(())
    }
}
