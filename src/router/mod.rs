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

    pub fn provider_for(&self, kind: TaskKind) -> &str {
        match kind {
            TaskKind::Planning => self.cfg.planning.provider.as_str(),
            TaskKind::Coding => self.cfg.coding.provider.as_str(),
            TaskKind::Summary => self.cfg.summary.provider.as_str(),
        }
    }

    pub fn model_for(&self, kind: TaskKind) -> &str {
        match kind {
            TaskKind::Planning => self.cfg.planning.model.as_str(),
            TaskKind::Coding => self.cfg.coding.model.as_str(),
            TaskKind::Summary => self.cfg.summary.model.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RouteConfig, RouterConfig};

    #[test]
    fn router_selects_provider_and_model_by_kind() {
        let cfg = RouterConfig {
            planning: RouteConfig {
                provider: "p1".to_string(),
                model: "plan".to_string(),
            },
            coding: RouteConfig {
                provider: "p2".to_string(),
                model: "code".to_string(),
            },
            summary: RouteConfig {
                provider: "p3".to_string(),
                model: "sum".to_string(),
            },
        };
        let router = ModelRouter::new(cfg);
        assert_eq!(router.provider_for(TaskKind::Planning), "p1");
        assert_eq!(router.model_for(TaskKind::Planning), "plan");
        assert_eq!(router.provider_for(TaskKind::Coding), "p2");
        assert_eq!(router.model_for(TaskKind::Coding), "code");
        assert_eq!(router.provider_for(TaskKind::Summary), "p3");
        assert_eq!(router.model_for(TaskKind::Summary), "sum");
    }
}
