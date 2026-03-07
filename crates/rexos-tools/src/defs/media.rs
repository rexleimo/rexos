use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ImageAnalyzeArgs {
    pub(crate) path: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct MediaDescribeArgs {
    pub(crate) path: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct MediaTranscribeArgs {
    pub(crate) path: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct SpeechToTextArgs {
    pub(crate) path: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TextToSpeechArgs {
    pub(crate) text: String,
    #[serde(default)]
    pub(crate) path: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ImageGenerateArgs {
    pub(crate) prompt: String,
    pub(crate) path: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct CanvasPresentArgs {
    pub(crate) html: String,
    #[serde(default)]
    pub(crate) title: Option<String>,
}

pub(crate) fn core_tool_defs() -> Vec<ToolDefinition> {
    Vec::new()
}

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "image_analyze".to_string(),
            description: "Analyze an image file in the workspace (basic metadata).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative image path." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "media_describe".to_string(),
            description: "Describe a media file in the workspace (best-effort metadata)."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative media path." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "media_transcribe".to_string(),
            description: "Transcribe media into text (currently supports text transcript files)."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative transcript path (.txt/.md/.srt/.vtt)." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "image_generate".to_string(),
            description: "Generate an image asset from a prompt (currently outputs SVG).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "prompt": { "type": "string", "description": "Image generation prompt." },
                    "path": { "type": "string", "description": "Workspace-relative output path (use .svg)." }
                },
                "required": ["prompt", "path"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "text_to_speech".to_string(),
            description: "Convert text to speech audio (MVP: writes a short .wav).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to convert to speech." },
                    "path": { "type": "string", "description": "Workspace-relative output path (use .wav). Optional." },
                    "voice": { "type": "string", "description": "Optional voice name (ignored in MVP)." },
                    "format": { "type": "string", "description": "Optional format (ignored in MVP; only .wav is supported)." }
                },
                "required": ["text"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "speech_to_text".to_string(),
            description: "Transcribe speech/audio into text (MVP: supports transcript files).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative transcript path (.txt/.md/.srt/.vtt)." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "canvas_present".to_string(),
            description: "Present sanitized HTML as a canvas artifact (saved to workspace output/).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "html": { "type": "string", "description": "HTML content to present (scripts/event handlers are forbidden)." },
                    "title": { "type": "string", "description": "Optional canvas title." }
                },
                "required": ["html"],
                "additionalProperties": false
            }),
        },
    });

    defs
}
