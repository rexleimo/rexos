use anyhow::{bail, Context};
use serde::de::DeserializeOwned;

use crate::defs::{
    A2aDiscoverArgs, A2aSendArgs, ApplyPatchArgs, BrowserClickArgs, BrowserNavigateArgs,
    BrowserPressKeyArgs, BrowserRunJsArgs, BrowserScreenshotArgs, BrowserScrollArgs,
    BrowserTypeArgs, BrowserWaitArgs, BrowserWaitForArgs, CanvasPresentArgs, DockerExecArgs,
    FileListArgs, FileReadArgs, FileWriteArgs, FsReadArgs, FsWriteArgs, ImageAnalyzeArgs,
    ImageGenerateArgs, MediaDescribeArgs, MediaTranscribeArgs, PdfArgs, ProcessKillArgs,
    ProcessPollArgs, ProcessStartArgs, ProcessWriteArgs, ShellArgs, ShellExecArgs,
    SpeechToTextArgs, TextToSpeechArgs, WebFetchArgs, WebSearchArgs,
};
use crate::Toolset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolCallDomain {
    Fs,
    Process,
    Web,
    Media,
    Browser,
    RuntimeCompat,
}

pub(crate) fn tool_call_domain(name: &str) -> Option<ToolCallDomain> {
    match name {
        "fs_read" | "file_read" | "fs_write" | "file_write" | "file_list" | "apply_patch" => {
            Some(ToolCallDomain::Fs)
        }
        "shell" | "shell_exec" | "docker_exec" | "process_start" | "process_poll"
        | "process_write" | "process_kill" | "process_list" => Some(ToolCallDomain::Process),
        "web_fetch" | "pdf" | "pdf_extract" | "web_search" | "a2a_discover" | "a2a_send"
        | "location_get" => Some(ToolCallDomain::Web),
        "image_analyze" | "media_describe" | "media_transcribe" | "speech_to_text"
        | "text_to_speech" | "image_generate" | "canvas_present" => Some(ToolCallDomain::Media),
        "browser_navigate" | "browser_back" | "browser_close" | "browser_click"
        | "browser_type" | "browser_press_key" | "browser_scroll" | "browser_wait"
        | "browser_wait_for" | "browser_read_page" | "browser_run_js" | "browser_screenshot" => {
            Some(ToolCallDomain::Browser)
        }
        "memory_store"
        | "memory_recall"
        | "agent_send"
        | "agent_spawn"
        | "agent_list"
        | "agent_kill"
        | "agent_find"
        | "hand_list"
        | "hand_activate"
        | "hand_status"
        | "hand_deactivate"
        | "task_post"
        | "task_claim"
        | "task_complete"
        | "task_list"
        | "event_publish"
        | "schedule_create"
        | "schedule_list"
        | "schedule_delete"
        | "knowledge_add_entity"
        | "knowledge_add_relation"
        | "knowledge_query"
        | "cron_create"
        | "cron_list"
        | "cron_cancel"
        | "channel_send"
        | "workflow_run" => Some(ToolCallDomain::RuntimeCompat),
        _ => None,
    }
}

impl Toolset {
    pub async fn call(&self, name: &str, arguments_json: &str) -> anyhow::Result<String> {
        self.ensure_tool_allowed(name)?;

        match tool_call_domain(name) {
            Some(ToolCallDomain::Fs) => self.call_fs_tool(name, arguments_json),
            Some(ToolCallDomain::Process) => self.call_process_tool(name, arguments_json).await,
            Some(ToolCallDomain::Web) => self.call_web_tool(name, arguments_json).await,
            Some(ToolCallDomain::Media) => self.call_media_tool(name, arguments_json),
            Some(ToolCallDomain::Browser) => self.call_browser_tool(name, arguments_json).await,
            Some(ToolCallDomain::RuntimeCompat) => Self::call_runtime_compat_tool(name),
            None => bail!("unknown tool: {name}"),
        }
    }

    fn ensure_tool_allowed(&self, name: &str) -> anyhow::Result<()> {
        if let Some(allowed) = self.allowed_tools.as_ref() {
            if !allowed.contains(name) {
                bail!("tool not allowed for this session: {name}");
            }
        }
        Ok(())
    }

