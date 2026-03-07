mod browser;
mod compat;
mod fs;
mod media;
mod process;
mod web;

use rexos_llm::openai_compat::ToolDefinition;

pub(crate) use browser::{
    ensure_browser_url_allowed, resolve_host_ips, BrowserClickArgs, BrowserNavigateArgs,
    BrowserPressKeyArgs, BrowserRunJsArgs, BrowserScreenshotArgs, BrowserScrollArgs,
    BrowserTypeArgs, BrowserWaitArgs, BrowserWaitForArgs,
};
pub(crate) use fs::{
    ApplyPatchArgs, FileListArgs, FileReadArgs, FileWriteArgs, FsReadArgs, FsWriteArgs,
};
pub(crate) use media::{
    CanvasPresentArgs, ImageAnalyzeArgs, ImageGenerateArgs, MediaDescribeArgs, MediaTranscribeArgs,
    SpeechToTextArgs, TextToSpeechArgs,
};
pub(crate) use process::{
    DockerExecArgs, ProcessKillArgs, ProcessPollArgs, ProcessStartArgs, ProcessWriteArgs,
    ShellArgs, ShellExecArgs,
};
pub(crate) use web::{A2aDiscoverArgs, A2aSendArgs, PdfArgs, WebFetchArgs, WebSearchArgs};

pub(crate) fn core_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();
    defs.extend(fs::core_tool_defs());
    defs.extend(process::core_tool_defs());
    defs.extend(web::core_tool_defs());
    defs.extend(media::core_tool_defs());
    defs.extend(browser::core_tool_defs());
    defs
}

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();
    defs.extend(fs::compat_tool_defs());
    defs.extend(process::compat_tool_defs());
    defs.extend(web::compat_tool_defs());
    defs.extend(media::compat_tool_defs());
    defs.extend(browser::compat_tool_defs());
    defs.extend(compat::compat_tool_defs());
    defs
}

#[cfg(test)]
mod tests {
    use super::{compat_tool_defs, core_tool_defs};

    #[test]
    fn core_tool_defs_include_multiple_domains() {
        let defs = core_tool_defs();
        let names: Vec<&str> = defs.iter().map(|d| d.function.name.as_str()).collect();
        assert!(names.contains(&"fs_read"), "{names:?}");
        assert!(names.contains(&"web_fetch"), "{names:?}");
        assert!(names.contains(&"browser_navigate"), "{names:?}");
    }

    #[test]
    fn compat_tool_defs_include_aliases() {
        let defs = compat_tool_defs();
        let names: Vec<&str> = defs.iter().map(|d| d.function.name.as_str()).collect();
        assert!(names.contains(&"file_read"), "{names:?}");
        assert!(names.contains(&"file_write"), "{names:?}");
        assert!(names.contains(&"workflow_run"), "{names:?}");
    }
}
