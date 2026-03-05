use std::collections::{BTreeSet, HashMap};

use anyhow::bail;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillNode {
    pub name: String,
    pub version: semver::Version,
    pub dependencies: Vec<SkillDependencyConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDependencyConstraint {
    pub name: String,
    pub version_req: semver::VersionReq,
}

pub fn resolve_load_order(nodes: Vec<SkillNode>) -> anyhow::Result<Vec<String>> {
    if nodes.is_empty() {
        return Ok(Vec::new());
    }

    let mut by_name: HashMap<String, SkillNode> = HashMap::new();
    for node in nodes {
        let name = node.name.trim().to_string();
        if name.is_empty() {
            bail!("skill name cannot be empty");
        }
        if by_name.insert(name.clone(), node).is_some() {
            bail!("duplicate skill definition: {name}");
        }
    }

    let mut indegree: HashMap<String, usize> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();

    for (name, node) in &by_name {
        indegree.entry(name.clone()).or_insert(0);

        for dep in &node.dependencies {
            let dep_name = dep.name.trim();
            if dep_name.is_empty() {
                bail!("dependency name cannot be empty (skill={name})");
            }

            let Some(dep_node) = by_name.get(dep_name) else {
                bail!("missing dependency: skill={name}, dependency={dep_name}");
            };

            if !dep.version_req.matches(&dep_node.version) {
                bail!(
                    "dependency version mismatch: skill={name}, dependency={dep_name}, requires={}, got={}",
                    dep.version_req,
                    dep_node.version
                );
            }

            outgoing
                .entry(dep_name.to_string())
                .or_default()
                .push(name.clone());
            *indegree.entry(name.clone()).or_insert(0) += 1;
        }
    }

    let mut ready = BTreeSet::new();
    for (name, degree) in &indegree {
        if *degree == 0 {
            ready.insert(name.clone());
        }
    }

    let mut order = Vec::with_capacity(by_name.len());
    while let Some(name) = ready.pop_first() {
        order.push(name.clone());

        if let Some(next_skills) = outgoing.get(&name) {
            for next in next_skills {
                if let Some(entry) = indegree.get_mut(next) {
                    *entry -= 1;
                    if *entry == 0 {
                        ready.insert(next.clone());
                    }
                }
            }
        }
    }

    if order.len() != by_name.len() {
        bail!("dependency cycle detected in skills graph");
    }

    Ok(order)
}