    fn call_fs_tool(&self, name: &str, arguments_json: &str) -> anyhow::Result<String> {
        match name {
            "fs_read" => {
                let args: FsReadArgs = parse_args(arguments_json, "fs_read")?;
                self.fs_read(&args.path)
            }
            "file_read" => {
                let args: FileReadArgs = parse_args(arguments_json, "file_read")?;
                self.fs_read(&args.path)
            }
            "fs_write" => {
                let args: FsWriteArgs = parse_args(arguments_json, "fs_write")?;
                self.fs_write(&args.path, &args.content)
            }
            "file_write" => {
                let args: FileWriteArgs = parse_args(arguments_json, "file_write")?;
                self.fs_write(&args.path, &args.content)
            }
            "file_list" => {
                let args: FileListArgs = parse_args(arguments_json, "file_list")?;
                self.file_list(&args.path)
            }
            "apply_patch" => {
                let args: ApplyPatchArgs = parse_args(arguments_json, "apply_patch")?;
                self.apply_patch(&args.patch)
            }
            _ => unreachable!("unexpected fs tool: {name}"),
        }
    }

    async fn call_process_tool(&self, name: &str, arguments_json: &str) -> anyhow::Result<String> {
        match name {
            "shell" => {
                let args: ShellArgs = parse_args(arguments_json, "shell")?;
                self.shell(&args.command, args.timeout_ms).await
            }
            "shell_exec" => {
                let args: ShellExecArgs = parse_args(arguments_json, "shell_exec")?;
                let timeout_ms = args.timeout_seconds.map(|s| s.saturating_mul(1000));
                self.shell(&args.command, timeout_ms).await
            }
            "docker_exec" => {
                let args: DockerExecArgs = parse_args(arguments_json, "docker_exec")?;
                self.docker_exec(&args.command).await
            }
            "process_start" => {
                let args: ProcessStartArgs = parse_args(arguments_json, "process_start")?;
                self.process_start(&args.command, &args.args).await
            }
            "process_poll" => {
                let args: ProcessPollArgs = parse_args(arguments_json, "process_poll")?;
                self.process_poll(&args.process_id).await
            }
            "process_write" => {
                let args: ProcessWriteArgs = parse_args(arguments_json, "process_write")?;
                self.process_write(&args.process_id, &args.data).await
            }
            "process_kill" => {
                let args: ProcessKillArgs = parse_args(arguments_json, "process_kill")?;
                self.process_kill(&args.process_id).await
            }
            "process_list" => {
                let _args: serde_json::Value = parse_args(arguments_json, "process_list")?;
                self.process_list().await
            }
            _ => unreachable!("unexpected process tool: {name}"),
        }
    }

    async fn call_web_tool(&self, name: &str, arguments_json: &str) -> anyhow::Result<String> {
        match name {
            "web_fetch" => {
                let args: WebFetchArgs = parse_args(arguments_json, "web_fetch")?;
                self.web_fetch(
                    &args.url,
                    args.timeout_ms,
                    args.max_bytes,
                    args.allow_private,
                )
                .await
            }
            "pdf" | "pdf_extract" => {
                let args: PdfArgs = parse_args(arguments_json, "pdf")?;
                self.pdf_extract(
                    &args.path,
                    args.pages.as_deref(),
                    args.max_pages,
                    args.max_chars,
                )
                .await
            }
            "web_search" => {
                let args: WebSearchArgs = parse_args(arguments_json, "web_search")?;
                self.web_search(&args.query, args.max_results).await
            }
            "a2a_discover" => {
                let args: A2aDiscoverArgs = parse_args(arguments_json, "a2a_discover")?;
                self.a2a_discover(&args.url, args.allow_private).await
            }
            "a2a_send" => {
                let args: A2aSendArgs = parse_args(arguments_json, "a2a_send")?;
                let url = args
                    .agent_url
                    .as_deref()
                    .or(args.url.as_deref())
                    .context("missing agent_url (or url) for a2a_send")?;
                self.a2a_send(
                    url,
                    &args.message,
                    args.session_id.as_deref(),
                    args.allow_private,
                )
                .await
            }
            "location_get" => {
                let _args: serde_json::Value = parse_args(arguments_json, "location_get")?;
                self.location_get()
            }
            _ => unreachable!("unexpected web tool: {name}"),
        }
    }

