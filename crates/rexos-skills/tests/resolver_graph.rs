use rexos_skills::resolver::{SkillDependencyConstraint, SkillNode, resolve_load_order};

fn node(name: &str, version: &str, deps: Vec<(&str, &str)>) -> SkillNode {
    SkillNode {
        name: name.to_string(),
        version: version.parse().unwrap(),
        dependencies: deps
            .into_iter()
            .map(|(dep, req)| SkillDependencyConstraint {
                name: dep.to_string(),
                version_req: req.parse().unwrap(),
            })
            .collect(),
    }
}

#[test]
fn rejects_dependency_cycle() {
    let graph = vec![
        node("a", "1.0.0", vec![("b", "*")]),
        node("b", "1.0.0", vec![("a", "*")]),
    ];

    let err = resolve_load_order(graph).unwrap_err();
    assert!(err.to_string().contains("cycle"));
}

#[test]
fn rejects_unsatisfied_dependency_version() {
    let graph = vec![
        node("a", "1.0.0", vec![("b", "^2")]),
        node("b", "1.4.0", vec![]),
    ];

    let err = resolve_load_order(graph).unwrap_err();
    assert!(err.to_string().contains("version"));
}

#[test]
fn resolves_topological_order() {
    let graph = vec![
        node("app", "1.0.0", vec![("core", "^1")]),
        node("core", "1.2.0", vec![]),
    ];

    let order = resolve_load_order(graph).unwrap();
    assert_eq!(order, vec!["core".to_string(), "app".to_string()]);
}
