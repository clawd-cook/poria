use std::collections::HashMap;

use crate::types::RepoConfig;

#[derive(Debug, thiserror::Error)]
#[error("Circular dependency detected: {}", cycle.join(" → "))]
pub struct CircularDependencyError {
    pub cycle: Vec<String>,
}

pub fn topological_sort(repos: &[RepoConfig]) -> Result<Vec<RepoConfig>, CircularDependencyError> {
    let name_to_repo: HashMap<&str, &RepoConfig> =
        repos.iter().map(|r| (r.name.as_str(), r)).collect();
    let mut in_degree: HashMap<&str, usize> = repos.iter().map(|r| (r.name.as_str(), 0)).collect();
    let mut adjacency: HashMap<&str, Vec<&str>> = repos
        .iter()
        .map(|r| (r.name.as_str(), Vec::new()))
        .collect();

    for repo in repos {
        if let Some(deps) = &repo.depends_on {
            for dep in deps {
                if !name_to_repo.contains_key(dep.as_str()) {
                    continue;
                }
                adjacency
                    .get_mut(dep.as_str())
                    .unwrap()
                    .push(repo.name.as_str());
                *in_degree.get_mut(repo.name.as_str()).unwrap() += 1;
            }
        }
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();
    queue.sort();

    let mut sorted: Vec<RepoConfig> = Vec::new();
    while let Some(name) = queue.first().copied() {
        queue.remove(0);
        sorted.push(name_to_repo[name].clone());
        for &neighbor in adjacency.get(name).unwrap_or(&Vec::new()) {
            let deg = in_degree.get_mut(neighbor).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push(neighbor);
                queue.sort();
            }
        }
    }

    if sorted.len() != repos.len() {
        let remaining: Vec<String> = repos
            .iter()
            .filter(|r| !sorted.iter().any(|s| s.name == r.name))
            .map(|r| r.name.clone())
            .collect();
        return Err(CircularDependencyError { cycle: remaining });
    }

    Ok(sorted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str, deps: Option<Vec<&str>>) -> RepoConfig {
        RepoConfig {
            name: name.into(),
            git_url: String::new(),
            branch: "main".into(),
            base_branch: "main".into(),
            gitlab_project_path: String::new(),
            depends_on: deps.map(|d| d.into_iter().map(String::from).collect()),
            build_cmd: None,
        }
    }

    #[test]
    fn test_no_deps() {
        let repos = vec![repo("a", None), repo("b", None)];
        let sorted = topological_sort(&repos).unwrap();
        assert_eq!(sorted.len(), 2);
    }

    #[test]
    fn test_linear_deps() {
        let repos = vec![
            repo("c", Some(vec!["b"])),
            repo("b", Some(vec!["a"])),
            repo("a", None),
        ];
        let sorted = topological_sort(&repos).unwrap();
        let names: Vec<&str> = sorted.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_circular_dependency() {
        let repos = vec![repo("a", Some(vec!["b"])), repo("b", Some(vec!["a"]))];
        let err = topological_sort(&repos).unwrap_err();
        assert!(!err.cycle.is_empty());
    }

    #[test]
    fn test_diamond_deps() {
        let repos = vec![
            repo("d", Some(vec!["b", "c"])),
            repo("b", Some(vec!["a"])),
            repo("c", Some(vec!["a"])),
            repo("a", None),
        ];
        let sorted = topological_sort(&repos).unwrap();
        let names: Vec<&str> = sorted.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names[0], "a");
        assert_eq!(names[3], "d");
    }

    #[test]
    fn test_unknown_dep_ignored() {
        let repos = vec![repo("a", Some(vec!["nonexistent"]))];
        let sorted = topological_sort(&repos).unwrap();
        assert_eq!(sorted.len(), 1);
    }

    #[test]
    fn test_empty() {
        let sorted = topological_sort(&[]).unwrap();
        assert!(sorted.is_empty());
    }
}