    fn call_media_tool(&self, name: &str, arguments_json: &str) -> anyhow::Result<String> {
        match name {
            "image_analyze" => {
                let args: ImageAnalyzeArgs = parse_args(arguments_json, "image_analyze")?;
                self.image_analyze(&args.path)
            }
            "media_describe" => {
                let args: MediaDescribeArgs = parse_args(arguments_json, "media_describe")?;
                self.media_describe(&args.path)
            }
            "media_transcribe" => {
                let args: MediaTranscribeArgs = parse_args(arguments_json, "media_transcribe")?;
                self.media_transcribe(&args.path)
            }
            "speech_to_text" => {
                let args: SpeechToTextArgs = parse_args(arguments_json, "speech_to_text")?;
                self.speech_to_text(&args.path)
            }
            "text_to_speech" => {
                let args: TextToSpeechArgs = parse_args(arguments_json, "text_to_speech")?;
                self.text_to_speech(&args.text, args.path.as_deref())
            }
            "image_generate" => {
                let args: ImageGenerateArgs = parse_args(arguments_json, "image_generate")?;
                self.image_generate(&args.prompt, &args.path)
            }
            "canvas_present" => {
                let args: CanvasPresentArgs = parse_args(arguments_json, "canvas_present")?;
                self.canvas_present(&args.html, args.title.as_deref())
            }
            _ => unreachable!("unexpected media tool: {name}"),
        }
    }

    async fn call_browser_tool(&self, name: &str, arguments_json: &str) -> anyhow::Result<String> {
        match name {
            "browser_navigate" => {
                let args: BrowserNavigateArgs = parse_args(arguments_json, "browser_navigate")?;
                self.browser_navigate(
                    &args.url,
                    args.timeout_ms,
                    args.allow_private,
                    args.headless,
                )
                .await
            }
            "browser_back" => {
                let _args: serde_json::Value = parse_args(arguments_json, "browser_back")?;
                self.browser_back().await
            }
            "browser_close" => {
                let _args: serde_json::Value = parse_args(arguments_json, "browser_close")?;
                self.browser_close().await
            }
            "browser_click" => {
                let args: BrowserClickArgs = parse_args(arguments_json, "browser_click")?;
                self.browser_click(&args.selector).await
            }
            "browser_type" => {
                let args: BrowserTypeArgs = parse_args(arguments_json, "browser_type")?;
                self.browser_type(&args.selector, &args.text).await
            }
            "browser_press_key" => {
                let args: BrowserPressKeyArgs = parse_args(arguments_json, "browser_press_key")?;
                self.browser_press_key(args.selector.as_deref(), &args.key)
                    .await
            }
            "browser_scroll" => {
                let args: BrowserScrollArgs = parse_args(arguments_json, "browser_scroll")?;
                self.browser_scroll(args.direction.as_deref(), args.amount)
                    .await
            }
            "browser_wait" => {
                let args: BrowserWaitArgs = parse_args(arguments_json, "browser_wait")?;
                self.browser_wait(&args.selector, args.timeout_ms).await
            }
            "browser_wait_for" => {
                let args: BrowserWaitForArgs = parse_args(arguments_json, "browser_wait_for")?;
                self.browser_wait_for(
                    args.selector.as_deref(),
                    args.text.as_deref(),
                    args.timeout_ms,
                )
                .await
            }
            "browser_read_page" => {
                let _args: serde_json::Value = parse_args(arguments_json, "browser_read_page")?;
                self.browser_read_page().await
            }
            "browser_run_js" => {
                let args: BrowserRunJsArgs = parse_args(arguments_json, "browser_run_js")?;
                self.browser_run_js(&args.expression).await
            }
            "browser_screenshot" => {
                let args: BrowserScreenshotArgs = parse_args(arguments_json, "browser_screenshot")?;
                self.browser_screenshot(args.path.as_deref()).await
            }
            _ => unreachable!("unexpected browser tool: {name}"),
        }
    }

    fn call_runtime_compat_tool(name: &str) -> anyhow::Result<String> {
        bail!("tool '{name}' is implemented in the runtime, not Toolset")
    }
}

fn parse_args<T: DeserializeOwned>(arguments_json: &str, tool_name: &str) -> anyhow::Result<T> {
    serde_json::from_str(arguments_json).with_context(|| format!("parse {tool_name} arguments"))
}
