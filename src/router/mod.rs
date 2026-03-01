use crate::config::RouterConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Planning,
    Coding,
    Summary,
}

#[derive(Debug, Clone)]
pub struct ModelRouter {
    cfg: RouterConfig,
}

impl ModelRouter {
    pub fn new(cfg: RouterConfig) -> Self {
        Self { cfg }
    }

    pub fn model_for(&self, kind: TaskKind) -> &str {
        match kind {
            TaskKind::Planning => self.cfg.planning_model.as_str(),
            TaskKind::Coding => self.cfg.coding_model.as_str(),
            TaskKind::Summary => self.cfg.summary_model.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RouterConfig;

    #[test]
    fn router_selects_model_by_kind() {
        let cfg = RouterConfig {
            planning_model: "plan".to_string(),
            coding_model: "code".to_string(),
            summary_model: "sum".to_string(),
        };
        let router = ModelRouter::new(cfg);
        assert_eq!(router.model_for(TaskKind::Planning), "plan");
        assert_eq!(router.model_for(TaskKind::Coding), "code");
        assert_eq!(router.model_for(TaskKind::Summary), "sum");
    }
}

