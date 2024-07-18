use crate::configuration::Settings;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

fn get_recipe_list(search: Option<String>) -> Vec<String> {
    let mut items = Vec::new();

    if cfg!(debug_assertions) {
        let curr_dir = std::env::current_dir().expect("failed to read current directory");
        let recipes_dir = curr_dir.join("recipes");
        let search = search.unwrap_or("".to_string());
        let matcher = SkimMatcherV2::default();
        if recipes_dir.is_dir() {
            let data = std::fs::read_dir(recipes_dir).expect("Failed to find recipes directory.");
            for entry in data {
                let entry = entry.expect("Failed to read entry.");
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let file_name = path
                    .file_name()
                    .map(|v| v.to_string_lossy().to_string())
                    .unwrap_or("".to_string());
                if search.len() > 0 {
                    let name_score = matcher.fuzzy_match(&file_name, &search).unwrap_or(0);
                    if name_score > 0 {
                        items.push(file_name);
                    }
                } else {
                    items.push(file_name);
                }
            }
        }
    } else {
        // Implement the list for production fetch from github.
        todo!()
    }

    items
}

pub async fn recipe(config: &Settings, input: Option<String>) {
    let dirs = get_recipe_list(input);
    for dir in dirs {
        println!("{dir}");
    }
}
