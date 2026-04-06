use std::path::PathBuf;

use crate::skills;

pub(super) fn run_list(workspace: PathBuf, json: bool) -> anyhow::Result<()> {
    let list = skills::list_skills(&workspace)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_skills_list_json(&list)?)?
        );
    } else if list.is_empty() {
        println!("no skills discovered");
    } else {
        for item in list {
            println!(
                "{}  v{}  source={}  entry={}",
                item.name, item.version, item.source, item.entry_path
            );
        }
    }
    Ok(())
}

pub(super) fn run_show(name: String, workspace: PathBuf, json: bool) -> anyhow::Result<()> {
    let skill = skills::find_skill(&workspace, &name)?;
    let item = serde_json::json!({
        "name": skill.name,
        "version": skill.manifest.version.to_string(),
        "source": skills::source_name(skill.source),
        "root_dir": skill.root_dir,
        "manifest_path": skill.manifest_path,
        "entry": skill.manifest.entry,
        "permissions": skill.manifest.permissions,
        "dependencies": skill
            .manifest
            .dependencies
            .iter()
            .map(|dependency| serde_json::json!({
                "name": dependency.name,
                "version_req": dependency.version_req.to_string(),
            }))
            .collect::<Vec<_>>(),
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&item)?);
    } else {
        println!("name: {}", item["name"].as_str().unwrap_or("-"));
        println!("version: {}", item["version"].as_str().unwrap_or("-"));
        println!("source: {}", item["source"].as_str().unwrap_or("-"));
        println!("root_dir: {}", item["root_dir"].as_str().unwrap_or("-"));
        println!(
            "manifest_path: {}",
            item["manifest_path"].as_str().unwrap_or("-")
        );
        println!("entry: {}", item["entry"].as_str().unwrap_or("-"));
        let perms: Vec<String> = item["permissions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        println!(
            "permissions: {}",
            if perms.is_empty() {
                "(none)".to_string()
            } else {
                perms.join(", ")
            }
        );

        let deps = item["dependencies"].as_array().cloned().unwrap_or_default();
        if deps.is_empty() {
            println!("dependencies: (none)");
        } else {
            println!("dependencies:");
            for dep in deps {
                let name = dep.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                let version_req = dep
                    .get("version_req")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                println!("- {name} {version_req}");
            }
        }
    }
    Ok(())
}

fn build_skills_list_json(list: &[skills::SkillListItem]) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::to_value(list)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_skills_list_json_keeps_expected_shape() {
        let list = vec![skills::SkillListItem {
            name: "alpha".to_string(),
            version: "1.2.3".to_string(),
            source: "workspace".to_string(),
            root_dir: "/tmp/alpha".to_string(),
            entry_path: "/tmp/alpha/SKILL.md".to_string(),
            permissions: vec!["readonly".to_string()],
        }];

        let out = build_skills_list_json(&list).unwrap();
        assert_eq!(
            out,
            json!([{
                "name": "alpha",
                "version": "1.2.3",
                "source": "workspace",
                "root_dir": "/tmp/alpha",
                "entry_path": "/tmp/alpha/SKILL.md",
                "permissions": ["readonly"],
            }])
        );
    }
}
