mod execution;
mod state;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use anyhow::bail;
use rexos_kernel::router::TaskKind;
use rexos_tools::Toolset;

use execution::{
    emit_workflow_finished, emit_workflow_started, execute_workflow_step,
    record_workflow_step_failure, record_workflow_step_success, serialize_workflow_step_arguments,
    workflow_result_json,
};
use state::{build_workflow_state, finalize_workflow_state, mark_workflow_step_running};

use crate::records::WorkflowRunToolArgs;
use crate::{workflow_state_path, AgentRuntime};

impl AgentRuntime {
    pub(crate) async fn workflow_run(
        &self,
        workspace_root: &PathBuf,
        session_id: &str,
        _kind: TaskKind,
        args: WorkflowRunToolArgs,
    ) -> anyhow::Result<String> {
        if args.steps.is_empty() {
            bail!("workflow_run requires at least one step");
        }

        let workflow_id = args
            .workflow_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut state = build_workflow_state(
            &workflow_id,
            args.name.clone(),
            session_id,
            &args.steps,
            Self::now_epoch_seconds(),
        );
        let state_path = workflow_state_path(workspace_root, &workflow_id);
        self.write_workflow_state(&state_path, &state)?;

        let mut policy = self.load_session_policy_snapshot(session_id)?;
        let allowed_tools = policy.allowed_tools.take();
        let tools = Toolset::new_with_allowed_tools_security_and_mcp_config(
            workspace_root.clone(),
            allowed_tools,
            self.security.clone(),
            policy.mcp_config_json.as_deref(),
        )
        .await?;
        let continue_on_error = args.continue_on_error.unwrap_or(false);
        let mut failed_steps = 0usize;

        emit_workflow_started(self, session_id, &workflow_id, state.steps.len());

        for (idx, step) in args.steps.iter().enumerate() {
            let started_at = Self::now_epoch_seconds();
            mark_workflow_step_running(&mut state, idx, started_at);
            self.write_workflow_state(&state_path, &state)?;

            let args_json = serialize_workflow_step_arguments(&step.arguments)?;
            let step_res = execute_workflow_step(
                self,
                &tools,
                session_id,
                &workflow_id,
                idx,
                step,
                &args_json,
            )
            .await;

            let completed_at = Self::now_epoch_seconds();
            match step_res {
                Ok(output) => {
                    record_workflow_step_success(
                        self,
                        &mut state,
                        session_id,
                        &workflow_id,
                        idx,
                        &step.tool,
                        completed_at,
                        output,
                    );
                }
                Err(error) => {
                    failed_steps = failed_steps.saturating_add(1);
                    let error = error.to_string();
                    record_workflow_step_failure(
                        self,
                        &mut state,
                        session_id,
                        &workflow_id,
                        idx,
                        &step.tool,
                        completed_at,
                        &error,
                    );
                    state.updated_at = completed_at;
                    self.write_workflow_state(&state_path, &state)?;
                    if !continue_on_error {
                        break;
                    }
                }
            }

            state.updated_at = completed_at;
            self.write_workflow_state(&state_path, &state)?;
        }

        finalize_workflow_state(&mut state, Self::now_epoch_seconds());
        self.write_workflow_state(&state_path, &state)?;

        emit_workflow_finished(self, session_id, &workflow_id, &state.status, failed_steps);

        Ok(workflow_result_json(&state, failed_steps, &state_path))
    }
}
